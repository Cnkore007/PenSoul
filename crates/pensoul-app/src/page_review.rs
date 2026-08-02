//! 页面受控保存 —— 「保存并审核」流程
//!
//! 1. 收集页面待处理批注与编辑修改样本；
//! 2. LLM 判定每条是否有效 + 评估对全文影响；
//! 3. 用户二次确认后应用：记录快照 → 覆盖数据 → 批注状态流转 → 有效样本沉淀经验；
//! 4. 快照支持一键撤回。
use crate::commands::json_fix;
use crate::commands::llm_helper as lh;
use crate::edits::resolve_any_model;
use crate::llm_profile::LlmTask;
use crate::state::AppState;
use pensoul_core::{ChapterAnnotation, EditSample, PageSnapshot, WritingLesson};

/// 收集到的页面变更条目（批注或修改样本）
pub(crate) struct PageItem {
    pub(crate) source: String, // annotation / edit
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) content: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ReviewItem {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(serde::Serialize)]
pub struct PageReview {
    pub items: Vec<ReviewItem>,
    pub impact: String,
}

#[derive(serde::Deserialize)]
struct ReviewPayload {
    #[serde(default)]
    items: Vec<ReviewItem>,
    #[serde(default)]
    impact: String,
}

fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// 兼容解析世界观：优先后端完整结构；失败则按前端扁平结构
/// `{locations:[{id,name,description,spatial_tags}], timeline_events:[...], setting_rules:[...]}`
fn parse_world_layer(v: &serde_json::Value) -> Result<pensoul_core::WorldLayer, String> {
    if let Ok(layer) = serde_json::from_value::<pensoul_core::WorldLayer>(v.clone()) {
        return Ok(layer);
    }
    let arr = |key: &str| -> Vec<serde_json::Value> {
        v.get(key)
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let locations = arr("locations")
        .into_iter()
        .filter_map(|l| serde_json::from_value(l).ok())
        .collect();
    let events = arr("timeline_events")
        .into_iter()
        .filter_map(|e| serde_json::from_value(e).ok())
        .collect();
    let rules = arr("setting_rules")
        .into_iter()
        .filter_map(|r| serde_json::from_value(r).ok())
        .collect();
    Ok(pensoul_core::WorldLayer {
        world_id: pensoul_core::WorldId::new("default"),
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("default")
            .to_string(),
        spatial_model: pensoul_core::SpatialModel {
            locations,
            hierarchy: Vec::new(),
        },
        timeline: pensoul_core::Timeline {
            events,
            epoch_markers: Vec::new(),
        },
        setting_rules: rules,
        glossary: Vec::new(),
        item_graph: Vec::new(),
    })
}

/// 兼容解析人物志：优先后端完整结构；失败则按前端扁平 CharacterData[] 构建
fn parse_character_layer(v: &serde_json::Value) -> Result<pensoul_core::CharacterLayer, String> {
    if let Ok(layer) = serde_json::from_value::<pensoul_core::CharacterLayer>(v.clone()) {
        return Ok(layer);
    }
    let chars: Vec<pensoul_core::Character> = v
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|c| {
            let name = c.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let traits: Vec<(String, f32)> = c
                .get("personality_traits")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| {
                            let name = t.get(0).and_then(|x| x.as_str())?.to_string();
                            let strength = t
                                .get(1)
                                .and_then(|x| x.as_f64())
                                .unwrap_or(0.5) as f32;
                            Some((name, strength))
                        })
                        .collect()
                })
                .unwrap_or_default();
            pensoul_core::Character {
                id: pensoul_core::CharacterId::new(
                    c.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                ),
                name,
                core_personality: pensoul_core::PersonalityVector { traits },
                current_mood: pensoul_core::Emotion {
                    primary: c
                        .get("current_mood")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    intensity: 0.5,
                    secondary: String::new(),
                },
                current_location: String::new(),
                current_knowledge: pensoul_core::KnowledgeSet { facts: Vec::new() },
                state_history: Vec::new(),
                transition_rules: Vec::new(),
                dialogue_style: pensoul_core::DialogueStyle {
                    patterns: Vec::new(),
                    vocabulary_level: "normal".to_string(),
                    sentence_length_avg: 15.0,
                    catchphrases: Vec::new(),
                },
                growth_curve: Vec::new(),
                knowledge_base: pensoul_core::CharacterKnowledgeBase {
                    known_facts: Vec::new(),
                    knowledge_sources: Vec::new(),
                    decay_model: pensoul_core::DecayModel {
                        half_life_chapters: 10,
                        min_reliability: 0.1,
                    },
                },
                annotations: Vec::new(),
            }
        })
        .collect();
    // relationships：拍平各角色的关系并去重
    let mut seen = std::collections::HashSet::new();
    let relationships: Vec<pensoul_core::Relationship> = v
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|c| {
            c.get("relationships")
                .and_then(|x| x.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|r| {
            let from = r.get("from").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let to = r.get("to").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let relation_type = r
                .get("relation_type")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let key = format!("{from}|{to}|{relation_type}");
            if !seen.insert(key) {
                return None;
            }
            let strength = r
                .get("strength")
                .and_then(|x| x.as_f64())
                .filter(|s| s.is_finite())
                .unwrap_or(0.5) as f32;
            Some(pensoul_core::Relationship {
                from: pensoul_core::CharacterId::new(from),
                to: pensoul_core::CharacterId::new(to),
                relation_type,
                strength,
                history: Vec::new(),
            })
        })
        .collect();
    Ok(pensoul_core::CharacterLayer {
        characters: chars,
        relationships,
    })
}

