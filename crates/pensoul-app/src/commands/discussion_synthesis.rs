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

use super::discussion::{call_with_system_task, emit_discussion, AgentConfig};
use super::json_fix;

/// 单块讨论记录字符上限（保护输入上下文；超过则分块抽取）
const CHUNK_CHARS: usize = 12_000;
/// 触发分块模式的讨论总长度阈值（低于它仍走单遍提炼，省成本）
const CHUNK_TRIGGER_CHARS: usize = 14_000;
/// 单块抽取的输出预算（小产出，防推理型模型截断）
const CHUNK_OUTPUT_TOKENS: u32 = 4_096;
/// 分块汇总定稿的输出预算
const FINAL_OUTPUT_TOKENS: u32 = 8_192;
/// 短讨论单遍提炼的输出预算
const SINGLE_OUTPUT_TOKENS: u32 = 16_384;

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
    schema: r#"{"locations": [{"name": "地点名", "description": "100-200字的完整描述：外观、氛围、功能、与故事的关系", "level": "层级（如 L1 区域/L2 城市，可空）", "region": "所属区域", "faction": "控制该地的势力（可空）", "unlocked_chapter": "首次登场章节（可空）", "sources": ["来源：评审者名·第N轮"]}], "timeline_events": [{"story_time": "故事时间", "description": "80-150字：事件经过及其对后续的影响", "participants": ["参与人物"], "sources": ["来源：评审者名·第N轮"]}], "quality_notes": ["共识复核与质量提示"], "disagreements": [分歧列表]}"#,
};

const DIM_RULES: DimensionDef = DimensionDef {
    id: "rules",
    name: "设定规则提炼",
    focus: "设定规则：世界运行的法则、力量体系、社会/修行/文明规则，以及规则带来的约束、代价与可被利用的漏洞",
    schema: r#"{"setting_rules": [{"name": "设定规则标题", "description": "100-200字：规则内容、约束、代价、可被利用的漏洞", "constraints": ["约束条件"], "cost": "使用代价", "loophole": "可被利用的漏洞", "sources": ["来源：评审者名·第N轮"]}], "quality_notes": ["共识复核与质量提示"], "disagreements": [分歧列表]}"#,
};

const DIM_CHARACTERS: DimensionDef = DimensionDef {
    id: "characters",
    name: "人物与关系提炼",
    focus: "人物及人物关系：主角、关键配角、重要反派的身份、欲望、恐惧、故事功能，以及人物之间的关系",
    schema: r#"{"characters": [{"name": "人物名", "personality_traits": [["特质", 0.8]], "current_mood": "登场时的心境", "description": "100-200字：身份、欲望、恐惧、在故事中的功能", "relationships": [{"from": "人物名", "to": "人物名", "relation_type": "关系类型", "strength": 0.7}], "wants": "核心欲望", "fears": "核心恐惧", "secret": "暂不揭示的秘密（可空）", "speech_style": "说话方式（口癖/语气/信息量）", "arc": [{"name": "阶段名", "chapter_range": "章节范围", "trait_desc": "阶段特征", "goal": "阶段目标"}], "knows": ["知情边界：知道什么"], "does_not_know": ["知情边界：不知道什么"], "sources": ["来源：评审者名·第N轮"]}], "quality_notes": ["共识复核与质量提示"], "disagreements": [分歧列表]}"#,
};

const DIM_BEATS: DimensionDef = DimensionDef {
    id: "beats",
    name: "情节脉络提炼",
    focus: "情节脉络：开端、发展、转折、高潮、结局的关键节点，以及它们如何推进主线",
    schema: r#"{"outline_beats": [{"title": "情节节点标题", "description": "100-200字：该节点发生什么、核心冲突是什么、如何推进主线", "chapter_hint": "建议章节范围，如 第1-3章", "volume": "所属卷（如 第一卷·风起青云；未分卷留空）", "beat_type": "节拍类型（铺垫/转折/高潮/爽点/收束）", "hook": "章尾钩子（可空）", "payoff": "爽点/情绪释放点（可空）", "emotion_arc": "情绪曲线（如：压抑→紧张→爆发→余波）", "line_tags": ["主线/副线/交织"], "foreshadowing": [{"plant": "埋设内容", "payoff_hint": "预期回收章节/方式"}], "sources": ["来源：评审者名·第N轮"]}], "quality_notes": ["共识复核与质量提示"], "disagreements": [分歧列表]}"#,
};

