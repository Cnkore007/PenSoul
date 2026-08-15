// batch.rs — 大纲 → 细纲 → 批量写作流水线（P5）
// ① 一键细纲化：LLM(Outliner) 将大纲弧展开为带标题的细纲（建议制，可编辑）
// ② 细纲导入：写入章节表 title/summary（缺失章节自动创建）
// ③ 批量写作：逐章调用写作管线（Writer 角色），串行保证一致性
// 检查点：前端按批控制（默认每 3 章一批，作者审阅后再继续）——AI 无权跳步

use axum::extract::{Form, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::agent::AgentRole;
use crate::commands::llm::{build_llm_request, llm_client, structured_output_tokens};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::narrative::OutlineArc;
use pensoul_domain::ontology::NovelOntology;
use pensoul_infra::llm::LlmMessage;

/// 单章细纲条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailItem {
    pub chapter_no: i64,
    pub title: String,
    /// 细纲全文：要点 / 关键事件 / 涉及角色·地点·伏笔 / 情绪曲线 / 期望字数
    pub summary: String,
}

#[derive(Deserialize)]
pub struct DetailGenerateParams {
    /// 可选：指定大纲弧 id；缺省 = 全部弧
    pub arc_id: Option<String>,
}

#[derive(Deserialize)]
pub struct DetailImportParams {
    /// DetailItem JSON 数组字符串
    pub detail_json: String,
}

#[derive(Deserialize)]
pub struct BatchParams {
    /// 逗号分隔的章节 id（按此顺序串行生成）
    pub chapter_ids: String,
    /// 逗号分隔的叙事技巧 id（可选）
    pub technique_ids: Option<String>,
}