/// 把旧实体的批注合并回新层（受控保存覆盖时保留批注）
pub(crate) fn merge_world_annotations(
    old: &pensoul_core::WorldLayer,
    new: &mut pensoul_core::WorldLayer,
) {
    for l in &mut new.spatial_model.locations {
        if let Some(ol) = old.spatial_model.locations.iter().find(|o| o.id == l.id) {
            l.annotations = ol.annotations.clone();
        }
    }
    for e in &mut new.timeline.events {
        if let Some(oe) = old.timeline.events.iter().find(|o| o.event_id == e.event_id) {
            e.annotations = oe.annotations.clone();
        }
    }
    for r in &mut new.setting_rules {
        if let Some(or) = old.setting_rules.iter().find(|o| o.rule_id == r.rule_id) {
            r.annotations = or.annotations.clone();
        }
    }
    for g in &mut new.glossary {
        if let Some(og) = old.glossary.iter().find(|o| o.term == g.term) {
            g.annotations = og.annotations.clone();
        }
    }
}

pub(crate) fn merge_character_annotations(
    old: &pensoul_core::CharacterLayer,
    new: &mut pensoul_core::CharacterLayer,
) {
    for c in &mut new.characters {
        if let Some(oc) = old.characters.iter().find(|o| o.id == c.id) {
            c.annotations = oc.annotations.clone();
        }
    }
}

/// 后端完整层 → 前端扁平结构（与 store.ts transformWorldData 对齐，供撤回写回）
fn world_to_frontend(layer: &pensoul_core::WorldLayer) -> serde_json::Value {
    serde_json::json!({
        "locations": layer.spatial_model.locations.iter().map(|l| serde_json::json!({
            "id": l.id, "name": l.name, "description": l.description, "spatial_tags": l.spatial_tags,
        })).collect::<Vec<_>>(),
        "timeline_events": layer.timeline.events.iter().map(|e| serde_json::json!({
            "event_id": e.event_id, "story_time": e.story_time, "description": e.description,
        })).collect::<Vec<_>>(),
        "setting_rules": layer.setting_rules.iter().map(|r| serde_json::json!({
            "rule_id": r.rule_id, "category": r.category, "title": r.title,
            "description": r.description, "constraints": r.constraints,
        })).collect::<Vec<_>>(),
    })
}

/// 后端完整层 → 前端 CharacterData[]（关系按角色名分发，与 transformCharacters 对齐）
fn characters_to_frontend(layer: &pensoul_core::CharacterLayer) -> serde_json::Value {
    serde_json::json!(layer
        .characters
        .iter()
        .map(|c| {
            let rels: Vec<serde_json::Value> = layer
                .relationships
                .iter()
                .filter(|r| r.from.as_str() == c.name || r.to.as_str() == c.name)
                .map(|r| {
                    serde_json::json!({
                        "from": r.from, "to": r.to,
                        "relation_type": r.relation_type, "strength": r.strength,
                    })
                })
                .collect();
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "personality_traits": c.core_personality.traits.iter()
                    .map(|(t, s)| serde_json::json!([t, s])).collect::<Vec<_>>(),
                "current_mood": c.current_mood.primary,
                "relationships": rels,
            })
        })
        .collect::<Vec<_>>())
}