const DIM_SUMMARY: DimensionDef = DimensionDef {
    id: "summary",
    name: "共识与分歧提炼",
    focus: "共识总结、核心分歧与给作者的总体建议（这是给作者的总览，必须具体、引用讨论中的关键观点）",
    schema: r#"{"summary": "300-500字", "quality_notes": ["共识复核与质量提示"], "disagreements": [分歧列表]}"#,
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
    /// 块级摘要（仅分块抽取阶段使用；单遍/定稿阶段可为空）
    #[serde(default)]
    digest: String,
    /// 共识复核与质量提示（共识过于平庸/缺代价、候选溢出等，供作者参考）
    #[serde(default)]
    quality_notes: Vec<String>,
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
    /// 备选路径（供作者拍板，2-3 条）
    #[serde(default)]
    alternatives: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AdjudicationResult {
    #[serde(default)]
    verdicts: Vec<Verdict>,
}

/// 单维度提炼的产出：最终结构化结果 + 轨迹摘要（供冲突检查/裁判做证据）
struct DimensionOutcome {
    result: DimensionResult,
    digest: String,
}

/// 讨论成果提炼主流程：
/// 1. 按轨迹长度自动选择「单遍提炼」或「分块抽取 → 综合定稿」（map-reduce，
///    同时解决长讨论输入溢出与输出预算截断，且保留轨迹级信息）；
/// 2. 跨维度冲突检查（显式标记矛盾，不静默抹平）；
/// 3. 独立裁判裁决未收敛分歧（含共识复核标记的 open 项）。
pub async fn synthesize(ctx: &SynthesisContext<'_>) -> DiscussionSynthesis {
    let Some(caller) = ctx.enabled.first() else {
        return DiscussionSynthesis::default();
    };
    let all_turns = render_turns(ctx.turns);
    let chunks = chunk_turns(ctx.turns, CHUNK_CHARS);
    // 长讨论走分块 map-reduce；短讨论单遍提炼省成本
    let long = chunks.len() > 1 || all_turns.chars().count() > CHUNK_TRIGGER_CHARS;
    let chunks: Vec<String> = if long {
        chunks
    } else {
        vec![all_turns.clone()]
    };

    // ── 1. 五路并行分维度提炼（内部再按块抽取/综合）──
    let (world, rules, chars, beats, summary) = tokio::join!(
        extract_dimension(ctx, &chunks, caller, &DIM_WORLD),
        extract_dimension(ctx, &chunks, caller, &DIM_RULES),
        extract_dimension(ctx, &chunks, caller, &DIM_CHARACTERS),
        extract_dimension(ctx, &chunks, caller, &DIM_BEATS),
        extract_dimension(ctx, &chunks, caller, &DIM_SUMMARY),
    );

    let dims = [
        &world.result,
        &rules.result,
        &chars.result,
        &beats.result,
        &summary.result,
    ];

    // 证据：短讨论用完整轨迹；长讨论用各维度分块摘要，控制冲突检查/裁判的输入规模
    let evidence = if long {
        [&world, &rules, &chars, &beats, &summary]
            .iter()
            .map(|d| d.digest.clone())
            .filter(|d| !d.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    } else {
        all_turns.clone()
    };

    // ── 2. 跨维度冲突检查（显式标记矛盾，不静默抹平）──
    let mut disagreements: Vec<Disagreement> = dims
        .iter()
        .flat_map(|d| d.disagreements.clone())
        .collect();
    let conflicts = check_conflicts(ctx, &evidence, &dims, caller).await;
    disagreements.extend(conflicts);

    // ── 3. 裁判裁决未收敛分歧（独立于提炼者的模型）──
    let open: Vec<Disagreement> = disagreements
        .iter()
        .filter(|d| d.status == "open")
        .cloned()
        .collect();
    if !open.is_empty() {
        let adjudicator = ctx.enabled.get(1).copied().unwrap_or(caller);
        let verdicts = adjudicate(ctx, &evidence, &open, adjudicator).await;
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
                if !v.alternatives.is_empty() {
                    d.alternatives = v.alternatives.clone();
                }
            }
        }
    }

    merge(dims, disagreements)
}

