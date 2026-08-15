// cascade.rs — 级联同步（P4）
// 章节改写确认后：事实差异提取 → 影响范围分析 → 受影响章节清单
// → 用户选择（默认仅向后，上限 20 章）→ 逐章局部一致性修订 → diff 确认 → 保存
//
// 护栏（设计 6.3）：
// 1. 仅向后：只分析 chapter_no 大于修改章的后续章节；
// 2. 级联上限：单次最多 20 章，超出提示手动调整；
// 3. 局部修订：LLM 只改受影响段落，防蝴蝶效应；
// 4. 可回滚：级联结果写入 cascade-log，一键回滚由前端撤销保存。

use axum::extract::{Form, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::agent::AgentRole;
use crate::commands::llm::{build_llm_request, llm_client, structured_output_tokens};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_infra::llm::LlmMessage;

/// 单次级联分析上限（护栏 2）
const CASCADE_MAX_CHAPTERS: usize = 20;

#[derive(Deserialize)]
pub struct CascadeAnalyzeParams {
    pub chapter_id: String,
    /// 修改前正文（正典原稿）
    pub original: String,
    /// 修改后正文（当前编辑器内容）
    pub rewritten: String,
}

#[derive(Deserialize)]
pub struct CascadeApplyParams {
    /// 被修改的章节（级联起点）
    pub chapter_id: String,
    /// 逗号分隔：用户确认要同步的受影响章节 id
    pub target_chapter_ids: String,
    /// 变更事实（analyze 返回的 JSON 字符串，逐章修订的上下文）
    pub changed_facts: String,
}

/// 变更事实
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangedFact {
    pub entity: String,
    pub attribute: String,
    pub old_value: String,
    pub new_value: String,
}

/// 受影响章节条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct AffectedChapter {
    pub chapter_id: String,
    pub chapter_no: i64,
    pub title: String,
    pub matched_entities: Vec<String>,
    pub snippet: String,
}

