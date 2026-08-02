//! 全链路批注命令 —— 覆盖正文、细纲、脉络节点、人物志、世界观
//!
//! target 定位串约定：`<kind>:<id>[:field]`，如：
//! - 章节正文：`chapter:ch-1:body`
//! - 章节细纲：`chapter:ch-1:summary`
//! - 脉络节点：`outline_arc:arc-1`
//! - 人物：`character:char-1[:name]`
//! - 地点：`location:loc-1[:description]`
//! - 时间线事件：`timeline:evt-1[:description]`
//! - 设定规则：`rule:rule-1[:description]`
//! - 术语：`glossary:term-1[:definition]`
use crate::state::AppState;
use pensoul_core::{ChapterAnnotation, ChapterId};

/// 解析后的定位目标
struct ParsedTarget {
    kind: String,
    id: String,
    field: Option<String>,
}

fn parse_target(target: &str) -> Result<ParsedTarget, String> {
    let mut parts = target.splitn(3, ':');
    let kind = parts.next().unwrap_or("").to_string();
    let id = parts.next().unwrap_or("").to_string();
    let field = parts.next().filter(|f| !f.is_empty()).map(|f| f.to_string());
    if kind.is_empty() || id.is_empty() {
        return Err(format!("无效的批注定位: {target}，应为 <类型>:<ID>[:字段]"));
    }
    Ok(ParsedTarget { kind, id, field })
}

/// 定位实体并返回其批注列表的可变引用与当前字段文本快照
fn locate_mut<'o>(
    onto: &'o mut pensoul_core::NovelOntology,
    t: &ParsedTarget,
) -> Result<(&'o mut Vec<ChapterAnnotation>, Option<String>), String> {
    match t.kind.as_str() {
        "chapter" => {
            let id = ChapterId::new(&t.id);
            let ch = onto
                .chapters
                .iter_mut()
                .find(|c| c.chapter_id == id)
                .ok_or_else(|| format!("章节不存在: {}", t.id))?;
            let snapshot = match t.field.as_deref() {
                Some("summary") => Some(ch.summary.clone()),
                _ => Some(ch.content.clone()),
            };
            Ok((&mut ch.annotations, snapshot))
        }
        "outline_arc" => {
            let arc = onto
                .outline_arcs
                .iter_mut()
                .find(|a| a.arc_id == t.id)
                .ok_or_else(|| format!("脉络节点不存在: {}", t.id))?;
            Ok((&mut arc.annotations, Some(arc.description.clone())))
        }
        "character" => {
            let ch = onto
                .characters
                .characters
                .iter_mut()
                .find(|c| c.id.as_str() == t.id)
                .ok_or_else(|| format!("角色不存在: {}", t.id))?;
            Ok((&mut ch.annotations, Some(ch.name.clone())))
        }
        "location" => {
            let loc = onto
                .world
                .spatial_model
                .locations
                .iter_mut()
                .find(|l| l.id.as_str() == t.id)
                .ok_or_else(|| format!("地点不存在: {}", t.id))?;
            let snapshot = match t.field.as_deref() {
                Some("description") => Some(loc.description.clone()),
                _ => Some(loc.name.clone()),
            };
            Ok((&mut loc.annotations, snapshot))
        }
        "timeline" => {
            let ev = onto
                .world
                .timeline
                .events
                .iter_mut()
                .find(|e| e.event_id.as_str() == t.id)
                .ok_or_else(|| format!("时间线事件不存在: {}", t.id))?;
            let snapshot = match t.field.as_deref() {
                Some("description") => Some(ev.description.clone()),
                _ => Some(ev.story_time.clone()),
            };
            Ok((&mut ev.annotations, snapshot))
        }
        "rule" => {
            let r = onto
                .world
                .setting_rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == t.id)
                .ok_or_else(|| format!("设定规则不存在: {}", t.id))?;
            let snapshot = match t.field.as_deref() {
                Some("description") => Some(r.description.clone()),
                _ => Some(r.title.clone()),
            };
            Ok((&mut r.annotations, snapshot))
        }
        "glossary" => {
            let term = onto
                .world
                .glossary
                .iter_mut()
                .find(|g| g.term == t.id)
                .ok_or_else(|| format!("术语不存在: {}", t.id))?;
            Ok((&mut term.annotations, Some(term.definition.clone())))
        }
        other => Err(format!("不支持的批注目标类型: {other}")),
    }
}