/// 一键细纲化：LLM 将大纲弧展开为带标题的细纲（建议制，不落盘）
pub async fn detail_generate(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<DetailGenerateParams>,
) -> Result<String, ApiError> {
    let (base_dir, arcs, project_context) = {
        let state = state.read().await;
        let base_dir = state.base_dir.clone();
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        if ontology.outline_arcs.is_empty() {
            return Err(ApiError::bad_request("还没有大纲弧。请先在大纲中创建脉络。"));
        }
        let arcs: Vec<OutlineArc> = ontology
            .outline_arcs
            .iter()
            .filter(|a| params.arc_id.as_deref().is_none_or(|id| a.arc_id == id))
            .cloned()
            .collect();
        if arcs.is_empty() {
            return Err(ApiError::not_found("大纲弧不存在"));
        }
        // 细纲不是只“照着弧扩写”，而是要在全书上下文里保持设定、人物与文风一致。
        let project_context = detail_project_context(ontology);
        (base_dir, arcs, project_context)
    };

    let provider = crate::commands::agent::resolve(&base_dir, AgentRole::Outliner).map_err(|_| {
        ApiError::bad_request("细纲 Agent 未配置 LLM。请在「设定 → Agent 模型」绑定或配置默认 LLM。")
    })?;
    let client = llm_client(&provider);

    // 长篇小说关键修正：不要一次把几十上百章塞给模型。
    // 按「弧 × 至多 N 章」切块生成，避免 max_tokens 截断导致后半细纲变空/变糊。
    const MAX_CHAPTERS_PER_CALL: i64 = 12;
    let mut items: Vec<DetailItem> = Vec::new();
    let mut calls: usize = 0;
    for arc in &arcs {
        let mut start = arc.chapter_start.max(1);
        while start <= arc.chapter_end {
            let end = start
                .saturating_add(MAX_CHAPTERS_PER_CALL - 1)
                .min(arc.chapter_end);
            let user_content = serde_json::json!({
                "project_context": project_context,
                "arc": {
                    "title": arc.title,
                    "description": arc.description,
                    "chapter_start": arc.chapter_start,
                    "chapter_end": arc.chapter_end,
                },
                "required_range": { "chapter_start": start, "chapter_end": end },
            })
            .to_string();
            let request = build_llm_request(
                &provider,
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: user_content,
                }],
                detail_system_prompt(),
                true,
                structured_output_tokens(&provider, 8192, 16000),
            );
            let resp = client
                .complete(request)
                .await
                .map_err(|e| ApiError::internal(format!("细纲生成失败（第 {start}-{end} 章）: {e}")))?;
            let chunk = parse_detail_items(&resp.content, start, end).map_err(ApiError::internal)?;
            if chunk.is_empty() {
                return Err(ApiError::internal(format!(
                    "细纲结果为空（第 {start}-{end} 章，LLM 未返回有效章节）"
                )));
            }
            items.extend(chunk);
            calls += 1;
            start = end + 1;
        }
    }

    items.sort_by_key(|item| item.chapter_no);
    items.dedup_by(|a, b| a.chapter_no == b.chapter_no);

    let expected: Vec<i64> = arcs
        .iter()
        .flat_map(|a| a.chapter_start.max(1)..=a.chapter_end)
        .collect();
    let missing_chapters: Vec<i64> = expected
        .into_iter()
        .filter(|no| !items.iter().any(|item| item.chapter_no == *no))
        .collect();

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "items": items,
        "count": items.len(),
        "model": provider.model_id,
        "calls": calls,
        "missing_chapters": missing_chapters,
        "note": if missing_chapters.is_empty() {
            format!("已分 {calls} 批生成 {} 章细纲（建议制）。确认无误后点「导入笔耕」。", items.len())
        } else {
            format!(
                "已分 {calls} 批生成 {} 章细纲，但有 {} 章缺失（{}）。建议重试或手动补齐。",
                items.len(),
                missing_chapters.len(),
                missing_chapters
                    .iter()
                    .take(10)
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join("、")
            )
        },
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 细纲导入：按 chapter_no upsert 章节 title/summary（缺失自动创建）
pub async fn detail_import(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<DetailImportParams>,
) -> Result<String, ApiError> {
    let items: Vec<DetailItem> = serde_json::from_str(&params.detail_json)
        .map_err(|_| ApiError::bad_request("细纲 JSON 格式非法"))?;
    if items.is_empty() {
        return Err(ApiError::bad_request("细纲为空"));
    }
    // 章号上限防护：防止恶意/异常输入创建海量占位章节导致内存与磁盘 DoS。
    // 上限 = 当前最大章号 + 500（覆盖正常扩容余量）。
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let current_max = ontology
        .chapters
        .iter()
        .map(|c| c.chapter_no)
        .max()
        .unwrap_or(0);
    const CHAPTER_NO_LIMIT: i64 = 500;
    for item in &items {
        if item.chapter_no > current_max + CHAPTER_NO_LIMIT {
            return Err(ApiError::bad_request(format!(
                "章节号 {} 超出合理范围（当前最大 {}，上限 +{}），请检查细纲数据",
                item.chapter_no, current_max, CHAPTER_NO_LIMIT
            )));
        }
    }

    let mut created = 0usize;
    let mut updated = 0usize;
    let mut max_no = current_max;
    for item in items {
        if item.chapter_no < 1 {
            continue;
        }
        if let Some(chapter) = ontology
            .chapters
            .iter_mut()
            .find(|c| c.chapter_no == item.chapter_no)
        {
            chapter.title = item.title;
            chapter.summary = item.summary;
            updated += 1;
        } else {
            let mut chapter = pensoul_domain::chapter::Chapter::new(item.chapter_no, item.title);
            chapter.summary = item.summary;
            ontology.chapters.push(chapter);
            max_no = max_no.max(item.chapter_no);
            created += 1;
        }
    }
    // 章节号不连续时补齐占位（保持顺序语义）
    for no in 1..=max_no {
        if !ontology.chapters.iter().any(|c| c.chapter_no == no) {
            ontology
                .chapters
                .push(pensoul_domain::chapter::Chapter::new(no, format!("第{no}章")));
        }
    }
    ontology.chapters.sort_by_key(|c| c.chapter_no);

    // 细纲进度落盘：让大纲脉络知道已经展开到第几章，仪表盘/大纲页可据此显示缺口。
    for arc in &mut ontology.outline_arcs {
        let imported_until = ontology
            .chapters
            .iter()
            .filter(|c| {
                c.chapter_no >= arc.chapter_start
                    && c.chapter_no <= arc.chapter_end
                    && !c.summary.trim().is_empty()
            })
            .map(|c| c.chapter_no)
            .max();
        if let Some(until) = imported_until {
            arc.expanded_until = arc.expanded_until.max(until);
        }
    }

    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "created": created,
        "updated": updated,
        "note": format!("细纲已导入：新建 {created} 章，更新 {updated} 章。"),
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 批量写作：按指定章节顺序串行生成初稿（建议制，不落盘）
/// 检查点由前端控制：一次请求 = 一批（建议每 3 章），作者审阅后再继续
pub async fn batch_write(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<BatchParams>,
) -> Result<String, ApiError> {
    let chapter_ids: Vec<String> = params
        .chapter_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if chapter_ids.is_empty() {
        return Err(ApiError::bad_request("请指定要批量写作的章节"));
    }
    if chapter_ids.len() > 10 {
        return Err(ApiError::bad_request(
            "单批最多 10 章。请按每 3 章一批（检查点）逐批写作。",
        ));
    }
    let technique_ids: Vec<String> = params
        .technique_ids
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut results = Vec::new();
    for chapter_id in &chapter_ids {
        let (no, title, summary, word_count) = {
            let state = state.read().await;
            let ontology = state
                .ontology
                .as_ref()
                .ok_or(ApiError::bad_request("没有打开的项目"))?;
            match ontology
                .chapters_in_order()
                .into_iter()
                .find(|c| c.chapter_id.to_string() == *chapter_id)
            {
                Some(c) => (c.chapter_no, c.title.clone(), c.summary.clone(), c.word_count),
                None => return Err(ApiError::not_found("章节不存在")),
            }
        };

        // 工业化流程门控：批量写作只处理“已有细纲且为空”的章节。
        // 没有细纲的章节必须先走「大纲 → 一键细纲化 → 导入笔耕」；已有正文绝不覆盖。
        if word_count > 0 {
            return Err(ApiError::bad_request(format!(
                "第 {no} 章《{title}》已有正文，批量写作不会覆盖。请在笔耕单章处理。"
            )));
        }
        if summary.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "第 {no} 章《{title}》缺少细纲摘要。请先在大纲页完成细纲化并「导入笔耕」。"
            )));
        }

        let gen = crate::commands::writing::generate_chapter(
            &state,
            chapter_id,
            "draft",
            "",
            &technique_ids,
        )
        .await
        .map_err(|_| ApiError::bad_request(format!("第 {no} 章《{title}》生成失败（请检查 LLM 配置与日志）")))?;

        results.push(serde_json::json!({
            "chapter_id": chapter_id,
            "chapter_no": no,
            "title": title,
            "content": gen["content"].as_str().unwrap_or(""),
            "model": gen["model"].as_str().unwrap_or(""),
            "anti_slop_warnings": gen["anti_slop_warnings"].clone(),
        }));
    }

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "results": results,
        "batch_size": results.len(),
        "note": "草稿为建议制，未写入正典。逐章确认后保存。",
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 细纲系统提示词（Outliner 角色）
fn detail_system_prompt() -> String {
    "你是 PenSoul 的细纲编辑：把大纲弧展开为「带标题的章节细纲」，只输出 JSON，不要任何解释。\n\
     输出格式：\n\
     {\"chapters\": [{\"chapter_no\": 1, \"title\": \"章节标题\", \"summary\": \"细纲全文\"}]}\n\
     每章 summary 需包含（用换行分行）：\n\
     - 段落要点（3-6 条，按叙事顺序）\n\
     - 关键事件\n\
     - 涉及角色 / 地点 / 伏笔\n\
     - 情绪曲线（如：平静→紧张→悬念收尾）\n\
     - 期望字数（默认 2000 字）\n\
     要求：\n\
     1. 只输出 required_range 指定的章节范围（chapter_start ~ chapter_end，含端点），不得多写、少写或自由扩写；\n\
     2. 必须融合 project_context 中的核心概念、世界观规则、人物档案、伏笔与风格笔记，不得自相矛盾；\n\
     3. 标题要有钩子感（网文惯例），不重复；\n\
     4. summary 要可执行（能直接指导 AI 写作），不要空泛；\n\
     5. 严格遵循弧的剧情走向，不得另起炉灶。"
        .to_string()
}

