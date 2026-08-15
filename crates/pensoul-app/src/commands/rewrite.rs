// rewrite.rs — AI 审核改写管线（P2）
// 批注驱动：批注 + 正文 → LLM 改写 → 段级 diff → 前端逐段确认 → 保存
// 消痕改写：anti-slop 命中片段 → LLM 定位重写（从「提示」升级为「改写」，保持语义与文风）
// 建议制：只返回改写稿，写回走 save_chapter_content 集成层（修订历史 + 一致性评分 + 事件发布）
//
// 批注是章节派生数据：不落正典正文，随章节修订历史保留。

use axum::extract::{Form, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::agent::AgentRole;
use crate::commands::llm::{build_llm_request, llm_client, structured_output_tokens};
use crate::commands::writing::{anti_slop_scan, anti_slop_rules_text, hard_constraints_snapshot};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_infra::llm::LlmMessage;

// ---- 批注 CRUD（章节派生数据） ----

#[derive(Deserialize)]
pub struct AddAnnotationParams {
    pub chapter_id: String,
    /// 批注类型：批评 / 疑问 / 建议 / 指令
    pub kind: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct UpdateAnnotationParams {
    pub chapter_id: String,
    pub annotation_id: String,
    /// 新建 → 已指派 → 已解决
    pub status: String,
}

#[derive(Deserialize)]
pub struct DeleteAnnotationParams {
    pub chapter_id: String,
    pub annotation_id: String,
}

/// 添加批注（写入章节 annotations，随修订历史保留）
pub async fn add_annotation(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddAnnotationParams>,
) -> Result<String, ApiError> {
    let content = params.content.trim();
    if content.is_empty() {
        return Err(ApiError::bad_request("批注内容不能为空"));
    }
    let kind = params.kind.trim();
    if kind.is_empty() {
        return Err(ApiError::bad_request("批注类型不能为空"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let Some(chapter) = ontology
        .chapters
        .iter_mut()
        .find(|c| c.chapter_id.to_string() == params.chapter_id)
    else {
        return Err(ApiError::not_found("章节不存在"));
    };

    let annotation = pensoul_domain::entity::Annotation {
        annotation_id: uuid::Uuid::new_v4().to_string(),
        kind: kind.to_string(),
        content: content.to_string(),
        status: "新建".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let id = annotation.annotation_id.clone();
    chapter.annotations.push(annotation);
    state.save_project().map_err(ApiError::internal)?;
    Ok(id)
}

/// 批注状态机：合法状态与流转表（新建 → 已指派 → 已解决；已解决不可逆）
const ANNOTATION_STATES: [&str; 3] = ["新建", "已指派", "已解决"];

fn annotation_can_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true; // 幂等：重复设置相同状态允许
    }
    matches!(
        (from, to),
        ("新建", "已指派") | ("已指派", "已解决") | ("新建", "已解决")
    )
}

/// 更新批注状态（新建 → 已指派 → 已解决）
pub async fn update_annotation(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateAnnotationParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let Some(chapter) = ontology
        .chapters
        .iter_mut()
        .find(|c| c.chapter_id.to_string() == params.chapter_id)
    else {
        return Err(ApiError::not_found("章节不存在"));
    };
    let Some(annotation) = chapter
        .annotations
        .iter_mut()
        .find(|a| a.annotation_id == params.annotation_id)
    else {
        return Err(ApiError::not_found("批注不存在"));
    };
    // 状态白名单 + 合法流转校验
    if !ANNOTATION_STATES.contains(&params.status.as_str()) {
        return Err(ApiError::bad_request(format!(
            "批注状态必须是 {} 之一",
            ANNOTATION_STATES.join(" / ")
        )));
    }
    if !annotation_can_transition(&annotation.status, &params.status) {
        return Err(ApiError::bad_request(format!(
            "批注状态非法流转：{} → {}（合法路径：新建→已指派→已解决）",
            annotation.status, params.status
        )));
    }
    annotation.status = params.status;
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除批注
pub async fn delete_annotation(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteAnnotationParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let Some(chapter) = ontology
        .chapters
        .iter_mut()
        .find(|c| c.chapter_id.to_string() == params.chapter_id)
    else {
        return Err(ApiError::not_found("章节不存在"));
    };
    let before = chapter.annotations.len();
    chapter
        .annotations
        .retain(|a| a.annotation_id != params.annotation_id);
    if chapter.annotations.len() == before {
        return Err(ApiError::not_found("批注不存在"));
    }
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

// ---- AI 审核改写 ----

#[derive(Deserialize)]
pub struct RewriteParams {
    pub chapter_id: String,
    /// 要改写的正文（编辑器当前内容，可含未保存修改）
    pub content: String,
    /// 用户指令 / 未解决批注文本（可选）
    pub instructions: Option<String>,
    /// audit（审核改写，含默认质量项）| de-slop（仅消痕改写）
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteChange {
    pub what: String,
    pub why: String,
}

/// 段级 diff 条目（前端渲染原稿 | 改写稿）
#[derive(Debug, Clone, Serialize)]
pub struct DiffEntry {
    pub kind: &'static str, // equal | modified | added | removed
    pub text: String,
}

/// AI 审核改写（建议制：只返回改写稿 + 变更说明 + diff，不落盘）
pub async fn rewrite(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<RewriteParams>,
) -> Result<String, ApiError> {
    let mode = match params.mode.as_deref() {
        Some("de-slop") => "de-slop",
        Some("audit") | None => "audit",
        Some(other) => {
            return Err(ApiError::bad_request(format!(
                "mode 必须是 audit 或 de-slop，收到: {other}"
            )))
        }
    };
    let content = params.content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::bad_request("正文为空，无法改写"));
    }

    let (chapter_no, title, context_json, constraint_text, anti_slop, base_dir) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let chapter = ontology
            .chapters_in_order()
            .into_iter()
            .find(|c| c.chapter_id.to_string() == params.chapter_id)
            .cloned()
            .ok_or(ApiError::not_found("章节不存在"))?;
        let context_json = crate::commands::writing::writing_context(ontology, &chapter);
        let (constraint_text, _ids) = hard_constraints_snapshot(&state.constraints);
        (
            chapter.chapter_no,
            chapter.title.clone(),
            context_json,
            constraint_text,
            anti_slop_rules_text(),
            state.base_dir.clone(),
        )
    };

    let provider = crate::commands::agent::resolve(&base_dir, AgentRole::Writer).map_err(
        |_| ApiError::bad_request("写作 Agent 未配置 LLM，无法改写。请在「设定 → Agent 模型」绑定或配置默认 LLM。"),
    )?;
    let client = llm_client(&provider);

    let de_slop_hits = if mode == "de-slop" {
        anti_slop_scan(&content)
    } else {
        Vec::new()
    };

    let system = rewrite_system_prompt(
        mode,
        &title,
        chapter_no,
        &context_json,
        &constraint_text,
        &anti_slop,
        &de_slop_hits,
    );
    let user = rewrite_user_prompt(&content, params.instructions.as_deref(), &de_slop_hits);

    let request = build_llm_request(
        &provider,
        vec![LlmMessage {
            role: "user".to_string(),
            content: crate::commands::writing::truncate_chars(&user, 12000),
        }],
        system,
        true,
        structured_output_tokens(&provider, 8192, 16000),
    );

    let resp = client.complete(request).await.map_err(|e| {
        ApiError::internal(format!("改写调用失败：{e}"))
    })?;

    let parsed: serde_json::Value = pensoul_infra::llm::parse_llm_json::<serde_json::Value>(&resp.content)
        .map_err(|e| ApiError::internal(format!("改写响应解析失败: {e}")))?;
    let rewritten = parsed
        .get("rewritten")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if rewritten.is_empty() {
        return Err(ApiError::internal("改写结果为空"));
    }
    let changes: Vec<RewriteChange> = parsed
        .get("changes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    Some(RewriteChange {
                        what: c.get("what")?.as_str()?.to_string(),
                        why: c.get("why")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let diff = diff_sections(&content, &rewritten);

    serde_json::to_string(&serde_json::json!({
        "mode": mode,
        "chapter_id": params.chapter_id,
        "rewritten": rewritten,
        "changes": changes,
        "diff": diff,
        "de_slop_hits": de_slop_hits,
        "model": provider.model_id,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 段级 diff：按行做 LCS，标注 equal / modified / added / removed
fn diff_sections(before: &str, after: &str) -> Vec<DiffEntry> {
    let b: Vec<&str> = before.lines().map(|l| l.trim_end()).collect();
    let a: Vec<&str> = after.lines().map(|l| l.trim_end()).collect();
    if b.is_empty() && a.is_empty() {
        return Vec::new();
    }
    if b.is_empty() {
        return a.into_iter()
            .map(|t| DiffEntry { kind: "added", text: t.to_string() })
            .collect();
    }
    if a.is_empty() {
        return b.into_iter()
            .map(|t| DiffEntry { kind: "removed", text: t.to_string() })
            .collect();
    }

    // LCS DP（行粒度，正文行数有限，O(n*m) 可接受）
    let n = b.len();
    let m = a.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if b[i] == a[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut diff = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if b[i] == a[j] {
            diff.push(DiffEntry { kind: "equal", text: b[i].to_string() });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            diff.push(DiffEntry { kind: "removed", text: b[i].to_string() });
            i += 1;
        } else {
            diff.push(DiffEntry { kind: "added", text: a[j].to_string() });
            j += 1;
        }
    }
    while i < n {
        diff.push(DiffEntry { kind: "removed", text: b[i].to_string() });
        i += 1;
    }
    while j < m {
        diff.push(DiffEntry { kind: "added", text: a[j].to_string() });
        j += 1;
    }
    // 语义化：相邻 removed+added 成对 → modified
    let mut merged: Vec<DiffEntry> = Vec::with_capacity(diff.len());
    let mut k = 0;
    while k < diff.len() {
        if diff[k].kind == "removed"
            && k + 1 < diff.len()
            && diff[k + 1].kind == "added"
        {
            merged.push(DiffEntry {
                kind: "modified",
                text: format!("{} → {}", diff[k].text, diff[k + 1].text),
            });
            k += 2;
        } else {
            merged.push(DiffEntry { kind: diff[k].kind, text: diff[k].text.clone() });
            k += 1;
        }
    }
    merged
}

fn rewrite_system_prompt(
    mode: &str,
    title: &str,
    chapter_no: i64,
    context: &str,
    constraints: &str,
    anti_slop: &str,
    de_slop_hits: &[String],
) -> String {
    if mode == "de-slop" {
        format!(
            "你是 PenSoul 的润色编辑，任务是「消痕改写」：只重写用户正文中过度依赖 AI 高频表达的片段。\n\
             要求：\n\
             1. 只改写命中列表中的表达所在片段，其余内容一字不动；\n\
             2. 替换为更自然、具体的表达，保持原意、情绪与文风；\n\
             3. 不得整章重写、不得加戏、不得改变情节；\n\
             4. 只输出 JSON：{{\"rewritten\": \"完整改写稿\", \"changes\": [{{\"what\": \"改了什么\", \"why\": \"为什么\"}}]}}。\n\
             本片命中词：{hits}\n\
             反 AI 味规范：\n{anti_slop}\n\
             章节背景：第 {no} 章《{title}》\n{context}",
            hits = de_slop_hits.join("、"),
            anti_slop = anti_slop,
            no = chapter_no,
            title = title,
            context = context,
        )
    } else {
        format!(
            "你是 PenSoul 的执行编辑，根据用户的批注/指令与默认质量项对章节做「审核改写」。\n\
             要求：\n\
             1. 逐条落实用户指令（指令空则按默认质量项）；\n\
             2. 默认质量项：去除 AI 味表达、show-don't-tell（少贴标签）、对话有角色差异、不违反硬约束；\n\
             3. 只改必要处，保持原章节的叙事结构与文风，不要整章重写；\n\
             4. 不得改变情节走向、不得推翻已有设定；\n\
             5. 只输出 JSON：{{\"rewritten\": \"完整改写稿\", \"changes\": [{{\"what\": \"改了什么\", \"why\": \"为什么\"}}]}}。\n\
             硬约束：\n{constraints}\n\
             反 AI 味规范：\n{anti_slop}\n\
             章节背景：第 {no} 章《{title}》\n{context}",
            constraints = constraints,
            anti_slop = anti_slop,
            no = chapter_no,
            title = title,
            context = context,
        )
    }
}

fn rewrite_user_prompt(
    content: &str,
    instructions: Option<&str>,
    de_slop_hits: &[String],
) -> String {
    let mut prompt = String::new();
    if let Some(ins) = instructions {
        let ins = ins.trim();
        if !ins.is_empty() {
            prompt.push_str("用户指令/批注：\n");
            prompt.push_str(ins);
            prompt.push('\n');
        }
    }
    if de_slop_hits.is_empty() {
        prompt.push_str("正文：\n");
    } else {
        prompt.push_str("正文（仅重写命中反 AI 味词的片段）：\n");
    }
    prompt.push_str(content);
    prompt
}

// ---- 单元测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_marks_equal_modified_added_removed() {
        let before = "第一段\n第二段\n第三段";
        let after = "第一段\n第二段（改）\n新增段";
        let diff = diff_sections(before, after);
        let kinds: Vec<&str> = diff.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&"equal"), "应保留未变段: {kinds:?}");
        assert!(kinds.contains(&"modified"), "应有修改段: {kinds:?}");
        assert!(kinds.contains(&"added"), "应有新增段: {kinds:?}");
        // 修改后的文本在 diff 中可见
        let joined = diff.iter().map(|d| d.text.as_str()).collect::<Vec<_>>().join("|");
        assert!(joined.contains("第二段（改）"), "diff 应含改后文本: {joined}");
    }

    #[test]
    fn diff_empty_sides() {
        assert!(diff_sections("", "").is_empty());
        let added = diff_sections("", "只有一段");
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].kind, "added");
    }

    #[test]
    fn rewrite_system_prompt_de_slop_lists_hits() {
        let prompt = rewrite_system_prompt("de-slop", "测试", 1, "{}", "", "", &["静谧".into()]);
        assert!(prompt.contains("静谧"), "消痕模式应列出命中词");
        assert!(prompt.contains("只输出 JSON"));
        let audit = rewrite_system_prompt("audit", "测试", 1, "{}", "硬约束", "", &[]);
        assert!(audit.contains("硬约束"));
        assert!(!audit.contains("仅重写命中"));
    }
}
