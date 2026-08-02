//! 章节受控保存 —— 笔耕「保存并审核」
//!
//! 与页面受控保存同构：收集本章批注与修改样本 → LLM 判定有效性 + 影响评估
//! → 二次确认 → 快照（revisions）→ 批注流转 → 有效样本沉淀经验。
use crate::commands::json_fix;
use crate::commands::llm_helper as lh;
use crate::edits::{resolve_any_model, chapter_diff_samples, record_edit_samples};
use crate::llm_profile::LlmTask;
use crate::page_review::{PageReview, extract_block, review_prompt};
use crate::state::AppState;
use pensoul_core::{ChapterAnnotation, ChapterId, EditSample, WritingLesson};

fn now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// 收集本章批注（open）+ 本章修改样本 + 当前编辑与落库内容的临时 diff
fn collect_chapter_items(
    onto: &pensoul_core::NovelOntology,
    chapter_id: &ChapterId,
    new_content: &str,
) -> Vec<crate::page_review::PageItem> {
    let mut out = Vec::new();
    if let Some(ch) = onto.get_chapter(chapter_id) {
        for a in &ch.annotations {
            if a.status == "open" {
                out.push(crate::page_review::PageItem {
                    source: "annotation".to_string(),
                    id: a.annotation_id.clone(),
                    label: format!("第 {} 章批注", ch.chapter_no),
                    content: a.content.clone(),
                });
            }
        }
        // 本次编辑 vs 落库内容的修改样本（临时，不落库）
        let id = chapter_id.as_str();
        for s in chapter_diff_samples(ch, &ch.title, &ch.summary, new_content) {
            if s.chapter_id.as_deref() == Some(id) {
                out.push(crate::page_review::PageItem {
                    source: "edit".to_string(),
                    id: s.sample_id,
                    label: s.label,
                    content: format!("改前：{}\n改后：{}", s.before, s.after),
                });
            }
        }
    }
    // 之前积累的本章修改样本
    for s in &onto.pending_edit_samples {
        if s.chapter_id.as_deref() == Some(chapter_id.as_str()) {
            out.push(crate::page_review::PageItem {
                source: "edit".to_string(),
                id: s.sample_id.clone(),
                label: s.label.clone(),
                content: format!("改前：{}\n改后：{}", s.before, s.after),
            });
        }
    }
    out
}

