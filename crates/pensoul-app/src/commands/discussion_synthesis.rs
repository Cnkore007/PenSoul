//! 讨论成果提炼 —— 分维度提炼 + 跨维度冲突检查 + 裁判裁决
//!
//! 依据调研结论落地（替换原先「单次综合调用」的合成式提炼）：
//! - 选择聚集优于合成聚集：五路并行按维度独立提炼，避免把所有发言揉成一段共识；
//! - 聚合器应读完整推理轨迹：每个维度输出显式携带分歧（各方立场 + 依据），不静默抹平；
//! - 防强模型被弱模型带偏：跨维度冲突检查显式标记矛盾；未收敛的分歧交由独立裁判裁决。

use std::collections::HashMap;

use pensoul_core::{
    AgentTurn, CharacterItem, Disagreement, DiscussionSynthesis, NamedDesc, OutlineBeat,
    TimelineItem,
};
use serde::Deserialize;
use tauri::AppHandle;

use crate::state::AppState;

use super::discussion::{call_with_system, emit_discussion, AgentConfig};
use super::json_fix;

/// 提炼所需的外部依赖与上下文
pub struct SynthesisContext<'a> {
    pub app_handle: &'a AppHandle,
    pub state: &'a AppState,
    pub enabled: &'a [&'a AgentConfig],
    pub idea_description: &'a str,
    pub settings_context: &'a str,
    pub turns: &'a [AgentTurn],
    pub model_to_provider: &'a HashMap<String, String>,
    pub provider_api_bases: &'a HashMap<String, String>,
    pub api_keys: &'a HashMap<String, String>,
}

/// 单个提炼维度的定义
struct DimensionDef {
    /// 事件 agent_id 后缀（discussion-synthesis-<id>）
    id: &'static str,
    /// 中文显示名（进度事件用）
    name: &'static str,
    /// 该维度的提炼聚焦说明
    focus: &'static str,
    /// 该维度输出的 JSON 结构说明
    schema: &'static str,
}

const DIM_WORLD: DimensionDef = DimensionDef {
    id: "world",
    name: "世界观 · 地点与时间线",
    focus: "地点与时间线：故事发生的场景、地域风貌，以及故事时间脉络（时代、阶段、关键时点）",
    schema: r#"{"locations": [{"name": "地点名", "description": "100-200字的完整描述：外观、氛围、功能、与故事的关系"}], "timeline_events": [{"story_time": "故事时间", "description": "80-150字：事件经过及其对后续的影响"}], "disagreements": [分歧列表]}"#,
};

const DIM_RULES: DimensionDef = DimensionDef {
    id: "rules",
    name: "设定规则提炼",
    focus: "设定规则：世界运行的法则、力量体系、社会/修行/文明规则，以及规则带来的约束、代价与可被利用的漏洞",
    schema: r#"{"setting_rules": [{"name": "设定规则标题", "description": "100-200字：规则内容、约束、代价、可被利用的漏洞"}], "disagreements": [分歧列表]}"#,
};

const DIM_CHARACTERS: DimensionDef = DimensionDef {
    id: "characters",
    name: "人物与关系提炼",
    focus: "人物及人物关系：主角、关键配角、重要反派的身份、欲望、恐惧、故事功能，以及人物之间的关系",
    schema: r#"{"characters": [{"name": "人物名", "personality_traits": [["特质", 0.8]], "current_mood": "登场时的心境", "description": "100-200字：身份、欲望、恐惧、在故事中的功能", "relationships": [{"from": "人物名", "to": "人物名", "relation_type": "关系类型", "strength": 0.7}]}], "disagreements": [分歧列表]}"#,
};

const DIM_BEATS: DimensionDef = DimensionDef {
    id: "beats",
    name: "情节脉络提炼",
    focus: "情节脉络：开端、发展、转折、高潮、结局的关键节点，以及它们如何推进主线",
    schema: r#"{"outline_beats": [{"title": "情节节点标题", "description": "100-200字：该节点发生什么、核心冲突是什么、如何推进主线", "chapter_hint": "建议章节范围，如 第1-3章"}], "disagreements": [分歧列表]}"#,
};

