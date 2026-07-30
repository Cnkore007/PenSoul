//! 概念讨论命令 — 多 Agent 多轮交锋讨论
//!
//! 流程：
//! - 第 1 轮「立论」：每个 Agent 基于自己的技能与评审提示词，结合构思与创作设定独立分析
//! - 第 2 轮「交锋」：每个 Agent 完整阅读其他 Agent 的第 1 轮发言，进行回应/质疑/补强
//! - 第 3 轮「成果」：单次综合调用，把全部讨论提炼为丰富的结构化成果
//!   （共识总结 + 地点/时间线/设定规则/人物及人物关系），供前端确认后写入世界观与人物志
//!
//! 来自专家库的 Agent 会加载其 SKILL.md 技能文件作为讨论的系统提示词。
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
    #[serde(default)]
    pub description: String,
}

/// 讨论成果中的时间线条目
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineItem {
    pub story_time: String,
    #[serde(default)]
    pub description: String,
}

/// 讨论成果中的人物关系
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationItem {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    #[serde(default = "default_strength")]
    pub strength: f32,
}

fn default_strength() -> f32 {
    0.5
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
    #[serde(default)]
    pub relationships: Vec<RelationItem>,
}

/// 讨论成果中的情节节点（确认后写入大纲）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutlineBeat {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub chapter_hint: String,
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
    #[serde(default)]
    pub outline_beats: Vec<OutlineBeat>,
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
    settings_context: String,
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

    // ── 第 1 轮：立论（构思 + 创作设定 + 各自的技能）──
    for agent in &enabled {
        emit_discussion(&app_handle, &agent.id, &agent.name, 1, "running", "");
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
            2048,
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

    // ── 第 2 轮：交锋（完整阅读彼此发言后互相回应，不截断）──
    let round1: Vec<AgentTurn> = turns.iter().filter(|t| t.round == 1).cloned().collect();
    if round1.len() >= 2 {
        for agent in &enabled {
            // 跳过第 1 轮失败的 Agent
            let Some(own) = round1.iter().find(|t| t.agent_id == agent.id) else {
                continue;
            };
            emit_discussion(&app_handle, &agent.id, &agent.name, 2, "running", "");

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
                2048,
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

    // ── 第 3 轮：成果提炼（完整讨论记录 → 丰富的结构化 JSON）──
    emit_discussion(&app_handle, "synthesis", "成果提炼", 3, "running", "");
    let synthesis = synthesize(
        &app_handle,
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

/// 第 3 轮成果提炼：汇总全部发言（不截断），输出丰富的结构化 JSON
#[allow(clippy::too_many_arguments)]
async fn synthesize(
    app_handle: &tauri::AppHandle,
    enabled: &[&AgentConfig],
    idea_description: &str,
    settings_context: &str,
    turns: &[AgentTurn],
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> DiscussionSynthesis {
    // 用第一个可解析模型的 Agent 做提炼者
    let Some(caller) = enabled.first() else {
        return DiscussionSynthesis::default();
    };

    // 完整讨论记录（不截断，确保成果丰富）
    let all_turns: String = turns
        .iter()
        .map(|t| format!("【{} · 第{}轮】：\n{}", t.agent_name, t.round, t.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system = "你是创作讨论的成果提炼者。你的任务是从多位评审者的讨论中提炼出丰富、具体、\
        可直接落地的创作成果，输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "【故事构思】\n{idea_description}\n\n\
         【创作设定】\n{settings_context}\n\n\
         【全部讨论记录】\n{all_turns}\n\n\
         这场讨论的目的是帮助作者把构思落地为可写作的世界观与人物志。\
         请把讨论成果提炼为丰富的 JSON，用 ===SYNTHESIS_BEGIN=== 和 ===SYNTHESIS_END=== 包裹。结构：\n\
         {{\n\
         \"summary\": \"讨论共识、核心分歧与给作者的总体建议（300-500字，要具体，引用讨论中的关键观点）\",\n\
         \"locations\": [{{\"name\": \"地点名\", \"description\": \"100-200字的完整描述：外观、氛围、功能、与故事的关系\"}}],\n\
         \"timeline_events\": [{{\"story_time\": \"故事时间\", \"description\": \"80-150字：事件经过及其对后续的影响\"}}],\n\
         \"setting_rules\": [{{\"name\": \"设定规则标题\", \"description\": \"100-200字：规则内容、约束、代价、可被利用的漏洞\"}}],\n\
         \"characters\": [{{\"name\": \"人物名\", \"personality_traits\": [[\"特质\", 0.8]], \
         \"current_mood\": \"登场时的心境\", \"description\": \"100-200字：身份、欲望、恐惧、在故事中的功能\", \
         \"relationships\": [{{\"from\": \"人物名\", \"to\": \"人物名\", \"relation_type\": \"关系类型\", \"strength\": 0.7}}]}}],\n\
         \"outline_beats\": [{{\"title\": \"情节节点标题\", \"description\": \"100-200字：该节点发生什么、核心冲突是什么、如何推进主线\", \"chapter_hint\": \"建议章节范围，如 第1-3章\"}}]\n\
         }}\n\
         要求：\n\
         - 成果必须丰富：充分吸收讨论中出现的所有有价值设定，宁多勿缺；每个类别尽量覆盖讨论提到的内容\n\
         - 没有讨论到的类别留空数组，但不要遗漏讨论中实际提到的设定\n\
         - 人物从构思与讨论中提取所有核心人物（主角/关键配角/重要反派），特质 3-5 个，强度 0.0-1.0\n\
         - 人物关系只描述提炼出的人物之间的关系，strength 0.0-1.0\n\
         - 情节脉络按故事发生顺序排列，覆盖开端、发展、转折、高潮、结局的关键节点，并结合创作设定中的章数与字数规划来切分节点粒度\n\
         - 描述要写成可直接交给作者使用的设定文字，不要写成「讨论认为」的转述\n\
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
        8192,
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