/// 判定本章批注与修改的有效性及对全文的影响（只读）
#[tauri::command]
pub async fn review_chapter_changes(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    content: String,
) -> Result<PageReview, String> {
    lh::ensure_api_keys_loaded(&state);
    let id = ChapterId::new(&chapter_id);
    let (items, label) = {
        let onto = state.ontology.read();
        let ch = onto
            .get_chapter(&id)
            .ok_or_else(|| format!("章节不存在: {chapter_id}"))?;
        (
            collect_chapter_items(&onto, &id, &content),
            format!("第 {} 章《{}》", ch.chapter_no, ch.title),
        )
    };
    if items.is_empty() {
        return Ok(PageReview {
            items: Vec::new(),
            impact: format!("「{label}」没有待处理的批注或修改。"),
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
        &review_prompt(&items, &format!("章节 {label}")),
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

#[derive(serde::Deserialize)]
struct ReviewPayload {
    #[serde(default)]
    items: Vec<crate::page_review::ReviewItem>,
    #[serde(default)]
    impact: String,
}

/// 按最终决定流转本章批注状态
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

/// 确认并应用本章保存：revisions 快照 → 更新正文 → 批注流转 → 有效样本沉淀
#[tauri::command]
pub async fn apply_chapter_review(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    content: String,
    confirmations: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let id = ChapterId::new(&chapter_id);
    let mut verdicts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in &confirmations {
        if let (Some(aid), Some(v)) = (
            c.get("id").and_then(|v| v.as_str()),
            c.get("verdict").and_then(|v| v.as_str()),
        ) {
            verdicts.insert(aid.to_string(), v.to_string());
        }
    }

    let new_version = {
        let mut onto = state.ontology.write();
        let ch = onto
            .chapters
            .iter_mut()
            .find(|c| c.chapter_id == id)
            .ok_or_else(|| format!("章节不存在: {chapter_id}"))?;
        // 快照入版本历史（受控保存前，可回滚）
        ch.revisions.push(pensoul_core::ChapterRevision {
            version: ch.version,
            content: ch.content.clone(),
            word_count: ch.word_count,
            created_at: now(),
            reason: "受控保存前快照".to_string(),
        });
        if ch.revisions.len() > 30 {
            let excess = ch.revisions.len() - 30;
            ch.revisions.drain(..excess);
        }
        resolve_annotations(&mut ch.annotations, &verdicts);
        ch.content = content;
        ch.word_count = ch.content.chars().count() as u32;
        ch.version += 1;
        ch.updated_at = now();
        ch.version
    };
    state.save().map_err(|e| e.to_string())?;
    crate::integration::on_chapter_saved(&state, &id);

    // valid 批注转修改样本（统一进经验蒸馏），invalid 丢弃
    let valid_annos: Vec<EditSample> = {
        let onto = state.ontology.read();
        let ch = onto.get_chapter(&id);
        let mut out = Vec::new();
        if let Some(ch) = ch {
            for a in &ch.annotations {
                if a.status == "accepted"
                    && verdicts.get(&a.annotation_id).map(|v| v.as_str()) == Some("valid")
                {
                    out.push(EditSample {
                        sample_id: format!("edit-{}", uuid::Uuid::new_v4().simple()),
                        scope: "chapter".to_string(),
                        label: format!("第 {} 章批注", ch.chapter_no),
                        before: a.content.clone(),
                        after: "（批注已确认有效，按建议修订）".to_string(),
                        chapter_id: Some(chapter_id.clone()),
                        created_at: now(),
                    });
                }
            }
        }
        out
    };
    record_edit_samples(&state, valid_annos);

    // 蒸馏有效样本为经验（一次 LLM 调用）
    let lessons: Vec<WritingLesson> = crate::edits::distill_pending_lessons_internal(&state).await?;

    Ok(serde_json::json!({
        "new_version": new_version,
        "lessons": lessons,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_chapter_items_filters_by_chapter() {
        let mut onto = pensoul_core::NovelOntology::new(
            pensoul_core::ProjectId::new("p"),
            "t".to_string(),
        );
        onto.chapters.push(pensoul_core::Chapter {
            chapter_id: ChapterId::new("ch-1"),
            chapter_no: 1,
            volume_id: pensoul_core::VolumeId::new("v"),
            title: "第一章".to_string(),
            summary: "梗概".to_string(),
            content: "旧正文".to_string(),
            word_count: 3,
            version: 1,
            status: pensoul_core::ChapterStatus::Draft,
            consistency_score: 0.0,
            created_at: String::new(),
            updated_at: String::new(),
            annotations: vec![ChapterAnnotation {
                annotation_id: "a1".to_string(),
                status: "open".to_string(),
                ..ChapterAnnotation::default()
            }],
            revisions: Vec::new(),
        });
        let id = ChapterId::new("ch-1");
        let items = collect_chapter_items(&onto, &id, "新正文内容");
        // 1 条批注 + 1 条本次编辑 diff（正文变化）
        assert!(items.iter().any(|i| i.source == "annotation"));
        assert!(items.iter().any(|i| i.source == "edit"));
    }

    #[test]
    fn test_resolve_chapter_annotations() {
        let mut annos = vec![ChapterAnnotation {
            annotation_id: "a1".to_string(),
            status: "open".to_string(),
            ..ChapterAnnotation::default()
        }];
        let mut verdicts = std::collections::HashMap::new();
        verdicts.insert("a1".to_string(), "valid".to_string());
        resolve_annotations(&mut annos, &verdicts);
        assert_eq!(annos[0].status, "accepted");
        assert_eq!(annos[0].resolved_by.as_deref(), Some("manual"));
    }
}