const DIM_SUMMARY: DimensionDef = DimensionDef {
    id: "summary",
    name: "共识与分歧提炼",
    focus: "共识总结、核心分歧与给作者的总体建议（这是给作者的总览，必须具体、引用讨论中的关键观点）",
    schema: r#"{"summary": "300-500字", "disagreements": [分歧列表]}"#,
};

/// 统一的分维度提炼结果：每个维度只填充自己负责的字段
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct DimensionResult {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    locations: Vec<NamedDesc>,
    #[serde(default)]
    timeline_events: Vec<TimelineItem>,
    #[serde(default)]
    setting_rules: Vec<NamedDesc>,
    #[serde(default)]
    characters: Vec<CharacterItem>,
    #[serde(default)]
    outline_beats: Vec<OutlineBeat>,
    #[serde(default)]
    disagreements: Vec<Disagreement>,
}

/// 跨维度冲突检查结果
#[derive(Debug, Default, Deserialize)]
struct ConflictCheckResult {
    #[serde(default)]
    conflicts: Vec<Disagreement>,
}

/// 裁判对单条分歧的裁决
#[derive(Debug, Default, Deserialize)]
struct Verdict {
    topic: String,
    #[serde(default)]
    verdict: String,
    #[serde(default)]
    rationale: String,
    #[serde(default)]
    final_text: String,
}

#[derive(Debug, Default, Deserialize)]
struct AdjudicationResult {
    #[serde(default)]
    verdicts: Vec<Verdict>,
}

/// 讨论成果提炼主流程：并行分维度提炼 → 跨维度冲突检查 → 裁判裁决
pub async fn synthesize(ctx: &SynthesisContext<'_>) -> DiscussionSynthesis {
    let Some(caller) = ctx.enabled.first() else {
        return DiscussionSynthesis::default();
    };
    // 完整讨论记录（不截断）：每个维度都基于完整推理轨迹提炼
    let all_turns = render_turns(ctx.turns);

    // ── 1. 五路并行分维度提炼 ──
    let (world, rules, chars, beats, summary) = tokio::join!(
        extract_dimension(ctx, &all_turns, caller, &DIM_WORLD),
        extract_dimension(ctx, &all_turns, caller, &DIM_RULES),
        extract_dimension(ctx, &all_turns, caller, &DIM_CHARACTERS),
        extract_dimension(ctx, &all_turns, caller, &DIM_BEATS),
        extract_dimension(ctx, &all_turns, caller, &DIM_SUMMARY),
    );

    let dims = [&world, &rules, &chars, &beats, &summary];

    // ── 2. 跨维度冲突检查（显式标记矛盾，不静默抹平）──
    let mut disagreements: Vec<Disagreement> = dims
        .iter()
        .flat_map(|d| d.disagreements.clone())
        .collect();
    let conflicts = check_conflicts(ctx, &all_turns, &dims, caller).await;
    disagreements.extend(conflicts);

    // ── 3. 裁判裁决未收敛分歧（独立于提炼者的模型）──
    let open: Vec<Disagreement> = disagreements
        .iter()
        .filter(|d| d.status == "open")
        .cloned()
        .collect();
    if !open.is_empty() {
        let adjudicator = ctx
            .enabled
            .get(1)
            .copied()
            .unwrap_or(caller);
        let verdicts = adjudicate(ctx, &all_turns, &open, adjudicator).await;
        for d in disagreements.iter_mut() {
            if d.status != "open" {
                continue;
            }
            if let Some(v) = verdicts
                .iter()
                .find(|(t, _)| topic_matches(t, &d.topic))
                .map(|(_, v)| v)
            {
                d.resolution = if v.final_text.is_empty() {
                    v.verdict.clone()
                } else {
                    format!("{}（{}）", v.verdict, v.final_text)
                };
                if !v.rationale.is_empty() {
                    d.resolution = format!("{}｜理由：{}", d.resolution, v.rationale);
                }
                d.adjudicated = true;
            }
        }
    }

    merge(dims, disagreements)
}

