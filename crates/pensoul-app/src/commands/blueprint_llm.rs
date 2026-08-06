//! LLM 账本化转换：把讨论成果熔炼为规范化六账本（承诺一句话、卷名清洗、
//! 副线识别、伏笔锚点类型化），失败时回退确定性映射。

use std::collections::HashMap;

use pensoul_core::{
    BlueprintForeshadow, BookBlueprint, CharacterMatrixEntry, Commitment, DiscussionSynthesis,
    MatrixArcStage, ResourceEntry, Subplot, VolumeBeat, VolumeBlueprint,
};
use serde::Deserialize;
use serde_json::json;

use crate::llm_profile::LlmTask;
use crate::state::AppState;

use super::blueprint::refine_commitments;
use super::discussion::call_with_system_task;
use super::json_fix;
use super::llm_helper as lh;
use super::story_modules::{module_ref_lines, StoryModule};

/// LLM 输出：规范化六账本（全部字段容错，缺省为空）
#[derive(Debug, Default, Deserialize)]
struct LlmBlueprint {
    #[serde(default)]
    commitments: Vec<LlmCommitment>,
    #[serde(default)]
    volumes: Vec<LlmVolume>,
    #[serde(default)]
    character_matrix: Vec<LlmCharacter>,
    #[serde(default)]
    foreshadows: Vec<LlmForeshadow>,
    #[serde(default)]
    subplots: Vec<LlmSubplot>,
    #[serde(default)]
    resources: Vec<LlmResource>,
}

#[derive(Debug, Default, Deserialize)]
struct LlmCommitment {
    #[serde(default)]
    statement: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    ongoing: bool,
}

#[derive(Debug, Default, Deserialize)]
struct LlmVolume {
    #[serde(default)]
    volume_no: u32,
    #[serde(default)]
    title: String,
    #[serde(default)]
    one_line: String,
    #[serde(default)]
    function: String,
    #[serde(default)]
    reader_promise: String,
    #[serde(default)]
    chapter_start: i64,
    #[serde(default)]
    chapter_end: i64,
    #[serde(default)]
    central_conflict: String,
    #[serde(default)]
    climax_scene: String,
    #[serde(default)]
    climax_chapter: Option<i64>,
    #[serde(default)]
    volume_hook: String,
    #[serde(default)]
    beats: Vec<LlmBeat>,
}

#[derive(Debug, Default, Deserialize)]
struct LlmBeat {
    #[serde(default)]
    beat_type: String,
    #[serde(default)]
    chapter: i64,
    #[serde(default)]
    note: String,
    #[serde(default)]
    links: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LlmCharacter {
    #[serde(default)]
    character_name: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    core_values: Vec<String>,
    #[serde(default)]
    speech_style: String,
    #[serde(default)]
    wants: String,
    #[serde(default)]
    fears: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    arc: Vec<LlmArcStage>,
    #[serde(default)]
    knows: Vec<String>,
    #[serde(default)]
    does_not_know: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LlmArcStage {
    #[serde(default)]
    name: String,
    #[serde(default)]
    chapter_range: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    turning_point: String,
}

#[derive(Debug, Default, Deserialize)]
struct LlmForeshadow {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    planted_chapter: i64,
    #[serde(default)]
    payoff_anchor_type: String,
    #[serde(default)]
    payoff_anchor: String,
}

#[derive(Debug, Default, Deserialize)]
struct LlmSubplot {
    #[serde(default)]
    name: String,
    #[allow(dead_code)] // 子结构无对应字段，保留供模型输出（信息已在其他字段体现）
    #[serde(default)]
    description: String,
    #[serde(default)]
    mainline_relation: String,
    #[serde(default)]
    chapter_range: String,
    #[serde(default)]
    open_threads: Vec<String>,
    #[serde(default)]
    characters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LlmResource {
    #[serde(default)]
    name: String,
    #[serde(default)]
    rtype: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    constraints: Vec<String>,
}

/// 调用 LLM 做账本化转换；失败时返回错误（调用方回退确定性映射）
pub(crate) async fn llm_convert_blueprint(
    state: &AppState,
    syn: &DiscussionSynthesis,
    fallback: &BookBlueprint,
    reference_modules: &[StoryModule],
) -> Result<BookBlueprint, String> {
    let model = pick_blueprint_model(state)?;
    let saved_providers = lh::load_providers(state);
    let saved_models = lh::load_models(state);
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };

    let system = "你是资深小说总设计师，负责把多 Agent 讨论成果熔炼为「开书定盘蓝图」。\
        你必须严格按用户给定的 JSON 结构输出，不要输出任何解释、标记或额外文本。";
    let module_block = if reference_modules.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n         【作者勾选的参考模块（灵感输入，禁止照搬案例；至少吸收一个手法，\
             但必须适配本书设定）】\n         {}",
            module_ref_lines(reference_modules)
        )
    };
    let user = format!(
        "【讨论成果（输入）】\n{}\n\n\
         【熔炼要求】\n\
         1. commitments：每条一句话（≤60字），宁少勿滥，重复主题合并；\
         kind 取值 theme/promise/tone/rule/no_go；scope 默认 book\n\
         2. volumes：规范分卷，第一卷到第N卷连续；跨卷节点归入其起始卷；\
         「各卷」类节点不建卷；title 只写卷名（如 风起青云，不带「第X卷」前缀）；\
         volume_no 从 1 连续；chapter_start/end 用章号；每卷必须给出 climax_scene；\
         没有明显高潮的卷 climax_scene 写「待卷定盘补充」；\
         beats 给 3-6 个节奏点（hook/buildup/payoff/fall/climax/hook_end），\
         覆盖本卷开头钩子、蓄力、爽点、回落与高潮，chapter 用章号，note 一句话\n\
         3. character_matrix：只保留有名字的具体个体，群像/派系/同盟剔除；\
         role 取值 protagonist/mentor/antagonist/ally/love_interest/minor\n\
         4. foreshadows：每条给出 payoff_anchor_type（chapter/volume/event）与 \
         payoff_anchor（如 第2卷 / 身份揭破时）；没有锚点也要给出事件型锚点，禁止留空\n\
         5. subplots：把贯穿多卷的独立叙事线提炼为副线（关系线/势力线/探索线等），\
         不要与单节点混淆\n\
         6. resources：登记金手指/关键道具/特殊能力/重要信息等资源；没有就空数组\n\
         7. 所有内容必须来自讨论成果，禁止凭空添加；全部用中文\n\
         {module_block}\n\
        用 ===BLUEPRINT_BEGIN=== 与 ===BLUEPRINT_END=== 包裹纯 JSON，结构如下：\n{}\n\
        若某字段无内容，用空数组。",
        compact_synthesis(syn),
        LLM_SCHEMA,
    );

