//! 概念讨论命令 — 多 Agent 多轮交锋讨论
//!
//! 流程：
//! - 第 1 轮「立论」：每个 Agent 基于自己的技能与评审提示词，结合构思与创作设定独立分析
//! - 第 2 轮「交锋」：每个 Agent 完整阅读其他 Agent 的第 1 轮发言，进行回应/质疑/补强
//! - 第 3 轮「成果」：单次综合调用，把全部讨论提炼为丰富的结构化成果
//!   （共识总结 + 地点/时间线/设定规则/人物及人物关系），供前端确认后写入世界观与人物志
//!
//! 来自专家库的 Agent 会加载其 SKILL.md 技能文件作为讨论的系统提示词。
//! 每个 Agent 的进度通过 `discussion-event` 事件实时推送给前端；
//! 事件同时写入控制面缓冲、结果持久化到 `sprout.last_discussion`，
//! 前端切换页面后可通过 `get_discussion_state` 重连恢复。
use crate::state::AppState;
use pensoul_core::{AgentTurn, DiscussionRecord, DiscussionSynthesis};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::discussion_synthesis;
use super::llm_helper as lh;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub model: String,
    pub prompt: String,
    pub perspective: String,
    pub enabled: bool,
    /// 关联专家的技能文件路径（可选，来自专家库的 Agent 携带）
    pub skill_path: Option<String>,
}

/// 讨论进度事件 —— 实时推送给前端
#[derive(Debug, Clone, Serialize)]
pub struct DiscussionEvent {
    pub agent_id: String,
    pub agent_name: String,
    /// 1=立论 2=交锋 3=成果
    pub round: u8,
    /// running / done / error
    pub status: String,
    pub content: String,
}

/// 讨论事件缓冲上限（环形：满了丢最旧）
const DISCUSSION_EVENT_CAP: usize = 100;

/// 讨论控制面（挂 AppState）：运行旗标 + 事件缓冲，支持页面切换后重连
pub struct DiscussionControl {
    /// 是否有讨论正在进行（防重入）
    pub running: AtomicBool,
    /// 事件环形缓冲：前端重连后重放恢复进度
    pub events: parking_lot::RwLock<Vec<DiscussionEvent>>,
}

impl DiscussionControl {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            events: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// 事件入缓冲（发射前调用）
    fn record(&self, ev: &DiscussionEvent) {
        let mut buf = self.events.write();
        if buf.len() >= DISCUSSION_EVENT_CAP {
            buf.remove(0);
        }
        buf.push(ev.clone());
    }
}

impl Default for DiscussionControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 讨论完整输出
#[derive(Debug, Serialize)]
pub struct DiscussionOutput {
    pub turns: Vec<AgentTurn>,
    pub synthesis: DiscussionSynthesis,
}

/// 概念讨论 — 两轮交锋 + 结构化成果，进度实时推送
///
/// 防重入 + 结果持久化：同一时刻只允许一场讨论；完成后把发言与成果
/// 写入 `sprout.last_discussion` 并落盘——即使前端中途切走页面，
/// 讨论在后台继续，结果也不会丢失（前端重连走 `get_discussion_state`）。
#[tauri::command]
pub async fn discuss_concept(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    idea_description: String,
    settings_context: String,
    agents: Vec<AgentConfig>,
) -> Result<DiscussionOutput, String> {
    if state.discussion.running.swap(true, Ordering::SeqCst) {
        return Err("已有讨论正在进行，请等待当前讨论完成".to_string());
    }
    // 新一场讨论：清空事件缓冲（供前端重连重放）
    state.discussion.events.write().clear();

    let output = discuss_inner(
        &app_handle,
        &state,
        idea_description,
        settings_context,
        agents,
    )
    .await;

    // 无论成败都释放运行旗标；成功时把结果持久化到萌芽数据
    state.discussion.running.store(false, Ordering::SeqCst);
    if let Ok(out) = &output {
        {
            let mut onto = state.ontology.write();
            onto.sprout.last_discussion = Some(DiscussionRecord {
                turns: out.turns.clone(),
                synthesis: out.synthesis.clone(),
                author_feedback: String::new(),
            });
        }
        if let Err(e) = state.save() {
            eprintln!("讨论结果落盘失败: {e}");
        }
        // 终态事件：在持久化之后发射，前端收到后可立即安全拉取结果
        emit_discussion(
            &app_handle,
            &state,
            "__discussion__",
            "讨论",
            0,
            "finished",
            "",
        );
    }
    output
}