/// 只读定位批注列表
fn locate_ref<'o>(
    onto: &'o pensoul_core::NovelOntology,
    t: &ParsedTarget,
) -> Result<&'o Vec<ChapterAnnotation>, String> {
    match t.kind.as_str() {
        "chapter" => {
            let id = ChapterId::new(&t.id);
            onto.chapters
                .iter()
                .find(|c| c.chapter_id == id)
                .map(|c| &c.annotations)
                .ok_or_else(|| format!("章节不存在: {}", t.id))
        }
        "outline_arc" => onto
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == t.id)
            .map(|a| &a.annotations)
            .ok_or_else(|| format!("脉络节点不存在: {}", t.id)),
        "character" => onto
            .characters
            .characters
            .iter()
            .find(|c| c.id.as_str() == t.id)
            .map(|c| &c.annotations)
            .ok_or_else(|| format!("角色不存在: {}", t.id)),
        "location" => onto
            .world
            .spatial_model
            .locations
            .iter()
            .find(|l| l.id.as_str() == t.id)
            .map(|l| &l.annotations)
            .ok_or_else(|| format!("地点不存在: {}", t.id)),
        "timeline" => onto
            .world
            .timeline
            .events
            .iter()
            .find(|e| e.event_id.as_str() == t.id)
            .map(|e| &e.annotations)
            .ok_or_else(|| format!("时间线事件不存在: {}", t.id)),
        "rule" => onto
            .world
            .setting_rules
            .iter()
            .find(|r| r.rule_id.as_str() == t.id)
            .map(|r| &r.annotations)
            .ok_or_else(|| format!("设定规则不存在: {}", t.id)),
        "glossary" => onto
            .world
            .glossary
            .iter()
            .find(|g| g.term == t.id)
            .map(|g| &g.annotations)
            .ok_or_else(|| format!("术语不存在: {}", t.id)),
        other => Err(format!("不支持的批注目标类型: {other}")),
    }
}

fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// 添加批注
#[tauri::command]
pub async fn annotation_add(
    state: tauri::State<'_, AppState>,
    target: String,
    kind: String,
    content: String,
    anchor: Option<pensoul_core::AnnotationAnchor>,
) -> Result<ChapterAnnotation, String> {
    let t = parse_target(&target)?;
    if !["issue", "suggestion", "note"].contains(&kind.as_str()) {
        return Err(format!("无效的批注类型: {kind}"));
    }
    if content.trim().is_empty() {
        return Err("批注内容不能为空".to_string());
    }
    let mut onto = state.ontology.write();
    let (annos, snapshot) = locate_mut(&mut onto, &t)?;
    let anchor_snapshot = anchor
        .as_ref()
        .and_then(|a| {
            if !a.text.is_empty() {
                Some(a.text.clone())
            } else {
                snapshot.clone()
            }
        })
        .or(snapshot);
    let anno = ChapterAnnotation {
        annotation_id: format!("anno-{}", uuid::Uuid::new_v4().simple()),
        kind,
        anchor,
        content: content.trim().to_string(),
        status: "open".to_string(),
        created_at: now(),
        processed_in_version: 0,
        target: Some(target.clone()),
        resolved_by: None,
        anchor_snapshot,
        resolved_at: None,
    };
    annos.push(anno.clone());
    drop(onto);
    state.save().map_err(|e| e.to_string())?;
    Ok(anno)
}

