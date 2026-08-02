//! 情节脉络（大纲规划层）IPC 命令
//!
//! 两层大纲模型：
//! - 「情节脉络」节点（OutlineArc）：讨论成果的剧情规划，覆盖一个章节范围
//!   （如第 1-200 章），本身不可写正文；
//! - 「章节细纲」（Chapter.summary）：脉络节点按范围分批展开生成，
//!   造化工坊只对真正的章节写作。
//!
//! `expand_outline_arc` 每次展开一批（默认 20 章），多次点击逐步展开，
//! 让作者在每个故事段内按自己的节奏推进，而不是一次吞下几百章。
use crate::state::AppState;
use pensoul_core::workflow::WorkflowRef;
use pensoul_core::{Chapter, ChapterId, ChapterStatus, OutlineArc, VolumeId};
use serde::Deserialize;

use super::json_fix;
use super::llm_helper as lh;

/// 每批展开的章节数（一次 LLM 调用的产出量，过多容易截断）
const DEFAULT_BATCH: i64 = 20;

/// 列出全部情节脉络节点
#[tauri::command]
pub async fn list_outline_arcs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<OutlineArc>, String> {
    let ontology = state.ontology.read();
    Ok(ontology.outline_arcs.clone())
}

/// 整体保存情节脉络（新建/编辑/删除都走这里；导入讨论成果由前端组装后调用）
#[tauri::command]
pub async fn save_outline_arcs(
    state: tauri::State<'_, AppState>,
    arcs: Vec<OutlineArc>,
) -> Result<(), String> {
    let samples = {
        let onto = state.ontology.read();
        crate::edits::outline_arcs_diff_samples(&onto.outline_arcs, &arcs)
    };
    {
        let mut ontology = state.ontology.write();
        ontology.outline_arcs = arcs;
    }
    crate::edits::record_edit_samples(&state, samples);
    state.save().map_err(|e| e.to_string())
}

/// 展开结果：本批生成的章节范围与节点完成状态
#[derive(serde::Serialize)]
pub struct ExpandResult {
    pub created: usize,
    pub from: i64,
    pub to: i64,
    /// 该节点是否已全部展开（expanded_until >= chapter_end）
    pub arc_done: bool,
}

/// 细纲条目（LLM 产出的单章规划；章号由后端按批次顺序分配，不信任模型编号）
#[derive(Debug, Deserialize)]
struct BeatPlan {
    title: String,
    #[serde(default)]
    summary: String,
}