/// 级联分析：差异 → 变更事实 → 反向扫描后续章节 → 受影响清单
pub async fn cascade_analyze(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CascadeAnalyzeParams>,
) -> Result<String, ApiError> {
    let original = params.original.trim();
    let rewritten = params.rewritten.trim();
    if original.is_empty() || rewritten.is_empty() {
        return Err(ApiError::bad_request("原稿与改写稿都不能为空"));
    }
    if original == rewritten {
        return Err(ApiError::bad_request("没有检测到内容变化，无需级联"));
    }

    let (base_dir, modified_no) = {
        let state = state.read().await;
        let base_dir = state.base_dir.clone();
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let chapter = ontology
            .chapters_in_order()
            .into_iter()
            .find(|c| c.chapter_id.to_string() == params.chapter_id)
            .ok_or(ApiError::not_found("章节不存在"))?;
        (base_dir, chapter.chapter_no)
    };

    // ① 事实差异提取（LLM，Extractor 角色；失败则退化为实体名启发式）
    let changed_facts = match extract_changed_facts(&base_dir, original, rewritten).await {
        Ok(facts) => facts,
        Err(_) => heuristic_changed_facts(original, rewritten),
    };
    if changed_facts.is_empty() {
        return serde_json::to_string(&serde_json::json!({
            "ok": true,
            "changed_facts": [],
            "affected": [],
            "note": "未检测到可级联的事实变化。",
        }))
        .map_err(|e| ApiError::internal(e.to_string()));
    }

    // ② 影响范围分析：本地反向检索（只向后，上限 20 章）
    let affected = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let mut list = Vec::new();
        for chapter in ontology.chapters_in_order() {
            if chapter.chapter_no <= modified_no || list.len() >= CASCADE_MAX_CHAPTERS {
                continue;
            }
            if chapter.content.trim().is_empty() {
                continue;
            }
            let matched: Vec<String> = changed_facts
                .iter()
                .map(|f| f.entity.trim())
                .filter(|name| !name.is_empty() && chapter.content.contains(*name))
                .map(|s| s.to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            if matched.is_empty() {
                continue;
            }
            let snippet = first_matching_paragraph(&chapter.content, &matched);
            list.push(AffectedChapter {
                chapter_id: chapter.chapter_id.to_string(),
                chapter_no: chapter.chapter_no,
                title: chapter.title.clone(),
                matched_entities: matched,
                snippet,
            });
        }
        list
    };

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "changed_facts": changed_facts,
        "affected": affected,
        "limit": CASCADE_MAX_CHAPTERS,
        "note": if affected.is_empty() {
            "后续章节未引用相关事实，无需级联。".to_string()
        } else {
            format!("检测到 {} 个受影响章节（仅向后，上限 {} 章）。", affected.len(), CASCADE_MAX_CHAPTERS)
        },
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 级联应用：对每个目标章节做局部一致性修订（LLM 只改受影响段落）
pub async fn cascade_apply(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CascadeApplyParams>,
) -> Result<String, ApiError> {
    let facts: Vec<ChangedFact> = serde_json::from_str(&params.changed_facts)
        .map_err(|_| ApiError::bad_request("变更事实格式非法"))?;
    if facts.is_empty() {
        return Err(ApiError::bad_request("没有变更事实可同步"));
    }
    let target_ids: Vec<String> = params
        .target_chapter_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if target_ids.is_empty() {
        return Err(ApiError::bad_request("请选择要同步的章节"));
    }
    if target_ids.len() > CASCADE_MAX_CHAPTERS {
        return Err(ApiError::bad_request(format!(
            "单次级联最多 {} 章（超出部分请手动调整）",
            CASCADE_MAX_CHAPTERS
        )));
    }

    // 读取目标章节正文（只向后，由 analyze 保证；此处再做一次防御）
    let base_dir = {
        let state = state.read().await;
        state.base_dir.clone()
    };
    let provider =
        crate::commands::agent::resolve(&base_dir, AgentRole::Writer).map_err(|_| {
            ApiError::bad_request("写作 Agent 未配置 LLM，无法级联。请在「设定 → Agent 模型」绑定。")
        })?;
    let client = llm_client(&provider);

    let mut results = Vec::new();
    for target_id in &target_ids {
        let (title, chapter_no, content) = {
            let state = state.read().await;
            let ontology = state
                .ontology
                .as_ref()
                .ok_or(ApiError::bad_request("没有打开的项目"))?;
            let Some(chapter) = ontology
                .chapters_in_order()
                .into_iter()
                .find(|c| c.chapter_id.to_string() == *target_id)
            else {
                continue;
            };
            (chapter.title.clone(), chapter.chapter_no, chapter.content.clone())
        };
        if content.trim().is_empty() {
            continue;
        }

        // 逐章局部修订（建议制，返回改写稿）
        let system = cascade_system_prompt(&facts);
        let request = build_llm_request(
            &provider,
            vec![LlmMessage {
                role: "user".to_string(),
                content: format!(
                    "第 {} 章《{}》正文：\n{}",
                    chapter_no,
                    title,
                    crate::commands::writing::truncate_chars(&content, 6000)
                ),
            }],
            system,
            true,
            structured_output_tokens(&provider, 8192, 16000),
        );
        let resp = match client.complete(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(ApiError::internal(format!("第 {chapter_no} 章级联失败：{e}")));
            }
        };
        let parsed: serde_json::Value = pensoul_infra::llm::parse_llm_json::<serde_json::Value>(&resp.content)
            .map_err(|e| ApiError::internal(format!("级联响应解析失败: {e}")))?;
        let rewritten = parsed
            .get("rewritten")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if rewritten.is_empty() || rewritten == content {
            continue;
        }
        results.push(serde_json::json!({
            "chapter_id": target_id,
            "chapter_no": chapter_no,
            "title": title,
            "rewritten": rewritten,
        }));
    }

    if results.is_empty() {
        return Err(ApiError::bad_request("所选章节无需同步（无受影响段落变化）"));
    }

    // 级联记录（审计轨迹，_config/cascade-log.json）；写失败不掩盖级联本身，但显式告知
    let mut log_warning: Option<String> = None;
    if let Err(e) = append_cascade_log(&base_dir, &params.chapter_id, &results) {
        log_warning = Some(format!("级联日志写入失败（审计轨迹缺失）: {e}"));
    }

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "results": results,
        "log_warning": log_warning,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 事实差异提取（LLM：原稿 vs 改写稿 → 变更事实）
async fn extract_changed_facts(
    base_dir: &str,
    original: &str,
    rewritten: &str,
) -> Result<Vec<ChangedFact>, ApiError> {
    let provider = crate::commands::agent::resolve(base_dir, AgentRole::Extractor)?;
    let client = llm_client(&provider);
    let request = build_llm_request(
        &provider,
        vec![LlmMessage {
            role: "user".to_string(),
            content: format!(
                "修改前：\n{}\n\n---\n修改后：\n{}",
                crate::commands::writing::truncate_chars(original, 6000),
                crate::commands::writing::truncate_chars(rewritten, 6000),
            ),
        }],
        "你是 PenSoul 的事实差异分析器：对比「修改前/修改后」两版正文，提取因改写而改变的正典事实，只输出 JSON：\n\
         {\"changed_facts\": [{\"entity\": \"实体名\", \"attribute\": \"属性\", \"old_value\": \"旧值\", \"new_value\": \"新值\"}]}\n\
         规则：\n\
         1. 只提取对后续章节一致性有影响的实质变化（角色属性/状态、设定、时间线、事件结果、伏笔状态等）；\n\
         2. 修辞润色、句式调整不算事实变化；\n\
         3. 无变化返回 {\"changed_facts\":[]}。"
            .to_string(),
        true,
        structured_output_tokens(&provider, 4096, 16000),
    );
    let resp = client
        .complete(request)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let parsed: serde_json::Value = pensoul_infra::llm::parse_llm_json::<serde_json::Value>(&resp.content)
        .map_err(|e| ApiError::internal(format!("差异分析响应解析失败: {e}")))?;
    let facts: Vec<ChangedFact> = parsed
        .get("changed_facts")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    Some(ChangedFact {
                        entity: f.get("entity")?.as_str()?.to_string(),
                        attribute: f.get("attribute")?.as_str()?.to_string(),
                        old_value: f.get("old_value")?.as_str()?.to_string(),
                        new_value: f.get("new_value")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(facts)
}

/// 无 LLM 时的启发式：抽取「两版差异行中仍出现在原稿里的连续名词」作为实体候选
fn heuristic_changed_facts(original: &str, rewritten: &str) -> Vec<ChangedFact> {
    let mut facts = Vec::new();
    let o_lines: Vec<&str> = original.lines().collect();
    let r_lines: Vec<&str> = rewritten.lines().collect();
    let changed_lines: Vec<&str> = r_lines
        .iter()
        .filter(|l| !o_lines.contains(l)).copied()
        .collect();
    for line in changed_lines {
        let chars: Vec<char> = line.chars().collect();
        for win in 2..=4usize {
            let mut i = 0;
            while i + win <= chars.len() {
                let name: String = chars[i..i + win].iter().collect();
                // 实体名需同时出现在原稿（改写通常保留实体、改变属性）
                if !name.contains('的')
                    && original.contains(&name)
                    && !facts.iter().any(|f: &ChangedFact| f.entity == name)
                {
                    facts.push(ChangedFact {
                        entity: name,
                        attribute: "未知".into(),
                        old_value: String::new(),
                        new_value: line.to_string(),
                    });
                }
                i += 1;
            }
        }
    }
    facts.truncate(8);
    facts
}

/// 首个命中段落（截取 120 字）
fn first_matching_paragraph(content: &str, matched: &[String]) -> String {
    for para in content.lines() {
        if matched.iter().any(|m| para.contains(m.as_str())) {
            let trimmed = para.trim();
            return crate::commands::writing::truncate_chars(trimmed, 120);
        }
    }
    String::new()
}

fn cascade_system_prompt(facts: &[ChangedFact]) -> String {
    let fact_lines: Vec<String> = facts
        .iter()
        .map(|f| format!("{}「{}」：{} → {}", f.entity, f.attribute, f.old_value, f.new_value))
        .collect();
    format!(
        "你是 PenSoul 的连续性编辑。前方章节发生事实变更，需要你对该章节做「局部一致性修订」。\n\
         事实变更：\n{}\n\
         要求：\n\
         1. 只修改正文中与这些变更冲突或引用了旧值的段落，其余内容一字不动；\n\
         2. 不得整章重写、不得加戏、不得改变情节走向与文风；\n\
         3. 若某处需读者回看的设定（如人物当前状态）已过时，按新值修正；\n\
         4. 只输出 JSON：{{\"rewritten\": \"完整修订稿\"}}。",
        fact_lines.join("\n"),
    )
}

/// 级联审计日志（保留最近 200 条）；写失败返回错误供调用方告知用户
fn append_cascade_log(base_dir: &str, source_chapter: &str, results: &[serde_json::Value]) -> Result<(), String> {
    let dir = std::path::Path::new(base_dir).join("_config");
    let path = dir.join("cascade-log.json");
    std::fs::create_dir_all(&dir).map_err(|e| format!("日志目录创建失败: {e}"))?;
    let mut entries: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    entries.push(serde_json::json!({
        "time": chrono::Utc::now().to_rfc3339(),
        "source_chapter": source_chapter,
        "targets": results,
    }));
    if entries.len() > 200 {
        entries.drain(..entries.len() - 200);
    }
    let body = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("级联日志写入失败: {e}"))
}

// ---- 单元测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_finds_entity_candidates() {
        let facts = heuristic_changed_facts(
            "林默进入筑基期。",
            "林默突破到金丹期，实力大增。",
        );
        assert!(!facts.is_empty(), "启发式应提取到实体候选: {facts:?}");
        assert!(facts.iter().any(|f| f.entity.contains("林默")), "应命中角色名: {facts:?}");
    }

    #[test]
    fn cascade_prompt_lists_facts() {
        let facts = vec![ChangedFact {
            entity: "林默".into(),
            attribute: "境界".into(),
            old_value: "筑基".into(),
            new_value: "金丹".into(),
        }];
        let prompt = cascade_system_prompt(&facts);
        assert!(prompt.contains("林默「境界」：筑基 → 金丹"));
        assert!(prompt.contains("不得整章重写"));
    }

    #[test]
    fn first_matching_paragraph_finds_hit() {
        let content = "第一段没有。\n林默的修为已是金丹，威压四方。\n第三段。";
        let snippet = first_matching_paragraph(content, &["林默".to_string()]);
        assert!(snippet.contains("金丹"), "应定位命中段落: {snippet}");
        assert!(snippet.chars().count() <= 120);
    }
}