/// 讨论状态查询：运行旗标 + 事件缓冲（前端切换页面后重连恢复进度用）
#[tauri::command]
pub async fn get_discussion_state(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "running": state.discussion.running.load(Ordering::SeqCst),
        "events": state.discussion.events.read().clone(),
    }))
}

/// 讨论主流程（两轮交锋 + 成果提炼）
async fn discuss_inner(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    idea_description: String,
    settings_context: String,
    agents: Vec<AgentConfig>,
) -> Result<DiscussionOutput, String> {
    lh::ensure_api_keys_loaded(state);

    let saved_providers = lh::load_providers(state);
    let saved_models = lh::load_models(state);
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };

    let enabled: Vec<&AgentConfig> = agents.iter().filter(|a| a.enabled).collect();
    if enabled.is_empty() {
        return Err("没有启用的 Agent".to_string());
    }

    let mut turns: Vec<AgentTurn> = Vec::new();

    // ── 第 1 轮：立论（构思 + 创作设定 + 各自的技能）──
    for agent in &enabled {
        emit_discussion(app_handle, state, &agent.id, &agent.name, 1, "running", "");
        let user_prompt = format!(
            "【故事构思】\n{idea_description}\n\n\
             【创作设定】\n{settings_context}\n\n\
             请基于你的视角进行第一轮立论分析。要求：\n\
             1. 紧扣你的评审视角，给出具体、可操作的判断（哪里好、哪里有问题、怎么改），不要空泛的套话\n\
             2. 分析必须结合创作设定（篇幅、章数、类型等）——构思与设定不匹配的地方要指出来\n\
             3. 可以质疑构思或设定本身的合理性，并给出优化建议\n\
             800 字以内。"
        );
        let system = build_system_prompt(agent);
        match call_with_system(
            &agent.model,
            &system,
            &user_prompt,
            0.85,
            8192,
            &model_to_provider,
            &provider_api_bases,
            &api_keys,
        )
        .await
        {
            Ok(text) => {
                emit_discussion(app_handle, state, &agent.id, &agent.name, 1, "done", &text);
                turns.push(AgentTurn {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    round: 1,
                    content: text,
                });
            }
            Err(msg) => {
                emit_discussion(app_handle, state, &agent.id, &agent.name, 1, "error", &msg);
            }
        }
    }

    // ── 第 2 轮：交锋（完整阅读彼此发言后互相回应，不截断）──
    let round1: Vec<AgentTurn> = turns.iter().filter(|t| t.round == 1).cloned().collect();
    if round1.len() >= 2 {
        for agent in &enabled {
            // 跳过第 1 轮失败的 Agent
            let Some(own) = round1.iter().find(|t| t.agent_id == agent.id) else {
                continue;
            };
            emit_discussion(app_handle, state, &agent.id, &agent.name, 2, "running", "");

            // 其他 Agent 的第 1 轮完整发言（不截断，充分理解后再回应）
            let others: String = round1
                .iter()
                .filter(|t| t.agent_id != agent.id)
                .map(|t| format!("【{}（{}）】：\n{}", t.agent_name, t.perspective, t.content))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");

            let user_prompt = format!(
                "【故事构思】\n{idea_description}\n\n\
                 【创作设定】\n{settings_context}\n\n\
                 【你第一轮的发言】\n{}\n\n\
                 【其他评审者的第一轮发言】\n{}\n\n\
                 现在是交锋环节。请先完整理解其他评审者的发言，再给出你的回应：\n\
                 1. 你明确反对谁、哪一点？说出理由（这是讨论，不要一团和气）\n\
                 2. 谁的观点让你愿意修正自己的判断？修正了什么？\n\
                 3. 对其他评审者遗漏的重要问题做补充\n\
                 4. 基于交锋，给出你认为最重要的 2-3 条落地建议（面向作者，可直接执行）\n\
                 保持你的立场和语言风格，600 字以内。",
                own.content, others
            );
            let system = build_system_prompt(agent);
            match call_with_system(
                &agent.model,
                &system,
                &user_prompt,
                0.85,
                8192,
                &model_to_provider,
                &provider_api_bases,
                &api_keys,
            )
            .await
            {
                Ok(text) => {
                    emit_discussion(app_handle, state, &agent.id, &agent.name, 2, "done", &text);
                    turns.push(AgentTurn {
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        perspective: agent.perspective.clone(),
                        round: 2,
                        content: text,
                    });
                }
                Err(msg) => {
                    emit_discussion(app_handle, state, &agent.id, &agent.name, 2, "error", &msg);
                }
            }
        }
    }

    // ── 第 3 轮：成果提炼（分维度并行提炼 + 冲突检查 + 裁决）──
    let synthesis = synthesize(
        app_handle,
        state,
        &enabled,
        &idea_description,
        &settings_context,
        &turns,
        &model_to_provider,
        &provider_api_bases,
        &api_keys,
    )
    .await;

    Ok(DiscussionOutput { turns, synthesis })
}