    let mut last_err = String::new();
    for _attempt in 1..=2u8 {
        let text = match call_with_system_task(
            &model,
            system,
            &user,
            0.3,
            8192,
            LlmTask::Light,
            &model_to_provider,
            &provider_api_bases,
            &api_keys,
        )
        .await
        {
            Ok(t) => t,
            Err(msg) => {
                last_err = msg;
                continue;
            }
        };
        let json_str = extract_json_block(&text);
        let parsed = serde_json::from_str::<LlmBlueprint>(json_str).or_else(|strict_err| {
            last_err = strict_err.to_string();
            json_fix::repair_to_value(json_str)
                .and_then(|v| serde_json::from_value::<LlmBlueprint>(v).map_err(|e| e.to_string()))
        });
        if let Ok(lb) = parsed {
            return Ok(merge_llm(&lb, fallback));
        }
    }
    Err(format!("LLM 账本化转换失败：{last_err}"))
}

const LLM_SCHEMA: &str = r#"{
  "commitments": [{"statement": "一句话承诺（≤60字）", "kind": "theme|promise|tone|rule|no_go", "scope": "book", "ongoing": true}],
  "volumes": [{"volume_no": 1, "title": "卷名（不带第X卷前缀）", "one_line": "一句话", "function": "setup|escalation|climax|resolution", "reader_promise": "本卷读者获得什么", "chapter_start": 1, "chapter_end": 100, "central_conflict": "", "climax_scene": "高潮场景", "climax_chapter": 45, "volume_hook": "卷末钩子", "beats": [{"beat_type": "hook|buildup|payoff|fall|climax|hook_end", "chapter": 3, "note": "一句话", "links": []}]}],
  "character_matrix": [{"character_name": "名字", "role": "protagonist|mentor|antagonist|ally|love_interest|minor", "core_values": [], "speech_style": "", "wants": "核心欲望", "fears": "核心恐惧", "secret": "", "arc": [{"name": "阶段名", "chapter_range": "第1-100章", "goal": "", "turning_point": ""}], "knows": [], "does_not_know": []}],
  "foreshadows": [{"name": "伏笔名", "description": "", "kind": "object|line|secret|ability|event|relationship", "planted_chapter": 3, "payoff_anchor_type": "chapter|volume|event", "payoff_anchor": "第2卷"}],
  "subplots": [{"name": "副线名", "description": "", "mainline_relation": "", "chapter_range": "第31-300章", "open_threads": [], "characters": []}],
  "resources": [{"name": "资源名", "rtype": "item|ability|info|relationship|faction|asset", "owner": "", "note": "", "constraints": []}]
}"#;