/// 把全部发言渲染成文本（标明评审者与轮次）
fn render_turns(turns: &[AgentTurn]) -> String {
    turns
        .iter()
        .map(|t| format!("【{} · 第{}轮】：\n{}", t.agent_name, t.round, t.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// 单维度提炼：一次调用，输出该维度的结构化条目 + 显式分歧
async fn extract_dimension(
    ctx: &SynthesisContext<'_>,
    all_turns: &str,
    caller: &AgentConfig,
    dim: &DimensionDef,
) -> DimensionResult {
    let agent_id = format!("dim-{}", dim.id);
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        &agent_id,
        dim.name,
        3,
        "running",
        "",
    );

    let system = "你是创作讨论的分维度提炼者。你的任务是从多位评审者的完整讨论中，\
        只提炼出自己负责的维度，输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "【故事构思】\n{}\n\n\
         【创作设定】\n{}\n\n\
         【全部讨论记录（完整，不要截断）】\n{}\n\n\
         【本轮提炼任务】你只负责「{}」。\n\
         要求：\n\
         1. 只提取与「{}」相关的内容，宁多勿缺；无关内容一律忽略\n\
         2. 每条内容必须是可直接交给作者使用的设定文字，不要写成「讨论认为」「建议」式转述\n\
         3. 条目必须能从讨论记录中找到依据，禁止凭空添加讨论中未出现的内容\n\
         4. 同时输出 disagreements：本维度内评审者之间真实存在的分歧，每条含 topic（议题）、\
         dimension（当前维度名）、sides（各方：[{{\"agent\": 评审者名, \"position\": 立场, \
         \"rationale\": 依据}}]）、status（讨论中已收敛填 resolved 并写明 resolution；未收敛填 open，\
         resolution 留空）、resolution；没有分歧则 disagreements 为空数组\n\
         5. 全部内容用中文\n\
         用 ===DIMENSION_BEGIN=== 与 ===DIMENSION_END=== 包裹纯 JSON，结构如下：\n{}\n\
         若某字段本轮不产出，用空数组/空字符串，但不要遗漏讨论中实际提到的相关内容。",
        ctx.idea_description,
        ctx.settings_context,
        all_turns,
        dim.name,
        dim.focus,
        dim.schema,
    );

    let mut next_prompt = user_prompt.clone();
    let mut last_err = String::new();
    for attempt in 1..=2u8 {
        let text = match call_with_system(
            &caller.model,
            system,
            &next_prompt,
            0.3,
            8192,
            ctx.model_to_provider,
            ctx.provider_api_bases,
            ctx.api_keys,
        )
        .await
        {
            Ok(t) => t,
            Err(msg) => {
                last_err = msg;
                continue;
            }
        };

        let json_str = extract_dimension_block(&text);
        let parsed = serde_json::from_str::<DimensionResult>(json_str).or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<DimensionResult>(v).ok())
                .ok_or(strict_err)
        });
        match parsed {
            Ok(result) => {
                let brief = summarize_dimension(dim, &result);
                emit_discussion(
                    ctx.app_handle,
                    ctx.state,
                    &agent_id,
                    dim.name,
                    3,
                    "done",
                    &brief,
                );
                return result;
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < 2 {
                    emit_discussion(
                        ctx.app_handle,
                        ctx.state,
                        &agent_id,
                        dim.name,
                        3,
                        "running",
                        "首次输出无法解析，正在自愈重试…",
                    );
                    let head: String = text.chars().take(200).collect();
                    next_prompt = format!(
                        "{user_prompt}\n\n\
                         ⚠️ 你上一次的输出无法解析为 JSON（错误：{last_err}），开头内容是：\n\
                         「{head}…」\n\
                         这一次请只输出 ===DIMENSION_BEGIN=== 与 ===DIMENSION_END=== 包裹的纯 JSON，\
                         不要任何解释、前言、思考过程或 markdown 代码围栏。",
                    );
                }
            }
        }
    }
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        &agent_id,
        dim.name,
        3,
        "error",
        &format!("提炼失败: {last_err}"),
    );
    DimensionResult::default()
}