/// 组装细纲生成的「全书上下文」：不是只给大纲弧，而是把核心概念、世界观、
/// 人物档案、伏笔与风格笔记一起交给细纲编辑，避免细纲与已有设定脱节。
fn detail_project_context(ontology: &NovelOntology) -> serde_json::Value {
    let characters: Vec<_> = ontology
        .characters
        .characters
        .iter()
        .take(30)
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "occupation": c.properties.occupation,
                "realm": c.properties.realm,
                "wants": c.properties.wants,
                "fears": c.properties.fears,
                "secret": c.properties.secret,
                "backstory": truncate_detail_text(c.properties.backstory.as_deref().unwrap_or(""), 120),
            })
        })
        .collect();
    let locations: Vec<_> = ontology
        .world
        .locations
        .iter()
        .take(30)
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "category": s.category,
                "description": truncate_detail_text(&s.description, 160),
            })
        })
        .collect();
    let organizations: Vec<_> = ontology
        .world
        .organizations
        .iter()
        .take(20)
        .map(|o| {
            serde_json::json!({
                "name": o.name,
                "category": o.category,
                "description": truncate_detail_text(&o.description, 120),
            })
        })
        .collect();
    let foreshadows: Vec<_> = ontology
        .active_foreshadows()
        .into_iter()
        .take(20)
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "description": truncate_detail_text(&f.description, 120),
                "expected_payoff": f.expected_payoff,
            })
        })
        .collect();

    serde_json::json!({
        "high_concept": ontology.core_concept.high_concept,
        "premise": ontology.core_concept.premise,
        "protagonist_hint": ontology.core_concept.protagonist_hint,
        "tone": ontology.core_concept.tone,
        "central_conflict": ontology.core_concept.central_conflict,
        "genre": ontology.settings.genre,
        "style_notes": ontology.aesthetic.style_notes,
        "pacing_notes": ontology.aesthetic.pacing_notes,
        "world_rules": ontology.world.rules.iter().take(30).collect::<Vec<_>>(),
        "characters": characters,
        "locations": locations,
        "organizations": organizations,
        "active_foreshadows": foreshadows,
    })
}