/// 把讨论成果压缩为输入（只保留熔炼所需字段，省 token）
fn compact_synthesis(syn: &DiscussionSynthesis) -> String {
    let slim = json!({
        "summary": syn.summary,
        "characters": syn.characters.iter().map(|c| json!({
            "name": c.name, "entity_kind": c.entity_kind, "wants": c.wants, "fears": c.fears,
            "secret": c.secret, "speech_style": c.speech_style, "arc": c.arc,
            "knows": c.knows, "does_not_know": c.does_not_know
        })).collect::<Vec<_>>(),
        "outline_beats": syn.outline_beats.iter().map(|b| json!({
            "title": b.title, "description": b.description, "chapter_hint": b.chapter_hint,
            "volume": b.volume, "beat_type": b.beat_type, "hook": b.hook, "payoff": b.payoff,
            "foreshadowing": b.foreshadowing
        })).collect::<Vec<_>>(),
        "setting_rules": syn.setting_rules.iter().map(|r| json!({
            "name": r.name, "description": r.description
        })).collect::<Vec<_>>(),
        "subplots": syn.subplots,
        "commitments": syn.commitments,
        "resolved_disagreements": syn.disagreements.iter()
            .filter(|d| d.status == "resolved" || d.adjudicated)
            .map(|d| json!({"topic": d.topic, "resolution": d.resolution}))
            .collect::<Vec<_>>()
    });
    serde_json::to_string(&slim).unwrap_or_else(|_| syn.summary.clone())
}

/// 提取 JSON 块：优先 BLUEPRINT 标记，其次代码围栏，最后首尾大括号
fn extract_json_block(text: &str) -> &str {
    const BEGIN: &str = "===BLUEPRINT_BEGIN===";
    const END: &str = "===BLUEPRINT_END===";
    if let (Some(s), Some(e)) = (text.find(BEGIN), text.find(END)) {
        let inner = &text[s + BEGIN.len()..e];
        if let Some(bs) = inner.find('{') {
            return inner[bs..].trim_end();
        }
    }
    if text.contains("```json") {
        let s = text.find("```json").unwrap() + "```json".len();
        let e = text[s..]
            .find("```")
            .map(|i| s + i)
            .unwrap_or(text.len());
        return text[s..e].trim();
    }
    if let (Some(s), Some(e)) = (text.find('{'), text.rfind('}'))
        && e > s
    {
        return &text[s..=e];
    }
    text
}