/// 跨维度冲突检查：找出人物/规则/时间线/情节之间的真实矛盾，显式标记
async fn check_conflicts(
    ctx: &SynthesisContext<'_>,
    all_turns: &str,
    dims: &[&DimensionResult; 5],
    caller: &AgentConfig,
) -> Vec<Disagreement> {
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        "conflict-check",
        "跨维度冲突检查",
        3,
        "running",
        "",
    );

    let rendered: String = dims
        .iter()
        .filter_map(|d| {
            let json = serde_json::to_string(d).ok()?;
            let body: String = serde_json::from_str(&json)
                .map(|v: serde_json::Value| {
                    ["locations", "timeline_events", "setting_rules", "characters", "outline_beats"]
                        .iter()
                        .filter_map(|k| v.get(*k).map(|x| x.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|_| json);
            if body.trim().is_empty() { None } else { Some(body) }
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    if rendered.trim().is_empty() {
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            "conflict-check",
            "跨维度冲突检查",
            3,
            "done",
            "无可检查内容",
        );
        return Vec::new();
    }

    let system = "你是创作设定的冲突检查员。你的任务是从分维度提炼结果中找出跨维度的真实矛盾，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "【全部讨论记录（供核对依据）】\n{all_turns}\n\n\
         【各维度提炼结果】\n{rendered}\n\n\
         找出各维度之间的不协调与矛盾，例如：人物设定与设定规则冲突、时间线与情节节点矛盾、\
         地点与设定规则不符等。输出：\n\
         {{\"conflicts\": [{{\"topic\": \"矛盾议题\", \"dimension\": \"跨维度冲突\", \
         \"sides\": [{{\"agent\": \"冲突来源（维度/评审者）\", \"position\": \"一方内容\", \
         \"rationale\": \"依据\"}}], \"status\": \"open\", \"resolution\": \"\"}}]}}\n\
         要求：\n\
         1. 只报告真实矛盾，必须能从提炼结果与讨论记录中确认，不要无中生有\n\
         2. 每条矛盾说明双方内容冲突在哪里，以及为什么是矛盾\n\
         3. 没有矛盾时输出空数组\n\
         用 ===CONFLICT_BEGIN=== 与 ===CONFLICT_END=== 包裹纯 JSON，全部内容用中文。",
    );

    let mut next_prompt = user_prompt.clone();
    let mut last_err = String::new();
    for attempt in 1..=2u8 {
        let text = match call_with_system(
            &caller.model,
            system,
            &next_prompt,
            0.2,
            4096,
            ctx.model_to_provider,
            ctx.provider_api_bases,
            ctx.api_keys,
        )
        .await
        {
            Ok(t) => t,
            Err(msg) => {
                last_err = msg;
                continue;
            }
        };
        let json_str = extract_block(&text, "===CONFLICT_BEGIN===", "===CONFLICT_END===");
        let parsed = serde_json::from_str::<ConflictCheckResult>(json_str).or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<ConflictCheckResult>(v).ok())
                .ok_or(strict_err)
        });
        match parsed {
            Ok(mut r) => {
                for c in &mut r.conflicts {
                    if c.dimension.is_empty() {
                        c.dimension = "跨维度冲突".to_string();
                    }
                }
                emit_discussion(
                    ctx.app_handle,
                    ctx.state,
                    "conflict-check",
                    "跨维度冲突检查",
                    3,
                    "done",
                    &format!("发现 {} 处跨维度矛盾", r.conflicts.len()),
                );
                return r.conflicts;
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < 2 {
                    let head: String = text.chars().take(200).collect();
                    next_prompt = format!(
                        "{user_prompt}\n\n\
                         ⚠️ 你上一次的输出无法解析为 JSON（错误：{last_err}），开头内容是：\n\
                         「{head}…」\n这一次请只输出纯 JSON，不要任何额外文字。",
                    );
                }
            }
        }
    }
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        "conflict-check",
        "跨维度冲突检查",
        3,
        "error",
        &format!("冲突检查失败: {last_err}"),
    );
    Vec::new()
}