/// 展开脉络节点的下一批细纲：调 LLM 把该故事段的剧情规划
/// 拆成逐章梗概，生成真正的章节实体（正文留空，等待造化工坊写作）
/// `skill_cards`：工作流为细纲展开环节绑定的技法卡路径（可空，注入 system prompt）
#[tauri::command]
pub async fn expand_outline_arc(
    state: tauri::State<'_, AppState>,
    arc_id: String,
    model: Option<String>,
    batch: Option<i64>,
    skill_cards: Option<Vec<String>>,
) -> Result<ExpandResult, String> {
    lh::ensure_api_keys_loaded(&state);

    // 取节点快照并计算本批范围
    let (arc, from, to) = {
        let ontology = state.ontology.read();
        let arc = ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .cloned()
            .ok_or_else(|| "脉络节点不存在".to_string())?;
        let from = if arc.expanded_until <= 0 {
            arc.chapter_start
        } else {
            arc.expanded_until + 1
        };
        if from > arc.chapter_end {
            return Err("该节点已全部展开为细纲".to_string());
        }
        let to = (from + batch.unwrap_or(DEFAULT_BATCH) - 1).min(arc.chapter_end);
        (arc, from, to)
    };

    // 组装上下文：核心概念 + 创作设定 + 节点规划 + 衔接前情
    let (concept_brief, settings_brief, prev_tail, volume_id) = {
        let ontology = state.ontology.read();
        let c = &ontology.core_concept;
        let concept_brief = format!(
            "高概念：{}；前提：{}；主角：{}；基调：{}；核心冲突：{}",
            c.high_concept, c.premise, c.protagonist_hint, c.tone, c.central_conflict
        );
        let s = &ontology.settings;
        let settings_brief = format!(
            "类型：{}；目标总章数：{} 章；每章目标字数：{} 字",
            s.genre, s.target_chapters, s.chapter_target_words
        );
        // 衔接：本批起点之前最近 2 章的梗概，保证剧情连续
        let prev_tail: String = ontology
            .chapters
            .iter()
            .filter(|ch| ch.chapter_no > 0 && ch.chapter_no < from && !ch.summary.is_empty())
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|ch| format!("第{}章《{}》：{}", ch.chapter_no, ch.title, ch.summary))
            .collect::<Vec<_>>()
            .join("\n");
        // 章节落卷规则与讨论导入一致：优先第一个真实卷，否则隐式默认卷
        let volume_id = ontology
            .volumes
            .iter()
            .find(|v| v.volume_id.as_str() != "_default")
            .map(|v| v.volume_id.as_str().to_string())
            .unwrap_or_else(|| "_default".to_string());
        (concept_brief, settings_brief, prev_tail, volume_id)
    };

    let model_id = resolve_expand_model(&state, model)?;
    let count = to - from + 1;
    let mut system = "你是小说大纲规划师。你的任务是把一段剧情脉络拆解为逐章细纲，输出严格 JSON。\
        不评论、不解释，只输出 JSON 数组。"
        .to_string();
    // 工作流为细纲展开绑定的技法卡（结构/人物/张力/类型维度），注入为方法手册。
    // 显式参数优先，缺省时按「项目覆盖 → 模板绑定」解析（与造化工坊同一套规则）
    let cards_block =
        super::book_distill::load_writing_cards(&state, &resolve_expand_cards(&state, skill_cards));
    if !cards_block.is_empty() {
        system.push_str(&format!(
            "\n\n【写作技法卡】\n\
            以下是本书选定工作流绑定的写作技法卡，是你拆解细纲的方法手册：\n\
            篇章布局遵循其「I · 技法骨架」与「E · 执行步骤」，节奏与结构遵守其「B · 边界」。\n\n{cards_block}"
        ));
    }
    let user = format!(
        "【核心概念】\n{concept_brief}\n\n\
         【创作设定】\n{settings_brief}\n\n\
         【情节脉络节点】\n标题：{}\n覆盖范围：第 {}-{} 章（全段共 {} 章）\n剧情规划：\n{}\n\n\
         {}\
         现在请为本段生成第 {from} 章到第 {to} 章（共 {count} 章）的逐章细纲。\n\
         输出 JSON 数组，每章一个对象：\n\
         [{{\"title\": \"章节标题\", \"summary\": \"本章梗概（80-150字：本章发生什么、核心冲突是什么、如何推进主线）\"}}]\n\
         要求：\n\
         - 严格生成 {count} 章，不多不少，按故事发生顺序排列\n\
         - 每章梗概必须是独立的章节规划，承接前情、彼此衔接，不要把整段剧情压缩进一章\n\
         - 本批只是全段的一部分，节奏按全段跨度把控（该铺垫时铺垫，该推进时推进）\n\
         - 章节标题要有辨识度，不要「第一章」「第二章」式命名\n\
         - 所有内容用中文",
        arc.title,
        arc.chapter_start,
        arc.chapter_end,
        arc.chapter_end - arc.chapter_start + 1,
        arc.description,
        if prev_tail.is_empty() {
            String::new()
        } else {
            format!("【前情衔接（已展开的最近章节）】\n{prev_tail}\n\n")
        },
    );

    let (provider_id, api_key, api_base) = {
        let models = lh::load_models(&state);
        let providers = lh::load_providers(&state);
        let m2p = lh::build_model_to_provider(&models);
        let bases = lh::build_provider_api_bases(&providers);
        let keys = state.api_keys.read().clone();
        lh::resolve_provider(&model_id, &m2p, &bases, &keys)?
    };
    let raw = lh::call_llm_task(
        &lh::ProviderAuth {
            provider_id: &provider_id,
            api_key: &api_key,
            api_base: &api_base,
        },
        &model_id,
        &system,
        &user,
        0.6,
        // 每章梗概约 150 字，20 章约 4000 字正文；推理型模型还要预留思考预算
        16384,
        crate::llm_profile::LlmTask::Light,
    )
    .await?;

    let plans = parse_beat_plans(&raw)?;
    if plans.is_empty() {
        return Err("模型未产出任何细纲条目，请重试".to_string());
    }

    // 落库：按批次顺序分配章号（from + 下标），已有该章号的章节跳过防重复
    let created = {
        let mut ontology = state.ontology.write();
        let existing: std::collections::HashSet<i64> =
            ontology.chapters.iter().map(|c| c.chapter_no).collect();
        let now = chrono::Utc::now().to_rfc3339();
        let mut created = 0usize;
        for (i, plan) in plans.iter().enumerate() {
            let chapter_no = from + i as i64;
            if chapter_no > to {
                break; // 模型多产出的部分丢弃
            }
            if existing.contains(&chapter_no) {
                continue;
            }
            let title = plan.title.trim();
            if title.is_empty() {
                continue;
            }
            ontology.chapters.push(Chapter {
                chapter_id: ChapterId::new(format!(
                    "ch-{}-{}",
                    chapter_no,
                    uuid::Uuid::new_v4().simple()
                )),
                chapter_no,
                volume_id: VolumeId::new(volume_id.clone()),
                title: title.to_string(),
                summary: plan.summary.trim().to_string(),
                content: String::new(),
                word_count: 0,
                version: 1,
                status: ChapterStatus::Draft,
                consistency_score: 1.0,
                created_at: now.clone(),
                updated_at: now.clone(),
                annotations: Vec::new(),
                revisions: Vec::new(),
            });
            created += 1;
        }
        // 更新节点展开进度（按实际落库的最后一章）
        if created > 0 {
            let last = ontology
                .chapters
                .iter()
                .filter(|c| c.chapter_no >= from && c.chapter_no <= to)
                .map(|c| c.chapter_no)
                .max()
                .unwrap_or(from);
            if let Some(a) = ontology
                .outline_arcs
                .iter_mut()
                .find(|a| a.arc_id == arc_id)
            {
                a.expanded_until = last.max(a.expanded_until);
            }
        }
        // 同步卷的章节列表
        let mut by_volume: std::collections::HashMap<String, Vec<ChapterId>> =
            std::collections::HashMap::new();
        for ch in &ontology.chapters {
            by_volume
                .entry(ch.volume_id.as_str().to_string())
                .or_default()
                .push(ch.chapter_id.clone());
        }
        for vol in ontology.volumes.iter_mut() {
            if let Some(ids) = by_volume.get(vol.volume_id.as_str()) {
                vol.chapter_ids = ids.clone();
            }
        }
        created
    };

    if created == 0 {
        return Err("本批细纲没有新章节落库（对应章号可能已存在）".to_string());
    }
    state.save().map_err(|e| e.to_string())?;

    let arc_done = {
        let ontology = state.ontology.read();
        ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .map(|a| a.expanded_until >= a.chapter_end)
            .unwrap_or(false)
    };
    Ok(ExpandResult {
        created,
        from,
        to,
        arc_done,
    })
}