/// 合并：LLM 输出的账本替换确定性结果；空字段保留确定性结果
fn merge_llm(lb: &LlmBlueprint, fallback: &BookBlueprint) -> BookBlueprint {
    let mut bp = fallback.clone();
    if !lb.commitments.is_empty() {
        bp.commitments = refine_commitments(
            lb.commitments
                .iter()
                .enumerate()
                .map(|(i, c)| Commitment {
                    commitment_id: format!("cmt-{:03}", i + 1),
                    statement: c.statement.clone(),
                    kind: if c.kind.is_empty() { "rule" } else { &c.kind }.to_string(),
                    priority: 2,
                    scope: if c.scope.is_empty() { "book" } else { &c.scope }.to_string(),
                    resolution_chapter: None,
                    ongoing: c.ongoing,
                    status: "active".to_string(),
                    sources: Vec::new(),
                })
                .collect(),
        );
    }
    if !lb.volumes.is_empty() {
        bp.volumes = lb
            .volumes
            .iter()
            .map(|v| VolumeBlueprint {
                volume_no: v.volume_no,
                title: v.title.clone(),
                one_line: v.one_line.clone(),
                function: v.function.clone(),
                reader_promise: v.reader_promise.clone(),
                chapter_start: v.chapter_start,
                chapter_end: v.chapter_end,
                central_conflict: v.central_conflict.clone(),
                climax_scene: v.climax_scene.clone(),
                climax_chapter: v.climax_chapter,
                volume_hook: v.volume_hook.clone(),
                beats: v
                    .beats
                    .iter()
                    .enumerate()
                    .map(|(i, b)| VolumeBeat {
                        beat_id: format!("bt-{:02}", i + 1),
                        beat_type: b.beat_type.clone(),
                        chapter: b.chapter,
                        note: b.note.clone(),
                        links: b.links.clone(),
                    })
                    .collect(),
                status: "planned".to_string(),
                ..Default::default()
            })
            .collect();
    }
    if !lb.character_matrix.is_empty() {
        bp.character_matrix = lb
            .character_matrix
            .iter()
            .map(|c| CharacterMatrixEntry {
                character_name: c.character_name.clone(),
                role: c.role.clone(),
                core_values: c.core_values.clone(),
                speech_style: c.speech_style.clone(),
                wants: c.wants.clone(),
                fears: c.fears.clone(),
                secret: c.secret.clone(),
                arc: c
                    .arc
                    .iter()
                    .map(|a| MatrixArcStage {
                        name: a.name.clone(),
                        chapter_range: a.chapter_range.clone(),
                        goal: a.goal.clone(),
                        turning_point: a.turning_point.clone(),
                    })
                    .collect(),
                knows: c.knows.clone(),
                does_not_know: c.does_not_know.clone(),
                ..Default::default()
            })
            .collect();
    }
    if !lb.foreshadows.is_empty() {
        bp.foreshadows = lb
            .foreshadows
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let anchor_type = f.payoff_anchor_type.clone();
                let expected = if anchor_type == "chapter" {
                    parse_chapter_no(&f.payoff_anchor).unwrap_or(0)
                } else {
                    0
                };
                BlueprintForeshadow {
                    foreshadow_id: format!("fs-{:03}", i + 1),
                    name: f.name.clone(),
                    description: f.description.clone(),
                    kind: f.kind.clone(),
                    planted_chapter: f.planted_chapter,
                    expected_payoff_chapter: expected,
                    payoff_anchor_type: anchor_type,
                    payoff_anchor: f.payoff_anchor.clone(),
                    status: "planned".to_string(),
                    ..Default::default()
                }
            })
            .collect();
    }
    if !lb.subplots.is_empty() {
        bp.subplots = lb
            .subplots
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (start, end) = parse_chapter_range(&s.chapter_range)
                    .map(|(a, b)| (a, Some(b)))
                    .unwrap_or((0, None));
                Subplot {
                    subplot_id: format!("sp-{:03}", i + 1),
                    name: s.name.clone(),
                    line_tags: vec![s.name.clone()],
                    mainline_relation: s.mainline_relation.clone(),
                    status: "active".to_string(),
                    start_chapter: start,
                    end_chapter: end,
                    characters: s.characters.clone(),
                    last_touched_chapter: start,
                    touch_interval_limit: 3,
                    open_threads: s.open_threads.clone(),
                    sources: Vec::new(),
                }
            })
            .collect();
    }
    if !lb.resources.is_empty() {
        bp.resources = lb
            .resources
            .iter()
            .enumerate()
            .map(|(i, r)| ResourceEntry {
                resource_id: format!("res-{:03}", i + 1),
                name: r.name.clone(),
                rtype: r.rtype.clone(),
                owner: r.owner.clone(),
                status: "available".to_string(),
                constraints: r.constraints.clone(),
                note: r.note.clone(),
                ..Default::default()
            })
            .collect();
    }
    bp
}

fn parse_chapter_no(text: &str) -> Option<i64> {
    let mut num = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else if !num.is_empty() {
            break;
        }
    }
    if num.is_empty() {
        None
    } else {
        num.parse().ok()
    }
}

fn parse_chapter_range(hint: &str) -> Option<(i64, i64)> {
    let s = hint.trim();
    let s = match s.strip_prefix("第") {
        Some(rest) => rest.trim(),
        None => s,
    };
    let s = s.split('章').next()?.trim();
    if s.is_empty() {
        return None;
    }
    for sep in ["-", "–", "—", "~", "至", "到"] {
        if let Some(pos) = s.find(sep) {
            let a: i64 = s[..pos].trim().parse().ok()?;
            let b: i64 = s[pos + sep.len()..].trim().parse().ok()?;
            return Some((a, b));
        }
    }
    let n: i64 = s.parse().ok()?;
    Some((n, n))
}

/// 选转换模型：优先讨论 Agent 第一个启用模型，其次默认模型
pub(crate) fn pick_blueprint_model(state: &AppState) -> Result<String, String> {
    let from_sprout = state
        .ontology
        .read()
        .sprout
        .agents
        .iter()
        .find(|a| a.enabled)
        .map(|a| a.model.clone());
    if let Some(m) = from_sprout {
        return Ok(m);
    }
    let models = lh::load_models(state);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };
    lh::pick_default_model(&models, &api_keys)
        .ok_or_else(|| "没有可用的模型，请在模型设置中配置".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block_markers() {
        let text = "好的\n===BLUEPRINT_BEGIN===\n{\"commitments\": []}\n===BLUEPRINT_END===\n完毕";
        assert_eq!(extract_json_block(text), "{\"commitments\": []}");
    }

    #[test]
    fn test_extract_json_block_fence() {
        let text = "结果如下：\n```json\n{\"volumes\": []}\n```\n以上。";
        assert_eq!(extract_json_block(text), "{\"volumes\": []}");
    }

    #[test]
    fn test_parse_chapter_range_variants() {
        assert_eq!(parse_chapter_range("第1-100章"), Some((1, 100)));
        assert_eq!(parse_chapter_range("第45章"), Some((45, 45)));
        assert_eq!(parse_chapter_range("无"), None);
    }
}