/// 独立裁判裁决未收敛的分歧（采用与提炼者不同的模型，防同源带偏）
async fn adjudicate(
    ctx: &SynthesisContext<'_>,
    all_turns: &str,
    open: &[Disagreement],
    adjudicator: &AgentConfig,
) -> Vec<(String, Verdict)> {
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        "adjudicator",
        "分歧裁决",
        3,
        "running",
        &format!("共 {} 项分歧待裁决", open.len()),
    );

    let open_json = serde_json::to_string_pretty(&open).unwrap_or_default();
    let system = "你是创作设定的独立裁判。你负责对讨论中未收敛的分歧做出最终裁决，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "【故事构思】\n{}\n\n\
         【创作设定】\n{}\n\n\
         【全部讨论记录（供核对依据）】\n{all_turns}\n\n\
         【待裁决分歧】\n{open_json}\n\n\
         对每条分歧给出裁决，输出：\n\
         {{\"verdicts\": [{{\"topic\": \"议题（必须与输入完全一致）\", \
         \"verdict\": \"采纳哪一方或折中方案（一句话）\", \
         \"rationale\": \"裁决理由（引用讨论中的具体依据）\", \
         \"final_text\": \"可直接采用的最终设定文字（100-200字，供作者直接使用）\"}}]}}\n\
         要求：\n\
         1. 裁决必须有利于故事的整体一致性，不能只偏向一方\n\
         2. final_text 要写成可直接落地的设定文字，不要「建议」「可以」式措辞\n\
         3. 全部内容用中文\n\
         用 ===VERDICT_BEGIN=== 与 ===VERDICT_END=== 包裹纯 JSON。",
        ctx.idea_description, ctx.settings_context,
    );

    let mut next_prompt = user_prompt.clone();
    let mut last_err = String::new();
    for attempt in 1..=2u8 {
        let text = match call_with_system(
            &adjudicator.model,
            system,
            &next_prompt,
            0.2,
            4096,
            ctx.model_to_provider,
            ctx.provider_api_bases,
            ctx.api_keys,
        )
        .await
        {
            Ok(t) => t,
            Err(msg) => {
                last_err = msg;
                continue;
            }
        };
        let json_str = extract_block(&text, "===VERDICT_BEGIN===", "===VERDICT_END===");
        let parsed = serde_json::from_str::<AdjudicationResult>(json_str).or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<AdjudicationResult>(v).ok())
                .ok_or(strict_err)
        });
        match parsed {
            Ok(r) => {
                emit_discussion(
                    ctx.app_handle,
                    ctx.state,
                    "adjudicator",
                    "分歧裁决",
                    3,
                    "done",
                    &format!("完成 {} 项裁决", r.verdicts.len()),
                );
                return r
                    .verdicts
                    .into_iter()
                    .map(|v| (v.topic.clone(), v))
                    .collect();
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt < 2 {
                    let head: String = text.chars().take(200).collect();
                    next_prompt = format!(
                        "{user_prompt}\n\n\
                         ⚠️ 你上一次的输出无法解析为 JSON（错误：{last_err}），开头内容是：\n\
                         「{head}…」\n这一次请只输出纯 JSON，不要任何额外文字。",
                    );
                }
            }
        }
    }
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        "adjudicator",
        "分歧裁决",
        3,
        "error",
        &format!("裁决失败: {last_err}"),
    );
    Vec::new()
}