fn truncate_detail_text(input: &str, max_chars: usize) -> String {
    let mut output: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        output.push_str("…（已截断）");
    }
    output
}

/// 解析单批细纲输出；只接受本批要求的章号区间，乱入/超界的章节显式丢弃，
/// 避免模型自由发挥打乱工业化流水线。
fn parse_detail_items(
    raw: &str,
    expected_start: i64,
    expected_end: i64,
) -> Result<Vec<DetailItem>, String> {
    let parsed: serde_json::Value = pensoul_infra::llm::parse_llm_json(raw)?;
    let array = parsed
        .get("chapters")
        .and_then(|v| v.as_array())
        .or_else(|| parsed.as_array())
        .ok_or_else(|| {
            format!(
                "LLM 输出缺少 chapters 数组；原始内容前 300 字: {}",
                raw.trim().chars().take(300).collect::<String>()
            )
        })?;

    let mut items = Vec::new();
    for entry in array {
        let Some(chapter_no) = entry
            .get("chapter_no")
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())))
        else {
            continue;
        };
        if chapter_no < expected_start || chapter_no > expected_end {
            continue;
        }
        let Some(title) = entry.get("title").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(summary) = entry.get("summary").and_then(|v| v.as_str()) else {
            continue;
        };
        if title.trim().is_empty() || summary.trim().is_empty() {
            continue;
        }
        items.push(DetailItem {
            chapter_no,
            title: title.trim().to_string(),
            summary: summary.trim().to_string(),
        });
    }

    if items.is_empty() {
        return Err(format!(
            "LLM 未返回第 {expected_start}-{expected_end} 章的有效细纲"
        ));
    }
    items.dedup_by(|a, b| a.chapter_no == b.chapter_no);
    Ok(items)
}

// ---- 单元测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_prompt_guides_structure() {
        let prompt = detail_system_prompt();
        assert!(prompt.contains("chapter_no"));
        assert!(prompt.contains("情绪曲线"));
        assert!(prompt.contains("期望字数"));
    }

    #[test]
    fn parse_detail_items_keeps_only_requested_range() {
        let raw = r#"{"chapters":[
            {"chapter_no": 5, "title": "第五章", "summary": "要点"},
            {"chapter_no": 14, "title": "越界章", "summary": "应丢弃"}
        ]}"#;
        let items = parse_detail_items(raw, 4, 6).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].chapter_no, 5);
    }

    #[test]
    fn parse_detail_items_rejects_empty_chunk() {
        let raw = r#"{"chapters":[]}"#;
        let err = parse_detail_items(raw, 1, 3).unwrap_err();
        assert!(err.contains("未返回第 1-3 章"));
    }

    #[test]
    fn import_params_roundtrip() {
        let items = vec![DetailItem {
            chapter_no: 1,
            title: "开局".into(),
            summary: "要点1
要点2".into(),
        }];
        let json = serde_json::to_string(&items).unwrap();
        let parsed: Vec<DetailItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "开局");
        assert_eq!(parsed[0].summary, "要点1
要点2");
    }
}
