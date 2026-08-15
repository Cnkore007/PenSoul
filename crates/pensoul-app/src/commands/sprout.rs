// sprout.rs — 灵魂萌芽 API（对话式创作工作台）
// 所有 LLM 调用统一走 llm_helper（pensoul-infra::llm），产物默认为建议制，确认后才写入正典

use axum::extract::{Form, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::llm::{build_llm_request, default_provider, llm_client, structured_output_tokens};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::ontology::NovelOntology;
use pensoul_domain::{OutlineArc, Setting, SoulSproutSession, SproutProposal};
use pensoul_infra::llm::LlmMessage;

#[derive(Deserialize)]
pub struct ChatParams {
    pub message: String,
    pub perspective: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct EmptyParams {}

/// 获取萌芽会话（对话历史 + 待确认提案）
pub async fn get_session(State(state): State<Arc<RwLock<AppState>>>) -> Result<String, ApiError> {
    let state = state.read().await;
    let session = &state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?
        .soul_sprout;

    serde_json::to_string(&serde_json::json!({
        "messages": session.messages,
        "pending_proposal": session.pending_proposal,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 发送一条想法，LLM 以所选视角回应并参与讨论
pub async fn chat(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<ChatParams>,
) -> Result<String, ApiError> {
    let message = params.message.trim().to_string();
    if message.is_empty() {
        return Err(ApiError::bad_request("消息不能为空"));
    }

    let base_dir = state.read().await.base_dir.clone();
    let provider = default_provider(&base_dir)?;
    let client = llm_client(&provider);

    // 克隆正典上下文与历史，避免 LLM 调用期间持有锁
    let (history, context) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        (
            ontology.soul_sprout.messages.clone(),
            project_context(ontology),
        )
    };

    let perspective = params.perspective.as_deref().unwrap_or("综合");
    let system = chat_system_prompt(perspective, &context);
    let mut messages = to_llm_messages(&history);
    messages.push(LlmMessage {
        role: "user".to_string(),
        content: message.clone(),
    });

    let request = build_llm_request(
        &provider,
        messages,
        system,
        false,
        provider.max_output_tokens.max(2000),
    );
    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("LLM 调用失败: {e}")))?;

    let reply = response.content.trim().to_string();
    if reply.is_empty() {
        return Err(ApiError::bad_request("LLM 返回了空回复，请重试"));
    }

    // 调用成功后一起落盘，避免失败留下半截对话
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.soul_sprout.push_message("user", message);
    ontology.soul_sprout.push_message("assistant", &reply);
    state.save_project().map_err(ApiError::internal)?;

    serde_json::to_string(&serde_json::json!({
        "content": reply,
        "model": response.model,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 开始诘问：生成第一个问题（已有对话时幂等返回当前问题）
pub async fn start(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(_params): Form<EmptyParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let provider = default_provider(&base_dir)?;
    let client = llm_client(&provider);

    let (history, context) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        (
            ontology.soul_sprout.messages.clone(),
            project_context(ontology),
        )
    };

    // 已有对话：返回当前问题，避免重复提问
    if let Some(last) = history.iter().rev().find(|m| m.role == "assistant") {
        return serde_json::to_string(&serde_json::json!({
            "content": last.content,
            "model": "",
        }))
        .map_err(|e| ApiError::internal(e.to_string()));
    }

    let system = start_system_prompt(&context);
    let request = build_llm_request(
        &provider,
        Vec::new(),
        system,
        false,
        provider.max_output_tokens.max(2000),
    );
    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("LLM 调用失败: {e}")))?;

    let question = response.content.trim().to_string();
    if question.is_empty() {
        return Err(ApiError::bad_request("LLM 返回了空问题，请重试"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.soul_sprout.push_message("assistant", &question);
    state.save_project().map_err(ApiError::internal)?;

    serde_json::to_string(&serde_json::json!({
        "content": question,
        "model": response.model,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 基于当前对话生成结构化提案（世界观 + 核心概念 + 大纲）
pub async fn generate(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(_params): Form<EmptyParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let provider = default_provider(&base_dir)?;
    let client = llm_client(&provider);

    let (history, context) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        (
            ontology.soul_sprout.messages.clone(),
            project_context(ontology),
        )
    };

    let system = generate_system_prompt(&context);
    let messages = to_llm_messages(&history);
    let request = build_llm_request(
        &provider,
        messages,
        system,
        true,
        structured_output_tokens(&provider, 8192, 16000),
    );
    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("LLM 调用失败: {e}")))?;

    let proposal = parse_proposal(&response.content)?;

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.soul_sprout.pending_proposal = Some(proposal.clone());
    state.save_project().map_err(ApiError::internal)?;

    serde_json::to_string(&proposal).map_err(|e| ApiError::internal(e.to_string()))
}

/// 用户确认后，把待确认提案写入正典并重建派生状态
pub async fn apply(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(_params): Form<EmptyParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let proposal = ontology
        .soul_sprout
        .pending_proposal
        .clone()
        .ok_or(ApiError::bad_request("没有待应用的提案，请先生成提案"))?;

    // 核心概念：只覆盖非空字段，避免清空已有内容
    let concept = &mut ontology.core_concept;
    overwrite_if_not_empty(&mut concept.high_concept, &proposal.high_concept);
    overwrite_if_not_empty(&mut concept.premise, &proposal.premise);
    overwrite_if_not_empty(&mut concept.protagonist_hint, &proposal.protagonist_hint);
    overwrite_if_not_empty(&mut concept.tone, &proposal.tone);
    overwrite_if_not_empty(&mut concept.central_conflict, &proposal.central_conflict);
    overwrite_if_not_empty(&mut concept.inspiration, &proposal.inspiration);
    overwrite_if_not_empty(&mut ontology.settings.genre, &proposal.genre);

    // 世界观硬规则：按内容去重追加
    for rule in &proposal.world_rules {
        let rule = rule.trim();
        if !rule.is_empty() && !ontology.world.rules.iter().any(|r| r == rule) {
            ontology.world.rules.push(rule.to_string());
        }
    }

    // 世界观设定：追加为新地点设定
    for s in &proposal.world_settings {
        if s.name.trim().is_empty() {
            continue;
        }
        let mut setting = Setting::new(s.name.trim(), s.category.trim());
        setting.description = s.description.trim().to_string();
        ontology.world.locations.push(setting);
    }

    // 大纲脉络：章节范围非法的项跳过
    for a in &proposal.outline_arcs {
        let title = a.title.trim();
        if title.is_empty() || a.chapter_start < 1 || a.chapter_end < a.chapter_start {
            continue;
        }
        let mut arc = OutlineArc::new(title, a.chapter_start, a.chapter_end);
        arc.description = a.description.trim().to_string();
        ontology.outline_arcs.push(arc);
    }

    ontology.soul_sprout.pending_proposal = None;
    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 清空萌芽会话（含待确认提案）
pub async fn clear(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(_params): Form<EmptyParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.soul_sprout = SoulSproutSession::new();
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 丢弃待确认提案（保留对话历史）
pub async fn discard(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(_params): Form<EmptyParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.soul_sprout.pending_proposal = None;
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

// ---- 内部辅助 ----

fn to_llm_messages(history: &[pensoul_domain::SproutMessage]) -> Vec<LlmMessage> {
    history
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| LlmMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect()
}

/// 当前正典摘要：让 LLM 知道项目里已经有什么，避免重复/冲突
fn project_context(ontology: &NovelOntology) -> String {
    let world_settings: Vec<_> = ontology
        .world
        .locations
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "category": s.category,
                "description": s.description,
            })
        })
        .collect();
    let arcs: Vec<_> = ontology
        .outline_arcs
        .iter()
        .map(|a| {
            serde_json::json!({
                "title": a.title,
                "description": a.description,
                "chapters": format!("{}-{}", a.chapter_start, a.chapter_end),
            })
        })
        .collect();

    serde_json::json!({
        "genre": ontology.settings.genre,
        "core_concept": ontology.core_concept,
        "world_rules": ontology.world.rules,
        "world_settings": world_settings,
        "outline_arcs": arcs,
    })
    .to_string()
}

/// 对话系统提示：多视角讨论（参考 PenSoul 专家蒸馏术的多视角评审理念）
fn chat_system_prompt(perspective: &str, context: &str) -> String {
    let perspective_rule = match perspective {
        "结构" => "你以「叙事结构」视角追问：起承转合、章节节奏、冲突升级、伏笔回收，逼问故事的骨架是否立得住。",
        "人物" => "你以「人物情感」视角追问：主角的欲望、恐惧、弱点、成长弧光与人物关系，逼问人物是否有血肉。",
        "世界观" => "你以「世界观」视角追问：设定的自洽性、代价与规则、氛围质感，逼问世界是否可信、有记忆点。",
        _ => "你以「综合」视角追问：平衡结构、人物、世界观三个维度，直奔故事核心。",
    };
    format!(
        "你是 PenSoul 的「灵魂萌芽」诘问式创作助手，像一位犀利而耐心的采访者，\
         用一问一答的方式逼出用户故事的核心。\n\
         当前视角：{perspective_rule}\n\
         铁律：\n\
         1. 一次只问一个问题，绝不把多个问题塞进同一轮；\n\
         2. 用户回答后，先用一两句话提炼他回答中的关键信息（不要评价好坏），\
         然后紧接着抛出下一个问题；\n\
         3. 问题必须具体、尖锐、直指要害：主角的欲望与恐惧、核心冲突、世界观代价、\
         结局方向；禁止问「还有吗」「然后呢」这类泛泛的问题；\n\
         4. 不要替用户做决定，不要写整段设定，不要重复用户已经说过的话；\n\
         5. 按这个顺序覆盖：高概念 → 主角 → 核心冲突 → 世界观 → 基调 → 结局与灵感；\n\
         6. 关键维度都已问过且信息足够时，明确告诉用户「素材已经足够，\
         可以点『生成世界观与大纲提案』」，然后停止提问。\n\
         当前项目已有信息（JSON，不要复述全部）：\n{context}"
    )
}

/// 诘问起始提示：只负责抛出第一个问题
fn start_system_prompt(context: &str) -> String {
    format!(
        "你是 PenSoul 的「灵魂萌芽」诘问式创作助手。现在开始一场一问一答的创作访谈。\n\
         铁律：\n\
         1. 只输出一个问题，一次只问一个，不要自我介绍、不要铺垫、不要举例子；\n\
         2. 第一个问题应该从最核心处入手：问用户心里最想写的是一个什么样的故事，\
         让他用一两句话说出来；\n\
         3. 问题要具体、直接，像一位资深编辑在初次约谈时的发问。\n\
         当前项目已有信息（JSON，若为空则忽略）：\n{context}"
    )
}

/// 提案系统提示：要求严格输出 JSON
fn generate_system_prompt(context: &str) -> String {
    format!(
        "你是 PenSoul 的「灵魂萌芽」整理器。请把对话中用户的想法整理成一份可落地的项目提案，\
         并融合当前项目已有信息（避免与已有内容冲突）。\n\
         严格只输出一个 JSON 对象，不要输出任何其他文字或 Markdown。字段：\n\
         {{\n\
           \"high_concept\": \"一句话高概念\",\n\
           \"premise\": \"故事前提（2-4 句）\",\n\
           \"protagonist_hint\": \"主角设定提示\",\n\
           \"tone\": \"作品基调\",\n\
           \"central_conflict\": \"核心冲突\",\n\
           \"inspiration\": \"灵感来源（如对话中未提及可为空字符串）\",\n\
           \"genre\": \"题材类型\",\n\
           \"world_rules\": [\"世界观硬规则，3-8 条\"],\n\
           \"world_settings\": [{{\"name\":\"设定名\",\"category\":\"分类\",\"description\":\"描述\"}}],\n\
           \"outline_arcs\": [{{\"title\":\"脉络标题\",\"description\":\"脉络内容\",\"chapter_start\":1,\"chapter_end\":20}}]\n\
         }}\n\
         要求：高概念与前提必须完整；大纲脉络 3-6 条且章节范围自洽递增；未讨论清楚的内容宁缺毋滥。\n\
         当前项目已有信息（JSON）：\n{context}"
    )
}

/// 解析 LLM 返回的提案 JSON（容忍代码块包裹与前后空白）
pub fn parse_proposal(raw: &str) -> Result<SproutProposal, ApiError> {
    let mut cleaned = raw.trim();
    for prefix in ["```json", "```"] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            cleaned = rest.trim_start();
        }
    }
    if let Some(rest) = cleaned.strip_suffix("```") {
        cleaned = rest.trim_end();
    }

    let value: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|e| ApiError::bad_request(format!("提案解析失败: {e}")))?;

    let text = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let strings = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    };

    let world_settings = value
        .get("world_settings")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            Some(pensoul_domain::SproutSettingProposal {
                name: v.get("name")?.as_str()?.trim().to_string(),
                category: v
                    .get("category")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                description: v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            })
        })
        .filter(|s| !s.name.is_empty())
        .collect();

    let outline_arcs = value
        .get("outline_arcs")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            Some(pensoul_domain::SproutArcProposal {
                title: v.get("title")?.as_str()?.trim().to_string(),
                description: v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                chapter_start: v.get("chapter_start").and_then(|n| n.as_i64()).unwrap_or(0),
                chapter_end: v.get("chapter_end").and_then(|n| n.as_i64()).unwrap_or(0),
            })
        })
        .filter(|a| !a.title.is_empty())
        .collect();

    let proposal = SproutProposal {
        high_concept: text("high_concept"),
        premise: text("premise"),
        protagonist_hint: text("protagonist_hint"),
        tone: text("tone"),
        central_conflict: text("central_conflict"),
        inspiration: text("inspiration"),
        genre: text("genre"),
        world_rules: strings("world_rules"),
        world_settings,
        outline_arcs,
    };

    if proposal.high_concept.is_empty() {
        return Err(ApiError::bad_request(
            "提案缺少高概念字段，请调整对话后重试",
        ));
    }
    Ok(proposal)
}

fn overwrite_if_not_empty(target: &mut String, source: &str) {
    if !source.trim().is_empty() {
        *target = source.trim().to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proposal_handles_markdown_wrapper() {
        let raw = r#"```json
{"high_concept":"少年踏上不归路","premise":"一个少年…","genre":"奇幻","world_rules":["规则一"],"world_settings":[{"name":"灵气","category":"力量体系","description":"万物有灵"}],"outline_arcs":[{"title":"觉醒","description":"获得力量","chapter_start":1,"chapter_end":10}]}
```"#;
        let proposal = parse_proposal(raw).expect("解析失败");
        assert_eq!(proposal.high_concept, "少年踏上不归路");
        assert_eq!(proposal.world_rules, vec!["规则一"]);
        assert_eq!(proposal.world_settings.len(), 1);
        assert_eq!(proposal.outline_arcs[0].title, "觉醒");
        assert_eq!(proposal.outline_arcs[0].chapter_end, 10);
    }

    #[test]
    fn parse_proposal_rejects_missing_high_concept() {
        let raw = r#"{"premise":"只有前提"}"#;
        assert!(parse_proposal(raw).is_err());
    }
}