/// 合并各维度结果与分歧，组装最终成果
fn merge(dims: [&DimensionResult; 5], disagreements: Vec<Disagreement>) -> DiscussionSynthesis {
    let world = dims[0];
    let rules = dims[1];
    let chars = dims[2];
    let beats = dims[3];
    let summary = dims[4];

    let mut out = DiscussionSynthesis {
        summary: summary.summary.clone(),
        locations: world.locations.clone(),
        timeline_events: world.timeline_events.clone(),
        setting_rules: rules.setting_rules.clone(),
        characters: chars.characters.clone(),
        outline_beats: beats.outline_beats.clone(),
        disagreements,
    };
    if out.summary.is_empty() {
    out.summary = if has_content(&out) {
            "讨论完成，共识总结提炼失败；各维度成果与分歧已保留，可继续确认。".to_string()
        } else {
            "⚠️ 成果提炼失败：各维度均未产出结构化内容，请检查模型配置后重试。".to_string()
        };
    }
    out
}

/// 判断成果是否已有可落地的结构化内容
fn has_content(s: &DiscussionSynthesis) -> bool {
    !s.locations.is_empty()
        || !s.timeline_events.is_empty()
        || !s.setting_rules.is_empty()
        || !s.characters.is_empty()
        || !s.outline_beats.is_empty()
}

/// 维度提炼结果的进度摘要
fn summarize_dimension(dim: &DimensionDef, r: &DimensionResult) -> String {
    let n = match dim.id {
        "world" => r.locations.len() + r.timeline_events.len(),
        "rules" => r.setting_rules.len(),
        "characters" => r.characters.len(),
        "beats" => r.outline_beats.len(),
        _ => usize::from(!r.summary.is_empty()),
    };
    format!("提炼出 {} 条内容，分歧 {} 项", n, r.disagreements.len())
}

/// 议题匹配：完全相等或一方包含另一方（裁决回填用，宽松匹配）
fn topic_matches(a: &str, b: &str) -> bool {
    let a = a.trim();
    let b = b.trim();
    !a.is_empty() && !b.is_empty() && (a == b || a.contains(b) || b.contains(a))
}

/// 提取标记之间的内容
fn extract_between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let b = text.find(begin)? + begin.len();
    let e = text.rfind(end)?;
    if e <= b {
        return None;
    }
    Some(&text[b..e])
}

/// 从模型输出中提取 JSON 块：优先标记包裹内容，剥围栏，再截最外层花括号
fn extract_block<'a>(text: &'a str, begin: &str, end: &str) -> &'a str {
    let inner = extract_between(text, begin, end).unwrap_or(text);
    let cleaned = inner
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let (Some(b), Some(e)) = (cleaned.find('{'), cleaned.rfind('}'))
        && e > b
    {
        return &cleaned[b..=e];
    }
    cleaned
}

fn extract_dimension_block(text: &str) -> &str {
    extract_block(text, "===DIMENSION_BEGIN===", "===DIMENSION_END===")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dimension_block() {
        let text = "前言\n===DIMENSION_BEGIN===\n{\"summary\": \"x\"}\n===DIMENSION_END===\n后缀";
        assert_eq!(extract_dimension_block(text), "{\"summary\": \"x\"}");
    }

    #[test]
    fn test_extract_block_without_markers() {
        let text = "好的，如下：\n```json\n{\"locations\": []}\n```\n希望对你有帮助。";
        assert_eq!(extract_block(text, "===DIMENSION_BEGIN===", "===DIMENSION_END==="), "{\"locations\": []}");
    }

    #[test]
    fn test_topic_matches() {
        assert!(topic_matches("力量体系上限", "力量体系上限"));
        assert!(topic_matches("关于力量体系上限的讨论", "力量体系上限"));
        assert!(!topic_matches("主角", "配角"));
    }

    #[test]
    fn test_merge_fills_dimensions() {
        let mut world = DimensionResult::default();
        world.locations.push(NamedDesc {
            name: "青云宗".into(),
            description: "山门".into(),
        });
        let summary = DimensionResult::default();
        let out = merge(
            [
                &world,
                &DimensionResult::default(),
                &DimensionResult::default(),
                &DimensionResult::default(),
                &summary,
            ],
            vec![],
        );
        assert_eq!(out.locations.len(), 1);
        assert!(!out.summary.is_empty());
    }
}
