//! 概念讨论命令 — 多 Agent 多轮交锋讨论
//!
//! 流程：
//! - 第 1 轮「立论」：每个 Agent 基于自己的评审提示词独立分析构思
//! - 第 2 轮「交锋」：每个 Agent 看到其他 Agent 的第 1 轮发言摘要，进行回应/质疑/补强
//! - 第 3 轮「成果」：单一综合调用，把全部讨论提炼为结构化成果
//!   （共识总结 + 地点/时间线/设定规则/人物），供前端确认后写入世界观与人物志
//!
//! 每个 Agent 的进度通过 `discussion-event` 事件实时推送给前端。
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Emitter;

use super::llm_helper as lh;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub model: String,
    pub prompt: String,
    pub perspective: String,
    pub enabled: bool,
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

/// 一轮发言记录
#[derive(Debug, Clone, Serialize)]
pub struct AgentTurn {
    pub agent_id: String,
    pub agent_name: String,
    pub perspective: String,
    pub round: u8,
    pub content: String,
}

/// 讨论成果中的地点/设定规则条目
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamedDesc {
    pub name: String,
    pub description: String,
}

/// 讨论成果中的时间线条目
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineItem {
    pub story_time: String,
    pub description: String,
}

/// 讨论成果中的人物条目
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterItem {
    pub name: String,
    #[serde(default)]
    pub personality_traits: Vec<(String, f32)>,
    #[serde(default)]
    pub current_mood: String,
    #[serde(default)]
    pub description: String,
}

/// 结构化讨论成果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscussionSynthesis {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub locations: Vec<NamedDesc>,
    #[serde(default)]
    pub timeline_events: Vec<TimelineItem>,
    #[serde(default)]
    pub setting_rules: Vec<NamedDesc>,
    #[serde(default)]
    pub characters: Vec<CharacterItem>,
}

/// 讨论完整输出
#[derive(Debug, Serialize)]
pub struct DiscussionOutput {
    pub turns: Vec<AgentTurn>,
    pub synthesis: DiscussionSynthesis,
}