/// 解析展开模型：指定优先，其次项目覆盖/模板绑定的 outline_expand 模型，
/// 最后取第一个「供应商有 Key」的可用模型
fn resolve_expand_model(state: &AppState, model: Option<String>) -> Result<String, String> {
    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        return Ok(m);
    }
    if let Some(m) = resolve_bound_expand_model(state) {
        return Ok(m);
    }
    let models = lh::load_models(state);
    let keys = state.api_keys.read().clone();
    models
        .iter()
        .find_map(|m| {
            let model_id = m.get("model_id")?.as_str()?.to_string();
            let provider_id = m.get("provider_id")?.as_str()?;
            keys.contains_key(provider_id).then_some(model_id)
        })
        .ok_or_else(|| "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string())
}

/// 从项目工作流引用解析细纲展开的技法卡：显式参数 > 项目覆盖 > 模板绑定 > 空
fn resolve_expand_cards(state: &AppState, skill_cards: Option<Vec<String>>) -> Vec<String> {
    if let Some(cards) = skill_cards
        && !cards.is_empty()
    {
        return cards;
    }
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) else {
        return Vec::new();
    };
    if let Some(cards) = wf_ref
        .overrides
        .get("outline_expand")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        && !cards.is_empty()
    {
        return cards;
    }
    let Some(template_id) = wf_ref.template_id else {
        return Vec::new();
    };
    let templates = state.workflow_templates.read();
    let Some(bindings) = templates
        .iter()
        .find(|t| t.template_id == template_id)
        .map(|t| t.stage_bindings("outline_expand"))
    else {
        return Vec::new();
    };
    bindings
        .get("cards")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// 从项目工作流引用解析细纲展开的绑定模型（覆盖 > 模板绑定）