/// 更新批注（内容 / 类型）
#[tauri::command]
pub async fn annotation_update(
    state: tauri::State<'_, AppState>,
    target: String,
    annotation_id: String,
    patch: serde_json::Value,
) -> Result<(), String> {
    let t = parse_target(&target)?;
    let mut onto = state.ontology.write();
    let (annos, _) = locate_mut(&mut onto, &t)?;
    let anno = annos
        .iter_mut()
        .find(|a| a.annotation_id == annotation_id)
        .ok_or_else(|| format!("批注不存在: {annotation_id}"))?;
    if let Some(kind) = patch
        .get("kind")
        .and_then(|v| v.as_str())
        .filter(|k| ["issue", "suggestion", "note"].contains(k))
    {
        anno.kind = kind.to_string();
    }
    if let Some(content) = patch
        .get("content")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        anno.content = content.to_string();
    }
    if let Some(status) = patch
        .get("status")
        .and_then(|v| v.as_str())
        .filter(|s| ["open", "accepted", "rejected"].contains(s))
    {
        anno.status = status.to_string();
        if status == "open" {
            // 重开：清除判决记录
            anno.resolved_by = None;
            anno.resolved_at = None;
        } else {
            if anno.resolved_by.is_none() {
                anno.resolved_by = Some("manual".to_string());
            }
            if anno.resolved_at.is_none() {
                anno.resolved_at = Some(now());
            }
        }
    }
    drop(onto);
    state.save().map_err(|e| e.to_string())
}

/// 删除批注
#[tauri::command]
pub async fn annotation_remove(
    state: tauri::State<'_, AppState>,
    target: String,
    annotation_id: String,
) -> Result<(), String> {
    let t = parse_target(&target)?;
    let mut onto = state.ontology.write();
    let (annos, _) = locate_mut(&mut onto, &t)?;
    let before = annos.len();
    annos.retain(|a| a.annotation_id != annotation_id);
    if annos.len() == before {
        return Err(format!("批注不存在: {annotation_id}"));
    }
    drop(onto);
    state.save().map_err(|e| e.to_string())
}

/// 逐条处理批注（accept / reject），判决来源标记为 manual
#[tauri::command]
pub async fn annotation_resolve(
    state: tauri::State<'_, AppState>,
    target: String,
    decisions: Vec<serde_json::Value>,
) -> Result<Vec<ChapterAnnotation>, String> {
    let t = parse_target(&target)?;
    let mut onto = state.ontology.write();
    let (annos, _) = locate_mut(&mut onto, &t)?;
    let ts = now();
    let mut updated = Vec::new();
    for d in &decisions {
        let id = d
            .get("annotation_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let accept = d.get("accept").and_then(|v| v.as_bool()).unwrap_or(false);
        if let Some(anno) = annos.iter_mut().find(|a| a.annotation_id == id) {
            if anno.status == "open" {
                anno.status = if accept { "accepted" } else { "rejected" }.to_string();
                anno.resolved_by = Some("manual".to_string());
                anno.resolved_at = Some(ts.clone());
            }
            updated.push(anno.clone());
        }
    }
    drop(onto);
    state.save().map_err(|e| e.to_string())?;
    Ok(updated)
}

/// 列出某目标的批注
#[tauri::command]
pub async fn annotations_list(
    state: tauri::State<'_, AppState>,
    target: String,
) -> Result<Vec<ChapterAnnotation>, String> {
    let t = parse_target(&target)?;
    let onto = state.ontology.read();
    let annos = locate_ref(&onto, &t)?;
    Ok(annos.clone())
}

/// 聚合收件箱：所有实体的批注按目标分组
#[tauri::command]
pub async fn annotations_all(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let onto = state.ontology.read();
    let mut out: Vec<serde_json::Value> = Vec::new();
    for ch in &onto.chapters {
        if !ch.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("chapter:{}:body", ch.chapter_id),
                "label": format!("第 {} 章《{}》正文", ch.chapter_no, ch.title),
                "annotations": ch.annotations,
            }));
        }
    }
    for arc in &onto.outline_arcs {
        if !arc.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("outline_arc:{}", arc.arc_id),
                "label": format!("脉络节点《{}》", arc.title),
                "annotations": arc.annotations,
            }));
        }
    }
    for c in &onto.characters.characters {
        if !c.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("character:{}", c.id),
                "label": format!("人物「{}」", c.name),
                "annotations": c.annotations,
            }));
        }
    }
    for l in &onto.world.spatial_model.locations {
        if !l.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("location:{}", l.id),
                "label": format!("地点「{}」", l.name),
                "annotations": l.annotations,
            }));
        }
    }
    for e in &onto.world.timeline.events {
        if !e.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("timeline:{}", e.event_id),
                "label": format!("时间线事件「{}」", e.story_time),
                "annotations": e.annotations,
            }));
        }
    }
    for r in &onto.world.setting_rules {
        if !r.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("rule:{}", r.rule_id),
                "label": format!("设定规则《{}》", r.title),
                "annotations": r.annotations,
            }));
        }
    }
    for g in &onto.world.glossary {
        if !g.annotations.is_empty() {
            out.push(serde_json::json!({
                "target": format!("glossary:{}", g.term),
                "label": format!("术语「{}」", g.term),
                "annotations": g.annotations,
            }));
        }
    }
    Ok(out)
}

