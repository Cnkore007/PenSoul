//! 编辑经验沉淀 —— 把用户对世界观/人物志/大纲/细纲/正文的保存修改
//! 采样为 EditSample，批量蒸馏成 WritingLesson 注入审查，让修改也进入经验累计。
use crate::commands::chapter_rewrite::{LessonItem, merge_lessons};
use crate::commands::json_fix;
use crate::commands::llm_helper as lh;
use crate::llm_profile::LlmTask;
use crate::state::AppState;
use pensoul_core::{Chapter, EditSample, WritingLesson};

/// 样本摘录上限（防样本过大，蒸馏时也按此截断）
const SAMPLE_CAP: usize = 240;

fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// 已解析的可用模型（自持字符串，避免 ProviderAuth 生命周期纠缠）
pub(crate) struct ResolvedModel {
    pub provider_id: String,
    pub api_key: String,
    pub api_base: String,
    pub model_id: String,
}

/// 解析第一个可用供应商下的可用模型
pub(crate) fn resolve_any_model(state: &AppState) -> Result<ResolvedModel, String> {
    let providers = lh::load_providers(state);
    let models = lh::load_models(state);
    let api_keys = { state.api_keys.read().clone() };
    let (provider_id, api_key, api_base) =
        lh::find_any_available_provider(&providers, &api_keys)
            .ok_or_else(|| "未配置任何 LLM API Key，请在模型设置中配置".to_string())?;
    let model_id = models
        .iter()
        .find(|m| {
            m.get("provider_id").and_then(|v| v.as_str()) == Some(&provider_id)
                && m.get("is_available")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .and_then(|m| m.get("model_id").and_then(|v| v.as_str()))
        .unwrap_or("gpt-4o")
        .to_string();
    Ok(ResolvedModel {
        provider_id,
        api_key,
        api_base,
        model_id,
    })
}

/// 截断文本到上限
fn cap(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let head: String = text.chars().take(max).collect();
        format!("{head}…")
    }
}

/// 定位两段文本首个差异处，各取前后窗口生成「修改前/修改后」摘要
fn diff_snippet(before: &str, after: &str) -> (String, String) {
    let (before, after) = (before.trim(), after.trim());
    if before == after {
        return (String::new(), String::new());
    }
    let b_chars: Vec<char> = before.chars().collect();
    let a_chars: Vec<char> = after.chars().collect();
    let common_prefix = b_chars
        .iter()
        .zip(a_chars.iter())
        .take_while(|(x, y)| x == y)
        .count();
    // 差异区上下文：前缀（最多 40 字）+ 差异起始后的内容（最多 60 字）
    let ctx = common_prefix.min(40);
    let b_head: String = b_chars[..ctx].iter().collect();
    let a_head: String = a_chars[..ctx].iter().collect();
    let b_diff: String = b_chars[common_prefix..].iter().take(60).collect();
    let a_diff: String = a_chars[common_prefix..].iter().take(60).collect();
    (
        format!("…{}…", cap(&format!("{b_head}{b_diff}"), SAMPLE_CAP)),
        format!("…{}…", cap(&format!("{a_head}{a_diff}"), SAMPLE_CAP)),
    )
}

/// 采集单个修改样本（无实质变化返回 None）
fn capture_edit_sample(scope: &str, label: &str, before: &str, after: &str) -> Option<EditSample> {
    let (before, after) = (before.trim(), after.trim());
    if before.is_empty() && after.is_empty() {
        return None;
    }
    if before == after {
        return None;
    }
    let (b, a) = diff_snippet(before, after);
    Some(EditSample {
        sample_id: format!("edit-{}", uuid::Uuid::new_v4().simple()),
        scope: scope.to_string(),
        label: label.to_string(),
        before: b,
        after: a,
        created_at: now(),
    })
}

/// 世界观修改采样：对比新旧层，收集地点/时间线/设定/术语的内容变化
pub fn world_diff_samples(
    old: &pensoul_core::WorldLayer,
    new: &pensoul_core::WorldLayer,
) -> Vec<EditSample> {
    let mut out = Vec::new();
    for (o, n) in old.spatial_model.locations.iter().zip(new.spatial_model.locations.iter()) {
        if o.id != n.id {
            continue;
        }
        if let Some(s) = capture_edit_sample(
            "world",
            &format!("世界观·地点「{}」", n.name),
            &o.description,
            &n.description,
        ) {
            out.push(s);
        }
    }
    for (o, n) in old.timeline.events.iter().zip(new.timeline.events.iter()) {
        if o.event_id != n.event_id {
            continue;
        }
        if let Some(s) = capture_edit_sample(
            "world",
            &format!("世界观·时间线「{}」", n.story_time),
            &o.description,
            &n.description,
        ) {
            out.push(s);
        }
    }
    for (o, n) in old.setting_rules.iter().zip(new.setting_rules.iter()) {
        if o.rule_id != n.rule_id {
            continue;
        }
        if let Some(s) = capture_edit_sample(
            "world",
            &format!("世界观·设定规则《{}》", n.title),
            &o.description,
            &n.description,
        ) {
            out.push(s);
        }
    }
    for (o, n) in old.glossary.iter().zip(new.glossary.iter()) {
        if o.term != n.term {
            continue;
        }
        if let Some(s) = capture_edit_sample(
            "world",
            &format!("世界观·术语「{}」", n.term),
            &o.definition,
            &n.definition,
        ) {
            out.push(s);
        }
    }
    out
}

/// 人物志修改采样
pub fn characters_diff_samples(
    old: &pensoul_core::CharacterLayer,
    new: &pensoul_core::CharacterLayer,
) -> Vec<EditSample> {
    let mut out = Vec::new();
    for (o, n) in old.characters.iter().zip(new.characters.iter()) {
        if o.id != n.id {
            continue;
        }
        let traits_of = |c: &pensoul_core::Character| -> String {
            c.core_personality
                .traits
                .iter()
                .map(|(t, _)| t.as_str())
                .collect::<Vec<_>>()
                .join("、")
        };
        if let Some(s) = capture_edit_sample(
            "character",
            &format!("人物「{}」", n.name),
            &format!("性格：{}", traits_of(o)),
            &format!("性格：{}", traits_of(n)),
        ) {
            out.push(s);
        }
        if o.name != n.name
            && let Some(s) = capture_edit_sample("character", "人物名称", &o.name, &n.name)
        {
            out.push(s);
        }
    }
    out
}

/// 章节修改采样：细纲（outline scope）与正文（chapter scope）分开
pub fn chapter_diff_samples(
    old: &Chapter,
    title: &str,
    summary: &str,
    content: &str,
) -> Vec<EditSample> {
    let mut out = Vec::new();
    if let Some(s) = capture_edit_sample(
        "outline",
        &format!("细纲·第 {} 章《{}》", old.chapter_no, title),
        &old.summary,
        summary,
    ) {
        out.push(s);
    }
    if let Some(s) = capture_edit_sample(
        "chapter",
        &format!("正文·第 {} 章《{}》", old.chapter_no, title),
        &old.content,
        content,
    ) {
        out.push(s);
    }
    out
}

/// 脉络节点修改采样
pub fn outline_arcs_diff_samples(
    old: &[pensoul_core::OutlineArc],
    new: &[pensoul_core::OutlineArc],
) -> Vec<EditSample> {
    let mut out = Vec::new();
    for (o, n) in old.iter().zip(new.iter()) {
        if o.arc_id != n.arc_id {
            continue;
        }
        if let Some(s) = capture_edit_sample(
            "outline",
            &format!("脉络节点《{}》", n.title),
            &o.description,
            &n.description,
        ) {
            out.push(s);
        }
    }
    out
}

/// 合并样本进待沉淀队列：同 scope + label 只保留最新一条（避免反复编辑刷屏）
pub fn record_edit_samples(state: &AppState, samples: Vec<EditSample>) {
    if samples.is_empty() {
        return;
    }
    let mut onto = state.ontology.write();
    for s in samples {
        onto.pending_edit_samples
            .retain(|e| !(e.scope == s.scope && e.label == s.label));
        onto.pending_edit_samples.push(s);
    }
    if onto.pending_edit_samples.len() > 200 {
        let excess = onto.pending_edit_samples.len() - 200;
        onto.pending_edit_samples.drain(..excess);
    }
}

/// 待沉淀样本数量（前端角标用）
pub fn pending_edit_count(state: &AppState) -> usize {
    state.ontology.read().pending_edit_samples.len()
}

/// LLM 蒸馏待沉淀样本为写作经验并合并进经验库（scope 跟随来源环节）
pub(crate) async fn distill_pending_lessons_internal(
    state: &AppState,
) -> Result<Vec<WritingLesson>, String> {
    lh::ensure_api_keys_loaded(state);
    let samples = {
        let onto = state.ontology.read();
        onto.pending_edit_samples.clone()
    };
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    let rm = resolve_any_model(state)?;
    let model_id = rm.model_id.as_str();
    let auth = lh::ProviderAuth {
        provider_id: &rm.provider_id,
        api_key: &rm.api_key,
        api_base: &rm.api_base,
    };

    let system = "你是写作复盘教练，负责从作者的修改动作中提炼可复用的写作经验。\
        输出严格 JSON，不评论、不解释。";
    let sample_lines = samples
        .iter()
        .map(|s| {
            format!(
                "- [{}] {}：\n  改前：{}\n  改后：{}",
                s.scope, s.label, s.before, s.after
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let user = format!(
        "以下是作者对各创作环节内容的修改记录（改前 → 改后）：\n{sample_lines}\n\n\
         请把每个修改归类为一条写作经验，输出：\n\
         {{\"lessons\": [{{\"scope\": \"chapter 或 outline 或 world 或 character\", \
         \"category\": \"措辞 或 节奏 或 对话 或 一致性 或 反AI味 或 结构 或 设定 或 其他\", \
         \"problem\": \"具体问题（30字内，可复用的一句话教训）\", \
         \"fix\": \"改正方法（50字内，可直接执行的写法）\"}}]}}\n\
         要求：\n\
         1. 一个修改样本最多归纳为一条经验；多条样本可合并为同一条经验（如多次修正同一处设定）\n\
         2. 只归纳有可复用教训的修改，纯润色无实质变化的跳过\n\
         3. scope 取修改来源环节\n\
         4. 用 ===LESSONS_BEGIN=== 与 ===LESSONS_END=== 包裹纯 JSON，全部内容用中文"
    );
    let raw = lh::call_llm_task(&auth, model_id, system, &user, 0.2, 4096, LlmTask::Light).await?;
    let json_str = extract_block(&raw, "===LESSONS_BEGIN===", "===LESSONS_END===");
    let payload: DistillPayload = serde_json::from_str(&json_str)
        .or_else(|strict_err| {
            json_fix::repair_to_value(&json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<DistillPayload>(v).ok())
                .ok_or(strict_err.to_string())
        })
        .map_err(|e| format!("编辑经验提炼解析失败: {e}"))?;

    let mut onto = state.ontology.write();
    let merged = merge_lessons(&mut onto.writing_lessons, payload.lessons, "用户修改");
    onto.pending_edit_samples.clear();
    drop(onto);
    state.save().map_err(|e| e.to_string())?;
    Ok(merged)
}

/// 待沉淀编辑样本列表（前端展示用）
#[tauri::command]
pub async fn get_pending_edits(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<EditSample>, String> {
    let onto = state.ontology.read();
    Ok(onto.pending_edit_samples.clone())
}

/// 一键沉淀：把待处理编辑样本蒸馏为写作经验
#[tauri::command]
pub async fn distill_pending_lessons(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WritingLesson>, String> {
    distill_pending_lessons_internal(&state).await
}

fn extract_block(raw: &str, begin: &str, end: &str) -> String {
    let b = raw.find(begin).map(|i| i + begin.len());
    let e = raw.rfind(end);
    match (b, e) {
        (Some(b), Some(e)) if e > b => raw[b..e].trim().to_string(),
        _ => raw.trim().to_string(),
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct DistillPayload {
    #[serde(default)]
    lessons: Vec<LessonItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_snippet_same_text_empty() {
        let (b, a) = diff_snippet("一样的内容", "一样的内容");
        assert!(b.is_empty() && a.is_empty());
    }

    #[test]
    fn test_diff_snippet_captures_change() {
        let (b, a) = diff_snippet("主角叫林晚，性格冷峻", "主角叫林晚，性格温和");
        assert!(b.contains("冷峻"));
        assert!(a.contains("温和"));
        assert!(b.contains("林晚"));
    }

    #[test]
    fn test_capture_edit_sample_skips_noop() {
        assert!(capture_edit_sample("world", "地点", "同", "同").is_none());
        assert!(capture_edit_sample("world", "地点", "", "").is_none());
        let s = capture_edit_sample("world", "地点「谷」", "幽静", "热闹").unwrap();
        assert_eq!(s.scope, "world");
        assert_eq!(s.label, "地点「谷」");
        assert!(s.after.contains("热闹"));
    }

    #[test]
    fn test_record_edit_samples_dedup_by_label() {
        let state = crate::AppState::new(std::path::PathBuf::from("/tmp/pensoul-test-edits"));
        record_edit_samples(
            &state,
            vec![capture_edit_sample("world", "地点「谷」", "a", "b").unwrap()],
        );
        record_edit_samples(
            &state,
            vec![capture_edit_sample("world", "地点「谷」", "b", "c").unwrap()],
        );
        assert_eq!(pending_edit_count(&state), 1);
        let onto = state.ontology.read();
        assert_eq!(onto.pending_edit_samples[0].after, "…c…");
    }
}
