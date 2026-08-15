// dashboard.rs — 仪表盘 API

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;

/// 获取项目概览统计
pub async fn project_overview(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let character_count = ontology.characters.characters.len();
    let setting_count = ontology.world.locations.len();
    let event_count = ontology.world.timeline.len();
    let foreshadow_count = ontology.narrative.foreshadows.len();
    let chapter_count = ontology.chapters.len();
    let outline_count = ontology.outline_arcs.len();
    let total_words = ontology.chapters.iter().map(|c| c.word_count as u64).sum::<u64>();

    // 工业化流水线门禁：每个阶段都给出「是否可进入」与下一动作，
    // 让用户不必猜「现在该点哪里」。
    let detailed_chapters = ontology
        .chapters
        .iter()
        .filter(|c| !c.summary.trim().is_empty())
        .count();
    let writable_chapters = ontology
        .chapters
        .iter()
        .filter(|c| !c.summary.trim().is_empty() && c.word_count == 0)
        .count();
    let planned_chapters = ontology
        .outline_arcs
        .iter()
        .map(|a| a.chapter_end)
        .max()
        .unwrap_or(0);
    let reviewable_chapters = ontology
        .chapters
        .iter()
        .filter(|c| c.word_count > 0)
        .count();

    let concept_ready = !ontology.core_concept.high_concept.trim().is_empty()
        && !ontology.core_concept.premise.trim().is_empty()
        && !ontology.core_concept.central_conflict.trim().is_empty();
    let characters_ready = character_count > 0;
    let world_ready = !ontology.world.rules.is_empty() && setting_count > 0;
    let outline_ready = outline_count > 0;
    let detail_ready = detailed_chapters > 0;
    let writing_ready = writable_chapters > 0;

    let stages = serde_json::json!([
        {
            "id": "concept",
            "label": "萌芽定盘",
            "ready": concept_ready,
            "detail": if concept_ready {
                "高概念 / 前提 / 核心冲突已明确".to_string()
            } else {
                "先到「萌芽」完成诘问并应用提案".to_string()
            },
        },
        {
            "id": "characters",
            "label": "人物档案",
            "ready": characters_ready,
            "detail": format!("{character_count} 位角色（可在图谱手动维护，或保存章节后自动提取）"),
        },
        {
            "id": "world",
            "label": "世界观档案",
            "ready": world_ready,
            "detail": format!("{setting_count} 处地点 · {} 条规则", ontology.world.rules.len()),
        },
        {
            "id": "outline",
            "label": "大纲脉络",
            "ready": outline_ready,
            "detail": if outline_ready {
                format!("{outline_count} 条脉络 · 规划至第 {planned_chapters} 章")
            } else {
                "到「萌芽」生成提案，或在大纲页手动创建脉络".to_string()
            },
        },
        {
            "id": "detail",
            "label": "章节细纲",
            "ready": detail_ready,
            "detail": format!("已有细纲 {detailed_chapters} 章 · 可写空章 {writable_chapters} 章"),
        },
        {
            "id": "writing",
            "label": "批量写作",
            "ready": writing_ready,
            "detail": if writing_ready {
                format!("{writable_chapters} 个空章已具备细纲，可进入笔耕分批写作")
            } else {
                "先完成细纲化并导入笔耕".to_string()
            },
        },
        {
            "id": "review",
            "label": "审校发布",
            "ready": reviewable_chapters > 0,
            "detail": format!("{reviewable_chapters} 章有正文，可审校 / 改写 / 级联同步"),
        },
    ]);

    let next_action = if !concept_ready {
        "下一步：萌芽 → 开始诘问 → 生成并应用提案"
    } else if !characters_ready {
        "下一步：图谱建立人物档案（或先写第 1 章后自动提取）"
    } else if !world_ready {
        "下一步：图谱补充世界观地点与规则"
    } else if !outline_ready {
        "下一步：萌芽应用大纲脉络，或在大纲页手动创建"
    } else if !detail_ready {
        "下一步：大纲 → 选择脉络 → 生成细纲 → 导入笔耕"
    } else if !writing_ready {
        "下一步：确认所有空章都有细纲后进入笔耕"
    } else {
        "下一步：笔耕 → 批量写作（每 3 章检查点）"
    };

    let result = serde_json::json!({
        "title": ontology.title,
        "description": ontology.description,
        "character_count": character_count,
        "event_count": event_count,
        "setting_count": setting_count,
        "foreshadow_count": foreshadow_count,
        "chapter_count": chapter_count,
        "volume_count": ontology.volumes.len(),
        "outline_count": outline_count,
        "total_words": total_words,
        "high_concept": ontology.core_concept.high_concept,
        "tone": ontology.core_concept.tone,
        "pipeline": {
            "stages": stages,
            "next_action": next_action,
        },
    });

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}