/// 渲染单条发言（标明评审者与轮次）
fn render_turn(t: &AgentTurn) -> String {
    format!("【{} · 第{}轮】：\n{}", t.agent_name, t.round, t.content)
}

/// 把全部发言渲染成文本
fn render_turns(turns: &[AgentTurn]) -> String {
    turns
        .iter()
        .map(render_turn)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// 按轮次分块渲染：每块不超过 max_chars（单条超长发言允许略超）。
/// 任何一块都小于上下文窗口，是「分块抽取 → 综合定稿」的输入侧保障。
fn chunk_turns(turns: &[AgentTurn], max_chars: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for t in turns {
        let block = render_turn(t);
        if cur.chars().count() + block.chars().count() > max_chars && !cur.is_empty() {
            chunks.push(std::mem::take(&mut cur));
        }
        cur.push_str(&block);
        cur.push_str("\n\n---\n\n");
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

/// 单维度提炼：短讨论单遍；长讨论先逐块抽取候选（保留来源与块摘要），
/// 再综合定稿为最终条目。
async fn extract_dimension(
    ctx: &SynthesisContext<'_>,
    chunks: &[String],
    caller: &AgentConfig,
    dim: &DimensionDef,
) -> DimensionOutcome {
    let agent_id = format!("dim-{}", dim.id);
    if chunks.len() == 1 {
        let result = extract_once(ctx, &chunks[0], caller, dim, false).await;
        let digest = if result.digest.trim().is_empty() {
            head_chars(&chunks[0], 600)
        } else {
            result.digest.clone()
        };
        return DimensionOutcome { result, digest };
    }

    // ── 长讨论：分块抽取（逐块串行，避免并发打爆限流）──
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        &agent_id,
        dim.name,
        3,
        "running",
        &format!("讨论较长，分 {} 块抽取中…", chunks.len()),
    );
    let mut chunk_results = Vec::with_capacity(chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            &agent_id,
            dim.name,
            3,
            "running",
            &format!("分块 {}/{} 抽取中…", i + 1, chunks.len()),
        );
        chunk_results.push(extract_once(ctx, chunk, caller, dim, true).await);
    }
    let digest = chunk_results
        .iter()
        .map(|r| r.digest.clone())
        .filter(|d| !d.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    // 兜底：块摘要为空时取前两块的开头，保证冲突检查/裁判仍有证据可用
    let digest = if digest.trim().is_empty() {
        chunks
            .iter()
            .take(2)
            .map(|c| head_chars(c, 800))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    } else {
        digest
    };
    let result = integrate_chunks(ctx, &chunk_results, caller, dim).await;
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        &agent_id,
        dim.name,
        3,
        "done",
        &summarize_dimension(dim, &result),
    );
    DimensionOutcome { result, digest }
}

/// 单块/单遍抽取：一次调用输出该块的结构化候选条目 + 分歧 + 块摘要。
/// chunk_mode=true 时输出主张级短条目（30-80 字）供后续综合；
/// 否则直接输出最终条目（100-200 字）。
async fn extract_once(
    ctx: &SynthesisContext<'_>,
    chunk_text: &str,
    caller: &AgentConfig,
    dim: &DimensionDef,
    chunk_mode: bool,
) -> DimensionResult {
    let agent_id = format!("dim-{}", dim.id);
    if !chunk_mode {
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            &agent_id,
            dim.name,
            3,
            "running",
            "",
        );
    }
    let system = "你是创作讨论的分维度提炼者。你的任务是从多位评审者的讨论中，\
        只提炼出自己负责的维度，输出严格 JSON。不评论、不解释，只输出 JSON。";
    let (task_desc, caps, schema) = if chunk_mode {
        (
            "从本块讨论中抽取与「{}」相关的候选条目（主张级要点即可，每条 30-80 字；\
             完整描述由定稿阶段补全）",
            "地点≤4、时间线事件≤4、规则≤4、人物≤6、情节节点≤6、分歧≤4",
            format!(
                "{}（另加 digest 字段：\"本块讨论要点摘要，100-150字\"）",
                dim.schema
            ),
        )
    } else {
        (
            "提炼「{}」的最终条目（每条 100-200 字，可直接交给作者使用的设定文字，\
             不要写成「讨论认为」「建议」式转述）",
            "地点≤8、时间线事件≤8、规则≤8、人物≤10、情节节点≤12、分歧≤8",
            dim.schema.to_string(),
        )
    };
    let user_prompt = format!(
        "【故事构思】\n{}\n\n\
         【创作设定】\n{}\n\n\
         【讨论记录（本块）】\n{}\n\n\
         【本轮提炼任务】你只负责「{}」。\n\
         {}\n\
         要求：\n\
         1. 只提取与「{}」相关的内容；无关内容一律忽略\n\
         2. 只保留与主线/核心冲突直接相关的条目，宁可精炼不要堆砌；\
         每条都必须能直接支撑后续写作或审稿\n\
         3. 条目必须能从讨论记录中找到依据，禁止凭空添加讨论中未出现的内容\n\
         4. 每条必须填 sources（主要来源：评审者名·第N轮，可多条；多数共识可写「多位共识」）\n\
         5. 条目数量上限：{}；超出时按与主线的相关度取舍，被舍弃的候选名写入 quality_notes\n\
         6. 同时输出 disagreements：本维度内评审者之间真实存在的分歧，每条含 topic（议题）、\
         dimension（当前维度名）、sides（各方：[{{\"agent\": 评审者名, \"position\": 立场, \
         \"rationale\": 依据}}]）、status（讨论中已收敛填 resolved 并写明 resolution；未收敛填 open，\
         resolution 留空）、resolution；没有分歧则 disagreements 为空数组\n\
         7. quality_notes：对已收敛议题做共识复核——共识是否过于平庸、缺少代价/风险/冲突？\
         若是，把该议题改标 open 交由裁判复核，并在这里写明原因；同时记录被舍弃的候选名\n\
         8. 全部内容用中文\n\
         用 ===DIMENSION_BEGIN=== 与 ===DIMENSION_END=== 包裹纯 JSON，结构如下：\n{}\n\
         若某字段本轮不产出，用空数组/空字符串，但不要遗漏讨论中实际提到的相关内容。",
        ctx.idea_description,
        ctx.settings_context,
        chunk_text,
        dim.name,
        task_desc.replace("{}", dim.focus),
        dim.focus,
        caps,
        schema,
    );

    let mut next_prompt = user_prompt.clone();
    let mut last_err = String::new();
    let max_tokens = if chunk_mode {
        CHUNK_OUTPUT_TOKENS
    } else {
        SINGLE_OUTPUT_TOKENS
    };
    for attempt in 1..=2u8 {
        let text = match call_with_system_task(
            &caller.model,
            system,
            &next_prompt,
            0.3,
            max_tokens,
            crate::llm_profile::LlmTask::Light,
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
                if !chunk_mode {
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
                }
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
    if !chunk_mode {
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            &agent_id,
            dim.name,
            3,
            "error",
            &format!("提炼失败: {last_err}"),
        );
    }
    DimensionResult::default()
}

/// 综合定稿：读取各块候选，去重合并、补全为最终条目，保留出处，
/// 识别跨块矛盾，并对已收敛议题做共识复核。
async fn integrate_chunks(
    ctx: &SynthesisContext<'_>,
    chunk_results: &[DimensionResult],
    caller: &AgentConfig,
    dim: &DimensionDef,
) -> DimensionResult {
    let rendered: String = chunk_results
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    let system = "你是创作讨论分块提炼的综合定稿者。你的任务是把各块候选合并成该维度的最终成果，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "【故事构思】\n{}\n\n\
         【创作设定】\n{}\n\n\
         【各块候选提炼结果】\n{}\n\n\
         【综合任务】你负责「{}」。\n\
         要求：\n\
         1. 去重与合并：同名/近义条目合并，保留更完整的信息，不得遗漏各块提出的核心主张\n\
         2. 每条补全为可直接交给作者使用的设定文字（100-200 字），必须能从候选与讨论中找到依据\n\
         3. 每条 sources 合并各块的来源（评审者名·第N轮）\n\
         4. 跨块矛盾写入 disagreements（status=open），并说明矛盾双方内容与依据\n\
         5. 对已收敛议题做共识复核：共识是否过于平庸、缺少代价/风险/冲突？若是，改标 open \
         交由裁判复核，并写入 quality_notes\n\
         6. 条目数量上限：地点≤8、时间线事件≤8、规则≤8、人物≤10、情节节点≤12、分歧≤8；\
         超出时按与主线的相关度取舍，被舍弃的候选名写入 quality_notes\n\
         7. 全部内容用中文\n\
         用 ===DIMENSION_BEGIN=== 与 ===DIMENSION_END=== 包裹纯 JSON，结构如下：\n{}\n\
         若某字段不产出，用空数组/空字符串。",
        ctx.idea_description,
        ctx.settings_context,
        rendered,
        dim.name,
        dim.schema,
    );

    let mut next_prompt = user_prompt.clone();
    let mut last_err = String::new();
    for attempt in 1..=2u8 {
        let text = match call_with_system_task(
            &caller.model,
            system,
            &next_prompt,
            0.3,
            FINAL_OUTPUT_TOKENS,
            crate::llm_profile::LlmTask::Light,
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
            Ok(result) => return result,
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
        &format!("dim-{}", dim.id),
        dim.name,
        3,
        "error",
        &format!("综合定稿失败: {last_err}"),
    );
    DimensionResult::default()
}

/// 取文本开头 N 字符（按字符计数，不切坏 UTF-8）
fn head_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// 跨维度冲突检查：找出人物/规则/时间线/情节之间的真实矛盾，显式标记
async fn check_conflicts(
    ctx: &SynthesisContext<'_>,
    evidence: &str,
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
        "【讨论证据（完整轨迹或分块摘要，供核对依据）】\n{evidence}\n\n\
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
        let text = match call_with_system_task(
            &caller.model,
            system,
            &next_prompt,
            0.2,
            4096,
            crate::llm_profile::LlmTask::Light,
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
    evidence: &str,
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

    let system = "你是创作设定的独立裁判。你负责对讨论中未收敛的分歧做出最终裁决，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    // 逐条裁决：每条分歧独立调用（单条输出短，不会因预算截断而整批失败），
    // 讨论证据（完整轨迹或分块摘要）每次都带上，供裁判核对各方依据。
    let mut verdicts: Vec<(String, Verdict)> = Vec::new();
    for (i, d) in open.iter().enumerate() {
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            "adjudicator",
            "分歧裁决",
            3,
            "running",
            &format!("裁决中（{}/{}）：{}", i + 1, open.len(), d.topic),
        );
        let one = serde_json::json!({
            "topic": d.topic,
            "dimension": d.dimension,
            "sides": d.sides,
        });
        let user_prompt = format!(
            "【故事构思】\n{}\n\n\
             【创作设定】\n{}\n\n\
             【讨论证据（完整轨迹或分块摘要，供核对依据）】\n{}\n\n\
             【待裁决分歧（仅此一条）】\n{}\n\n\
             对这条分歧给出裁决，输出：\n\
             {{\"verdicts\": [{{\"topic\": \"议题（必须与输入完全一致）\", \
             \"verdict\": \"采纳哪一方或折中方案（一句话）\", \
             \"rationale\": \"裁决理由（引用讨论中的具体依据）\", \
             \"final_text\": \"可直接采用的最终设定文字（100-200字，供作者直接使用）\", \
             \"alternatives\": [\"备选路径1（一句话+代价）\", \"备选路径2（一句话+代价）\"]}}]}}\n\
             要求：\n\
             1. 裁决必须有利于故事的整体一致性，不能只偏向一方\n\
             2. final_text 要写成可直接落地的设定文字，不要「建议」「可以」式措辞\n\
             3. alternatives 给出 2-3 条与裁决不同的可行路径，每条一句话并注明取舍代价，\
             供作者拍板\n\
             4. 只输出这一条分歧的裁决，不要输出其他内容\n\
             5. 全部内容用中文\n\
             用 ===VERDICT_BEGIN=== 与 ===VERDICT_END=== 包裹纯 JSON。",
            ctx.idea_description, ctx.settings_context, evidence, one
        );

        let mut next_prompt = user_prompt.clone();
        let mut last_err = String::new();
        for attempt in 1..=2u8 {
            let text = match call_with_system_task(
                &adjudicator.model,
                system,
                &next_prompt,
                0.2,
                8192,
                crate::llm_profile::LlmTask::Light,
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
                    if let Some(v) = r.verdicts.into_iter().next() {
                        verdicts.push((v.topic.clone(), v));
                    }
                    break;
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
        if last_err.is_empty() {
            continue;
        }
        // 单条裁决失败：记录错误事件但继续裁决其余分歧，避免一条失败拖垮整批
        emit_discussion(
            ctx.app_handle,
            ctx.state,
            "adjudicator",
            "分歧裁决",
            3,
            "error",
            &format!("第 {}/{} 条裁决失败: {last_err}", i + 1, open.len()),
        );
    }
    emit_discussion(
        ctx.app_handle,
        ctx.state,
        "adjudicator",
        "分歧裁决",
        3,
        "done",
        &format!("完成 {} 项裁决", verdicts.len()),
    );
    verdicts
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
        quality_notes: Vec::new(),
    };
    // 汇总各维度的共识复核与质量提示（去重）
    let mut notes: Vec<String> = dims
        .iter()
        .flat_map(|d| d.quality_notes.iter().cloned())
        .collect();
    notes.sort();
    notes.dedup();
    out.quality_notes = notes;
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
            ..Default::default()
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

    #[test]
    fn test_chunk_turns_splits_long_trace() {
        let turns: Vec<AgentTurn> = (0..6)
            .map(|i| AgentTurn {
                agent_id: format!("a{i}"),
                agent_name: format!("评审者{i}"),
                perspective: String::new(),
                round: 1 + i % 2,
                content: "这是一段用于测试分块的长发言内容。".repeat(40),
            })
            .collect();
        let chunks = chunk_turns(&turns, 500);
        // 6 条长发言必然被拆成多块
        assert!(chunks.len() >= 2, "应拆成多块，实际 {}", chunks.len());
        // 每块不超过上限 + 单条发言上限（避免拆坏单条）
        let max_single = turns.iter().map(|t| render_turn(t).chars().count()).max().unwrap_or(0);
        for c in &chunks {
            assert!(
                c.chars().count() <= 500 + max_single,
                "块超长: {}",
                c.chars().count()
            );
        }
        // 全部评审者名都出现在分块中（内容未丢失）
        let joined = chunks.join("");
        for t in &turns {
            assert!(joined.contains(&t.agent_name));
        }
    }

    #[test]
    fn test_merge_quality_notes_dedup() {
        let mut world = DimensionResult::default();
        world.quality_notes.push("共识过于保守：力量体系缺少代价".into());
        let mut rules = DimensionResult::default();
        rules.quality_notes.push("共识过于保守：力量体系缺少代价".into());
        rules.quality_notes.push("候选溢出：雾谷、荒原（未入选）".into());
        let summary = DimensionResult::default();
        let out = merge(
            [
                &world,
                &rules,
                &DimensionResult::default(),
                &DimensionResult::default(),
                &summary,
            ],
            vec![],
        );
        assert_eq!(out.quality_notes.len(), 2);
        assert!(out.quality_notes.iter().any(|n| n.contains("代价")));
        assert!(out.quality_notes.iter().any(|n| n.contains("雾谷")));
    }
}