fn resolve_bound_expand_model(state: &AppState) -> Option<String> {
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) else {
        return None;
    };
    if let Some(m) = wf_ref
        .overrides
        .get("outline_expand")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.as_str())
        .filter(|m| !m.trim().is_empty())
    {
        return Some(m.to_string());
    }
    let template_id = wf_ref.template_id?;
    let templates = state.workflow_templates.read();
    let Some(bindings) = templates
        .iter()
        .find(|t| t.template_id == template_id)
        .map(|t| t.stage_bindings("outline_expand"))
    else {
        return None;
    };
    bindings
        .get("model")
        .and_then(|v| v.as_str())
        .filter(|m| !m.trim().is_empty())
        .map(|m| m.to_string())
}

/// 解析 LLM 产出的细纲 JSON 数组（先严格解析，失败则走容错修复）
fn parse_beat_plans(raw: &str) -> Result<Vec<BeatPlan>, String> {
    let text = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<Vec<BeatPlan>>(text) {
        return Ok(v);
    }
    let v = json_fix::repair_to_value(text).map_err(|e| format!("细纲 JSON 解析失败: {e}"))?;
    // 模型可能包一层对象（如 {"chapters": [...]}），取第一个数组值
    let arr = match &v {
        serde_json::Value::Array(_) => v,
        serde_json::Value::Object(m) => m
            .values()
            .find(|x| x.is_array())
            .cloned()
            .ok_or_else(|| "细纲 JSON 中找不到章节数组".to_string())?,
        _ => return Err("细纲 JSON 不是数组结构".to_string()),
    };
    serde_json::from_value::<Vec<BeatPlan>>(arr).map_err(|e| format!("细纲条目解析失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_beat_plans_plain_array() {
        let raw = r#"[{"title": "井边惊魂", "summary": "主角发现尸体"}, {"title": "初探", "summary": "展开调查"}]"#;
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].title, "井边惊魂");
    }

    #[test]
    fn test_parse_beat_plans_with_fence_and_prose() {
        let raw = "好的，以下是细纲：\n```json\n[{\"title\": \"a\", \"summary\": \"b\"}]\n```\n希望有帮助。";
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 1);
    }

    #[test]
    fn test_parse_beat_plans_wrapped_object() {
        let raw = r#"{"chapters": [{"title": "a", "summary": "b"}]}"#;
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].summary, "b");
    }

    #[test]
    fn test_parse_beat_plans_invalid() {
        assert!(parse_beat_plans("这不是 JSON").is_err());
    }
}