/// 概念讨论 — 两轮交锋 + 结构化成果，进度实时推送
#[tauri::command]
pub async fn discuss_concept(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    idea_description: String,
    agents: Vec<AgentConfig>,
) -> Result<DiscussionOutput, String> {
    lh::ensure_api_keys_loaded(&state);

    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };

    let enabled: Vec<&AgentConfig> = agents.iter().filter(|a| a.enabled).collect();
    if enabled.is_empty() {
        return Err("没有启用的 Agent".to_string());
    }

    let mut turns: Vec<AgentTurn> = Vec::new();

    // ── 第 1 轮：立论 ──
    for agent in &enabled {
        emit_discussion(&app_handle, &agent.id, &agent.name, 1, "running", "");
        let user_prompt = format!(
            "以下是故事构思，请基于你的视角进行第一轮立论分析：\n\n{idea_description}\n\n\
             要求：紧扣你的评审视角，给出具体、可操作的判断（哪里好、哪里有问题、怎么改），\
             不要空泛的套话。500 字以内。"
        );
        match call_for_agent(
            agent,
            &user_prompt,
            0.85,
            1536,
            &model_to_provider,
            &provider_api_bases,
            &api_keys,
        )
        .await
        {
            Ok(text) => {
                emit_discussion(&app_handle, &agent.id, &agent.name, 1, "done", &text);
                turns.push(AgentTurn {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    round: 1,
                    content: text,
                });
            }
            Err(msg) => {
                emit_discussion(&app_handle, &agent.id, &agent.name, 1, "error", &msg);
            }
        }
    }

    // ── 第 2 轮：交锋（看到彼此的第 1 轮发言后互相回应）──
    let round1: Vec<AgentTurn> = turns.iter().filter(|t| t.round == 1).cloned().collect();
    if round1.len() >= 2 {
        for agent in &enabled {
            // 跳过第 1 轮失败的 Agent
            let Some(own) = round1.iter().find(|t| t.agent_id == agent.id) else {
                continue;
            };
            emit_discussion(&app_handle, &agent.id, &agent.name, 2, "running", "");

            // 其他 Agent 的第 1 轮发言摘要（每人截断，控制上下文长度）
            let others: String = round1
                .iter()
                .filter(|t| t.agent_id != agent.id)
                .map(|t| {
                    format!(
                        "【{}】：\n{}",
                        t.agent_name,
                        truncate_chars(&t.content, 600)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            let user_prompt = format!(
                "故事构思：\n{idea_description}\n\n\
                 你第一轮的发言：\n{}\n\n\
                 其他评审者的第一轮发言：\n{}\n\n\
                 现在是交锋环节。请回应其他评审者：\n\
                 1. 你明确反对谁、哪一点？说出理由（这是讨论，不要一团和气）\n\
                 2. 谁的观点让你愿意修正自己的判断？修正了什么？\n\
                 3. 基于交锋，给出你认为最重要的 1-2 条落地建议\n\
                 保持你的立场和语言风格，400 字以内。",
                truncate_chars(&own.content, 600),
                others
            );
            match call_for_agent(
                agent,
                &user_prompt,
                0.85,
                1280,
                &model_to_provider,
                &provider_api_bases,
                &api_keys,
            )
            .await
            {
                Ok(text) => {
                    emit_discussion(&app_handle, &agent.id, &agent.name, 2, "done", &text);
                    turns.push(AgentTurn {
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        perspective: agent.perspective.clone(),
                        round: 2,
                        content: text,
                    });
                }
                Err(msg) => {
                    emit_discussion(&app_handle, &agent.id, &agent.name, 2, "error", &msg);
                }
            }
        }
    }

    // ── 第 3 轮：成果提炼（单次综合调用，输出严格 JSON）──
    emit_discussion(&app_handle, "synthesis", "成果提炼", 3, "running", "");
    let synthesis = synthesize(
        &app_handle,
        &enabled,
        &idea_description,
        &turns,
        &model_to_provider,
        &provider_api_bases,
        &api_keys,
    )
    .await;

    Ok(DiscussionOutput { turns, synthesis })
}

/// 为单个 Agent 解析供应商并调用 LLM（system 为该 Agent 的评审提示词）
async fn call_for_agent(
    agent: &AgentConfig,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> Result<String, String> {
    call_with_system(
        &agent.model,
        &agent.prompt,
        user_prompt,
        temperature,
        max_tokens,
        model_to_provider,
        provider_api_bases,
        api_keys,
    )
    .await
}

/// 解析供应商并以指定 system 提示词调用 LLM
#[allow(clippy::too_many_arguments)]
async fn call_with_system(
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

/// 第 3 轮成果提炼：汇总全部发言，输出结构化 JSON
async fn synthesize(
    app_handle: &tauri::AppHandle,
    enabled: &[&AgentConfig],
    idea_description: &str,
    turns: &[AgentTurn],
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> DiscussionSynthesis {
    // 用第一个可解析模型的 Agent 做提炼者
    let Some(caller) = enabled.first() else {
        return DiscussionSynthesis::default();
    };

    let all_turns: String = turns
        .iter()
        .map(|t| {
            format!(
                "【{} · 第{}轮】：\n{}",
                t.agent_name,
                t.round,
                truncate_chars(&t.content, 800)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let system = "你是创作讨论的成果提炼者。你的任务是从多位评审者的讨论中提炼出结构化的创作成果，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "故事构思：\n{idea_description}\n\n\
         全部讨论记录：\n{all_turns}\n\n\
         请把讨论成果提炼为 JSON，用 ===SYNTHESIS_BEGIN=== 和 ===SYNTHESIS_END=== 包裹。结构：\n\
         {{\n\
         \"summary\": \"讨论共识与核心分歧的总结（200字以内）\",\n\
         \"locations\": [{{\"name\": \"地点名\", \"description\": \"描述\"}}],\n\
         \"timeline_events\": [{{\"story_time\": \"故事时间\", \"description\": \"事件描述\"}}],\n\
         \"setting_rules\": [{{\"name\": \"设定规则标题\", \"description\": \"规则描述\"}}],\n\
         \"characters\": [{{\"name\": \"人物名\", \"personality_traits\": [[\"特质\", 0.8]], \
         \"current_mood\": \"初始情绪\", \"description\": \"一句话人物定位\"}}]\n\
         }}\n\
         要求：\n\
         - 只提炼讨论中真正出现的成果，没有讨论到的类别留空数组，不要硬凑\n\
         - 人物从构思中提取核心人物（主角/关键配角），特质 2-4 个，强度 0.0-1.0\n\
         - 所有内容用中文",
    );

    let fallback = |msg: &str| {
        emit_discussion(app_handle, "synthesis", "成果提炼", 3, "error", msg);
        DiscussionSynthesis {
            summary: format!("⚠️ 成果提炼失败: {msg}"),
            ..Default::default()
        }
    };

    let text = match call_with_system(
        &caller.model,
        system,
        &user_prompt,
        0.3,
        2048,
        model_to_provider,
        provider_api_bases,
        api_keys,
    )
    .await
    {
        Ok(t) => t,
        Err(msg) => return fallback(&msg),
    };

    let Some(json_str) = extract_between(&text, "===SYNTHESIS_BEGIN===", "===SYNTHESIS_END===")
    else {
        return fallback("LLM 输出缺少成果标记");
    };
    let json_str = json_str
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<DiscussionSynthesis>(json_str) {
        Ok(mut s) => {
            // setting_rules 的 title 语义：前端用 title，这里统一用 name 接收
            emit_discussion(
                app_handle,
                "synthesis",
                "成果提炼",
                3,
                "done",
                &s.summary.clone(),
            );
            if s.summary.is_empty() {
                s.summary = "（讨论成果已生成）".to_string();
            }
            s
        }
        Err(e) => fallback(&format!("成果 JSON 解析失败: {e}")),
    }
}

/// 提取两个标记之间的内容
fn extract_between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let b = text.find(begin)? + begin.len();
    let e = text.rfind(end)?;
    if e <= b {
        return None;
    }
    Some(&text[b..e])
}

/// 按字符数截断（交锋上下文控制）
fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// 推送讨论进度事件
fn emit_discussion(
    app_handle: &tauri::AppHandle,
    agent_id: &str,
    agent_name: &str,
    round: u8,
    status: &str,
    content: &str,
) {
    let _ = app_handle.emit(
        "discussion-event",
        DiscussionEvent {
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            round,
            status: status.to_string(),
            content: content.to_string(),
        },
    );
}