/// 导出标注集（JSONL）：只含已处理批注（accepted/rejected），过滤 open 与 note
#[tauri::command]
pub async fn annotations_export(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let groups = annotations_all(state).await?;
    let mut lines: Vec<String> = Vec::new();
    for g in &groups {
        let target = g.get("target").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(annos) = g.get("annotations").and_then(|v| v.as_array()) {
            for a in annos {
                let kind = a.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let status = a.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if kind == "note" || !["accepted", "rejected"].contains(&status) {
                    continue;
                }
                let snapshot = a.get("anchor_snapshot").and_then(|v| v.as_str());
                let resolved_by = a.get("resolved_by").and_then(|v| v.as_str());
                let content = a.get("content").and_then(|v| v.as_str()).unwrap_or("");
                lines.push(serde_json::json!({
                    "target": target,
                    "snapshot": snapshot,
                    "content": content,
                    "decision": status,
                    "resolved_by": resolved_by,
                })
                .to_string());
            }
        }
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::{Chapter, ChapterStatus, NovelOntology, ProjectId, VolumeId};

    fn empty_ontology() -> NovelOntology {
        NovelOntology::new(ProjectId::new("p-test"), "测试项目".to_string())
    }

    fn make_chapter(id: &str, title: &str) -> Chapter {
        Chapter {
            chapter_id: ChapterId::new(id),
            chapter_no: 1,
            volume_id: VolumeId::new("v1"),
            title: title.to_string(),
            summary: "本章梗概".to_string(),
            content: "本章正文".to_string(),
            word_count: 4,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 0.0,
            created_at: String::new(),
            updated_at: String::new(),
            annotations: Vec::new(),
            revisions: Vec::new(),
        }
    }

    #[test]
    fn test_parse_target_formats() {
        let t = parse_target("chapter:ch-1:summary").unwrap();
        assert_eq!(t.kind, "chapter");
        assert_eq!(t.id, "ch-1");
        assert_eq!(t.field.as_deref(), Some("summary"));

        let t = parse_target("character:char-1").unwrap();
        assert_eq!(t.kind, "character");
        assert!(t.field.is_none());

        assert!(parse_target("").is_err());
        assert!(parse_target("chapter:").is_err());
    }

    #[test]
    fn test_locate_ref_unknown_entity_errors() {
        let onto = empty_ontology();
        let t = parse_target("location:loc-x").unwrap();
        assert!(locate_ref(&onto, &t).is_err());
    }

    #[test]
    fn test_locate_ref_finds_chapter_annotations() {
        let mut onto = empty_ontology();
        onto.chapters.push(make_chapter("ch-1", "第一章"));
        let t = parse_target("chapter:ch-1:body").unwrap();
        let annos = locate_ref(&onto, &t).unwrap();
        assert!(annos.is_empty());

        // 可变路径：添加批注
        let (annos_mut, snapshot) = locate_mut(&mut onto, &t).unwrap();
        assert_eq!(snapshot.as_deref(), Some("本章正文"));
        annos_mut.push(ChapterAnnotation {
            annotation_id: "anno-1".to_string(),
            kind: "issue".to_string(),
            anchor: None,
            content: "节奏偏慢".to_string(),
            status: "open".to_string(),
            created_at: String::new(),
            processed_in_version: 0,
            target: Some(format!("{}:{}", t.kind, t.id)),
            resolved_by: None,
            anchor_snapshot: None,
            resolved_at: None,
        });
        assert_eq!(locate_ref(&onto, &t).unwrap().len(), 1);
    }

}