/// 收集页面批注（open）+ 修改样本（scope 匹配）
fn collect_page_items(onto: &pensoul_core::NovelOntology, page: &str) -> Vec<PageItem> {
    let mut out = Vec::new();
    let mut push_annos = |annos: &[ChapterAnnotation], label_prefix: &str| {
        for a in annos {
            if a.status == "open" {
                out.push(PageItem {
                    source: "annotation".to_string(),
                    id: a.annotation_id.clone(),
                    label: format!("{label_prefix}批注"),
                    content: a.content.clone(),
                });
            }
        }
    };
    match page {
        "world" => {
            for l in &onto.world.spatial_model.locations {
                push_annos(&l.annotations, &format!("地点「{}」·", l.name));
            }
            for e in &onto.world.timeline.events {
                push_annos(&e.annotations, &format!("时间线「{}」·", e.story_time));
            }
            for r in &onto.world.setting_rules {
                push_annos(&r.annotations, &format!("设定《{}》·", r.title));
            }
            for g in &onto.world.glossary {
                push_annos(&g.annotations, &format!("术语「{}」·", g.term));
            }
        }
        "character" => {
            for c in &onto.characters.characters {
                push_annos(&c.annotations, &format!("人物「{}」·", c.name));
            }
        }
        _ => {}
    }
    for s in &onto.pending_edit_samples {
        if s.scope == page {
            out.push(PageItem {
                source: "edit".to_string(),
                id: s.sample_id.clone(),
                label: s.label.clone(),
                content: format!("改前：{}\n改后：{}", s.before, s.after),
            });
        }
    }
    out
}