/// 构建 Agent 的系统提示词：优先加载其专家技能文件（SKILL.md），
/// 再附上该 Agent 的评审重点提示。无技能文件时只用评审提示词。
fn build_system_prompt(agent: &AgentConfig) -> String {
    let mut parts = Vec::new();
    if let Some(skill) = load_skill_content(agent.skill_path.as_deref()) {
        parts.push(skill);
    }
    parts.push(format!(
        "【本次讨论中你的角色】\n你是「{}」，评审视角：{}。\n{}",
        agent.name, agent.perspective, agent.prompt
    ));
    parts.join("\n\n")
}

/// 读取专家技能文件内容。安全约束：必须是 <名字>-expert/ 或 <名字>-perspective/
/// 目录下名为 SKILL.md 的文件；内容超过 12000 字符时截断（保护上下文窗口）。
fn load_skill_content(skill_path: Option<&str>) -> Option<String> {
    let path = std::path::Path::new(skill_path?);
    if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
        return None;
    }
    let dir_name = path.parent()?.file_name()?.to_str()?;
    if !(dir_name.ends_with("-expert") || dir_name.ends_with("-perspective")) {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    const MAX: usize = 12_000;
    let truncated = if content.chars().count() > MAX {
        format!(
            "{}…（技能内容过长已截断）",
            content.chars().take(MAX).collect::<String>()
        )
    } else {
        content
    };
    Some(format!(
        "【你的思维技能卡】\n{truncated}\n\n\
         以上是你的思维方式与表达风格，讨论时请全程以这个身份思考与发言。"
    ))
}

/// 解析供应商并以指定 system 提示词调用 LLM
#[allow(clippy::too_many_arguments)]
pub(crate) async fn call_with_system(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> Result<String, String> {
    let (provider_id, api_key, api_base) =
        lh::resolve_provider(model, model_to_provider, provider_api_bases, api_keys)?;
    lh::call_llm(
        &lh::ProviderAuth {
            provider_id: &provider_id,
            api_key: &api_key,
            api_base: &api_base,
        },
        model,
        system_prompt,
        user_prompt,
        temperature,
        max_tokens,
    )
    .await
}

/// 第 3 轮成果提炼：分维度并行提炼 + 跨维度冲突检查 + 裁判裁决
#[allow(clippy::too_many_arguments)]
async fn synthesize(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    enabled: &[&AgentConfig],
    idea_description: &str,
    settings_context: &str,
    turns: &[AgentTurn],
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> DiscussionSynthesis {
    let ctx = discussion_synthesis::SynthesisContext {
        app_handle,
        state,
        enabled,
        idea_description,
        settings_context,
        turns,
        model_to_provider,
        provider_api_bases,
        api_keys,
    };
    discussion_synthesis::synthesize(&ctx).await
}

/// 推送讨论进度事件：先入控制面缓冲（供页面重连重放），再推送给前端
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_discussion(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    agent_id: &str,
    agent_name: &str,
    round: u8,
    status: &str,
    content: &str,
) {
    let ev = DiscussionEvent {
        agent_id: agent_id.to_string(),
        agent_name: agent_name.to_string(),
        round,
        status: status.to_string(),
        content: content.to_string(),
    };
    state.discussion.record(&ev);
    let _ = app_handle.emit("discussion-event", ev);
}