pub(crate) fn review_prompt(items: &[PageItem], page: &str) -> String {
    let page_label = match page {
        "world" => "世界观",
        "character" => "人物志",
        _ => page,
    };
    let item_lines = items
        .iter()
        .map(|i| format!("- [{}#{}] {}：{}", i.source, i.id, i.label, i.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "你是小说项目的编辑审校。以下是用户在「{page_label}」页面留下的批注与修改记录：\n\
         {item_lines}\n\n\
         请逐条判定：\n\
         1. verdict：该批注/修改是否为有效问题或合理改进（valid=有效，invalid=无效/误报，uncertain=待商榷）\n\
         2. reason：一句话理由\n\
         并评估整体影响 impact：这些变更会牵动全文哪些章节/实体、有无一致性风险、是否需要联动修改其他页面。\n\n\
         输出严格 JSON：\n\
         {{\"items\":[{{\"source\":\"annotation|edit\",\"id\":\"...\",\"label\":\"...\",\"content\":\"...\",\
         \"verdict\":\"valid|invalid|uncertain\",\"reason\":\"...\"}}],\"impact\":\"...\"}}\n\
         用 ===REVIEW_BEGIN=== 与 ===REVIEW_END=== 包裹纯 JSON，全部内容用中文"
    )
}

/// 判定页面批注与修改的有效性及对全文的影响（只读，不落库）
#[tauri::command]
pub async fn review_page_changes(
    state: tauri::State<'_, AppState>,
    page: String,
    _content_json: String,
) -> Result<PageReview, String> {
    lh::ensure_api_keys_loaded(&state);
    let items = {
        let onto = state.ontology.read();
        collect_page_items(&onto, &page)
    };
    if items.is_empty() {
        return Ok(PageReview {
            items: Vec::new(),
            impact: "本页没有待处理的批注或修改。".to_string(),
        });
    }
    let rm = resolve_any_model(&state)?;
    let auth = lh::ProviderAuth {
        provider_id: &rm.provider_id,
        api_key: &rm.api_key,
        api_base: &rm.api_base,
    };
    let raw = lh::call_llm_task(
        &auth,
        &rm.model_id,
        "你是严谨的编辑审校，输出严格 JSON，不评论、不解释。",
        &review_prompt(&items, &page),
        0.1,
        4096,
        LlmTask::Light,
    )
    .await?;
    let json_str = extract_block(&raw, "===REVIEW_BEGIN===", "===REVIEW_END===");
    let payload: ReviewPayload = serde_json::from_str(&json_str)
        .or_else(|strict_err| {
            json_fix::repair_to_value(&json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<ReviewPayload>(v).ok())
                .ok_or(strict_err.to_string())
        })
        .map_err(|e| format!("审校判定解析失败: {e}"))?;
    Ok(PageReview {
        items: payload.items,
        impact: payload.impact,
    })
}

/// 按最终决定流转批注状态（valid→accepted / invalid→rejected / 其余保持 open）
fn resolve_annotations(annos: &mut [ChapterAnnotation], verdicts: &std::collections::HashMap<String, String>) {
    let ts = now();
    for a in annos.iter_mut() {
        if a.status != "open" {
            continue;
        }
        match verdicts.get(&a.annotation_id).map(|s| s.as_str()) {
            Some("valid") => {
                a.status = "accepted".to_string();
                a.resolved_by = Some("manual".to_string());
                a.resolved_at = Some(ts.clone());
            }
            Some("invalid") => {
                a.status = "rejected".to_string();
                a.resolved_by = Some("manual".to_string());
                a.resolved_at = Some(ts.clone());
            }
            _ => {}
        }
    }
}

/// 确认并应用页面保存：快照 → 覆盖数据 → 批注流转 → 有效样本沉淀经验
#[tauri::command]
pub async fn apply_page_review(
    state: tauri::State<'_, AppState>,
    page: String,
    content_json: String,
    confirmations: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // 解析用户最终决定：id → verdict（覆盖 LLM 初判）
    let mut verdicts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in &confirmations {
        if let (Some(id), Some(v)) = (
            c.get("id").and_then(|v| v.as_str()),
            c.get("verdict").and_then(|v| v.as_str()),
        ) {
            verdicts.insert(id.to_string(), v.to_string());
        }
    }

    // 快照优先取「编辑前」完整数据（即时保存机制下，当前层已是修改后的值）
    let before = {
        let edit_before = {
            let mut onto = state.ontology.write();
            onto.page_edit_before.remove(&page)
        };
        match edit_before {
            Some(b) => b,
            None => {
                let onto = state.ontology.read();
                match page.as_str() {
                    "world" => serde_json::to_value(&onto.world).map_err(|e| e.to_string())?,
                    "character" => {
                        serde_json::to_value(&onto.characters).map_err(|e| e.to_string())?
                    }
                    other => return Err(format!("不支持的页面类型: {other}")),
                }
            }
        }
    };
    let after: serde_json::Value =
        serde_json::from_str(&content_json).map_err(|e| format!("页面数据不是合法 JSON: {e}"))?;

    {
        let mut onto = state.ontology.write();
        match page.as_str() {
            "world" => {
                let mut layer = parse_world_layer(&after)?;
                let mut old_layer: pensoul_core::WorldLayer =
                    serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
                // 在旧实体批注上按判定流转，再合并进新层（新数据通常不带批注字段）
                for l in old_layer.spatial_model.locations.iter_mut() {
                    resolve_annotations(&mut l.annotations, &verdicts);
                }
                for e in old_layer.timeline.events.iter_mut() {
                    resolve_annotations(&mut e.annotations, &verdicts);
                }
                for r in old_layer.setting_rules.iter_mut() {
                    resolve_annotations(&mut r.annotations, &verdicts);
                }
                for g in old_layer.glossary.iter_mut() {
                    resolve_annotations(&mut g.annotations, &verdicts);
                }
                merge_world_annotations(&old_layer, &mut layer);
                onto.world = layer;
            }
            "character" => {
                let mut layer = parse_character_layer(&after)?;
                let mut old_layer: pensoul_core::CharacterLayer =
                    serde_json::from_value(before.clone()).map_err(|e| e.to_string())?;
                for c in old_layer.characters.iter_mut() {
                    resolve_annotations(&mut c.annotations, &verdicts);
                }
                merge_character_annotations(&old_layer, &mut layer);
                onto.characters = layer;
            }
            _ => return Err("不支持的页面类型".to_string()),
        }
        // 快照入栈（上限 10）
        onto.page_snapshots.push(PageSnapshot {
            page: page.clone(),
            before: before.clone(),
            after: after.clone(),
            created_at: now(),
        });
        if onto.page_snapshots.len() > 10 {
            let excess = onto.page_snapshots.len() - 10;
            onto.page_snapshots.drain(..excess);
        }
    }
    state.save().map_err(|e| e.to_string())?;

    // 有效样本沉淀：valid 的修改样本保留，valid 的批注转样本；invalid 的丢弃
    let valid_edits = {
        let onto = state.ontology.read();
        let mut keep = Vec::new();
        for s in &onto.pending_edit_samples {
            if s.scope == page {
                match verdicts.get(&s.sample_id).map(|v| v.as_str()) {
                    Some("valid") => keep.push(s.clone()),
                    Some("invalid") => {} // 丢弃无效修改样本
                    _ => keep.push(s.clone()), // 未涉及/待商榷保留
                }
            } else {
                keep.push(s.clone());
            }
        }
        keep
    };
    {
        let mut onto = state.ontology.write();
        onto.pending_edit_samples = valid_edits;
    }

    // valid 批注转成修改样本（统一进蒸馏）
    let annotation_samples: Vec<EditSample> = {
        let onto = state.ontology.read();
        let mut out = Vec::new();
        let mut push_valid = |annos: &[ChapterAnnotation], label_prefix: &str| {
            for a in annos {
                if a.status == "accepted" && verdicts.get(&a.annotation_id).map(|v| v.as_str()) == Some("valid") {
                    out.push(EditSample {
                        sample_id: format!("edit-{}", uuid::Uuid::new_v4().simple()),
                        scope: page.clone(),
                        label: format!("{label_prefix}批注"),
                        before: a.content.clone(),
                        after: "（批注已确认有效，按建议修订）".to_string(),
                        chapter_id: None,
                        created_at: now(),
                    });
                }
            }
        };
        match page.as_str() {
            "world" => {
                for l in &onto.world.spatial_model.locations {
                    push_valid(&l.annotations, &format!("地点「{}」·", l.name));
                }
                for e in &onto.world.timeline.events {
                    push_valid(&e.annotations, &format!("时间线「{}」·", e.story_time));
                }
                for r in &onto.world.setting_rules {
                    push_valid(&r.annotations, &format!("设定《{}》·", r.title));
                }
                for g in &onto.world.glossary {
                    push_valid(&g.annotations, &format!("术语「{}」·", g.term));
                }
            }
            "character" => {
                for c in &onto.characters.characters {
                    push_valid(&c.annotations, &format!("人物「{}」·", c.name));
                }
            }
            _ => {}
        }
        out
    };
    crate::edits::record_edit_samples(&state, annotation_samples);

    // 蒸馏有效样本为经验（一次 LLM 调用；无样本时直接返回）
    let lessons: Vec<WritingLesson> = crate::edits::distill_pending_lessons_internal(&state).await?;

    Ok(serde_json::json!({
        "applied": true,
        "page": page,
        "lessons": lessons,
        "can_undo": true,
    }))
}

/// 撤回该页面最近一次受控保存（恢复快照）
#[tauri::command]
pub async fn undo_page_change(
    state: tauri::State<'_, AppState>,
    page: String,
) -> Result<serde_json::Value, String> {
    let mut onto = state.ontology.write();
    let idx = onto
        .page_snapshots
        .iter()
        .rposition(|s| s.page == page)
        .ok_or_else(|| "该页面没有可撤回的保存".to_string())?;
    let snap = onto.page_snapshots.remove(idx);
    match page.as_str() {
        "world" => {
            let layer: pensoul_core::WorldLayer =
                serde_json::from_value(snap.before.clone()).map_err(|e| e.to_string())?;
            let frontend = world_to_frontend(&layer);
            onto.world = layer;
            drop(onto);
            state.save().map_err(|e| e.to_string())?;
            Ok(frontend)
        }
        "character" => {
            let layer: pensoul_core::CharacterLayer =
                serde_json::from_value(snap.before.clone()).map_err(|e| e.to_string())?;
            let frontend = characters_to_frontend(&layer);
            onto.characters = layer;
            drop(onto);
            state.save().map_err(|e| e.to_string())?;
            Ok(frontend)
        }
        other => return Err(format!("不支持的页面类型: {other}")),
    }
}

/// 该页面是否有可撤回的受控保存快照
#[tauri::command]
pub async fn page_undo_available(
    state: tauri::State<'_, AppState>,
    page: String,
) -> Result<bool, String> {
    let onto = state.ontology.read();
    Ok(onto.page_snapshots.iter().any(|s| s.page == page))
}

pub(crate) fn extract_block(raw: &str, begin: &str, end: &str) -> String {
    let b = raw.find(begin).map(|i| i + begin.len());
    let e = raw.rfind(end);
    match (b, e) {
        (Some(b), Some(e)) if e > b => raw[b..e].trim().to_string(),
        _ => raw.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_prompt_contains_items() {
        let items = vec![PageItem {
            source: "annotation".to_string(),
            id: "a1".to_string(),
            label: "地点「谷」批注".to_string(),
            content: "描述太抽象".to_string(),
        }];
        let prompt = review_prompt(&items, "world");
        assert!(prompt.contains("地点「谷」批注"));
        assert!(prompt.contains("描述太抽象"));
        assert!(prompt.contains("verdict"));
    }

    #[test]
    fn test_collect_page_items_filters_open_annotations() {
        let onto = pensoul_core::NovelOntology::new(
            pensoul_core::ProjectId::new("p"),
            "t".to_string(),
        );
        let mut onto = onto;
        onto.pending_edit_samples.push(EditSample {
            sample_id: "e1".to_string(),
            scope: "world".to_string(),
            label: "地点「谷」".to_string(),
            before: "a".to_string(),
            after: "b".to_string(),
            chapter_id: None,
            created_at: String::new(),
        });
        onto.pending_edit_samples.push(EditSample {
            sample_id: "e2".to_string(),
            scope: "character".to_string(),
            label: "人物".to_string(),
            before: "x".to_string(),
            after: "y".to_string(),
            chapter_id: None,
            created_at: String::new(),
        });
        let items = collect_page_items(&onto, "world");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "e1");
        assert_eq!(items[0].source, "edit");
    }

    #[test]
    fn test_resolve_annotations_by_verdict() {
        let mut annos = vec![
            ChapterAnnotation {
                annotation_id: "a1".to_string(),
                status: "open".to_string(),
                ..ChapterAnnotation::default()
            },
            ChapterAnnotation {
                annotation_id: "a2".to_string(),
                status: "open".to_string(),
                ..ChapterAnnotation::default()
            },
        ];
        let mut verdicts = std::collections::HashMap::new();
        verdicts.insert("a1".to_string(), "valid".to_string());
        verdicts.insert("a2".to_string(), "invalid".to_string());
        resolve_annotations(&mut annos, &verdicts);
        assert_eq!(annos[0].status, "accepted");
        assert_eq!(annos[0].resolved_by.as_deref(), Some("manual"));
        assert_eq!(annos[1].status, "rejected");
    }

    #[test]
    fn test_parse_world_layer_flat_structure() {
        let v = serde_json::json!({
            "locations": [{"id": "loc-1", "name": "山谷", "description": "幽静"}],
            "timeline_events": [{"event_id": "evt-1", "story_time": "第一年", "description": "大战"}],
            "setting_rules": [{"rule_id": "rule-1", "title": "禁术", "description": "不可用"}],
        });
        let layer = parse_world_layer(&v).unwrap();
        assert_eq!(layer.spatial_model.locations.len(), 1);
        assert_eq!(layer.spatial_model.locations[0].name, "山谷");
        assert_eq!(layer.timeline.events.len(), 1);
        assert_eq!(layer.setting_rules.len(), 1);
    }

    #[test]
    fn test_parse_character_layer_flat_array() {
        let v = serde_json::json!([
            {
                "id": "char-1",
                "name": "林晚",
                "personality_traits": [["冷静", 0.8]],
                "current_mood": "平静",
                "relationships": [{"from": "林晚", "to": "顾沉", "relation_type": "宿敌", "strength": 0.9}]
            }
        ]);
        let layer = parse_character_layer(&v).unwrap();
        assert_eq!(layer.characters.len(), 1);
        assert_eq!(layer.characters[0].core_personality.traits[0].0, "冷静");
        assert_eq!(layer.relationships.len(), 1);
        assert_eq!(layer.relationships[0].relation_type, "宿敌");
    }

    #[test]
    fn test_merge_world_annotations_keeps_batch() {
        use pensoul_core::ChapterAnnotation;
        let anno = ChapterAnnotation {
            annotation_id: "a1".to_string(),
            status: "open".to_string(),
            ..ChapterAnnotation::default()
        };
        let old = pensoul_core::WorldLayer {
            world_id: pensoul_core::WorldId::new("w"),
            name: "世界".to_string(),
            spatial_model: pensoul_core::SpatialModel {
                locations: vec![pensoul_core::Location {
                    id: pensoul_core::LocationId::new("loc-1"),
                    name: "谷".to_string(),
                    description: "幽静".to_string(),
                    spatial_tags: vec![],
                    annotations: vec![anno],
                }],
                hierarchy: vec![],
            },
            timeline: pensoul_core::Timeline {
                events: vec![],
                epoch_markers: vec![],
            },
            setting_rules: vec![],
            glossary: vec![],
            item_graph: vec![],
        };
        let mut new = old.clone();
        new.spatial_model.locations[0].annotations = vec![];
        merge_world_annotations(&old, &mut new);
        assert_eq!(new.spatial_model.locations[0].annotations.len(), 1);
    }
}
