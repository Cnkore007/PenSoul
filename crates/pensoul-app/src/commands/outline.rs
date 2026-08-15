// outline.rs — 大纲管理 API

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::chapter::ChapterStatus;
use pensoul_domain::narrative::OutlineArc;

#[derive(Deserialize)]
pub struct CreateArcParams {
    pub title: String,
    pub chapter_start: i64,
    pub chapter_end: i64,
}

#[derive(Deserialize)]
pub struct CreateChapterParams {
    pub title: String,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub arc_id: Option<String>,
    pub chapter_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GetChapterContentParams {
    pub chapter_id: String,
}

#[derive(Deserialize, Default)]
pub struct UpdateArcParams {
    pub arc_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub chapter_start: Option<String>,
    pub chapter_end: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct UpdateChapterParams {
    pub chapter_id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveContentParams {
    pub chapter_id: String,
    pub content: String,
}

/// 获取所有大纲脉络
pub async fn list_arcs(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .outline_arcs
        .iter()
        .map(|a| {
            serde_json::json!({
                "arc_id": a.arc_id,
                "title": a.title,
                "description": a.description,
                "chapter_start": a.chapter_start,
                "chapter_end": a.chapter_end,
                "chapter_count": a.chapter_count(),
                "expanded_until": a.expanded_until,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 创建大纲脉络（校验章节范围）
pub async fn create_arc(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CreateArcParams>,
) -> Result<String, ApiError> {
    if params.chapter_start < 1 || params.chapter_end < params.chapter_start {
        return Err(ApiError::bad_request(
            "脉络章节范围非法（起始章 >= 1 且结束章 >= 起始章）",
        ));
    }
    let title = params.title.trim();
    if title.is_empty() {
        return Err(ApiError::bad_request("脉络标题不能为空"));
    }

    let arc = OutlineArc::new(title, params.chapter_start, params.chapter_end);
    let id = arc.arc_id.clone();

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.outline_arcs.push(arc);
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 获取所有章节
pub async fn list_chapters(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .chapters_in_order()
        .iter()
        .map(|c| {
            serde_json::json!({
                "chapter_id": c.chapter_id.to_string(),
                "chapter_no": c.chapter_no,
                "title": c.title,
                "summary": c.summary,
                "word_count": c.word_count,
                "status": format!("{:?}", c.status),
                "version": c.version,
                "consistency_score": c.consistency_score,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 创建章节（章节号取当前最大值 + 1，避免删除后重复）
pub async fn create_chapter(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CreateChapterParams>,
) -> Result<String, ApiError> {
    let title = params.title.trim();
    if title.is_empty() {
        return Err(ApiError::bad_request("章节标题不能为空"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let chapter_no = ontology
        .chapters
        .iter()
        .map(|c| c.chapter_no)
        .max()
        .unwrap_or(0)
        + 1;
    let chapter = pensoul_domain::chapter::Chapter::new(chapter_no, title);
    let id = chapter.chapter_id.to_string();

    ontology.chapters.push(chapter);
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 更新大纲脉络
pub async fn update_arc(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateArcParams>,
) -> Result<String, ApiError> {
    let chapter_start = crate::commands::params::parse_optional_i64(
        params.chapter_start.clone(),
        "chapter_start",
    )?;
    let chapter_end =
        crate::commands::params::parse_optional_i64(params.chapter_end.clone(), "chapter_end")?;
    if let (Some(start), Some(end)) = (chapter_start, chapter_end) {
        if start < 1 || end < start {
            return Err(ApiError::bad_request(
                "脉络章节范围非法（起始章 >= 1 且结束章 >= 起始章）",
            ));
        }
    }
    if let Some(title) = &params.title {
        if title.trim().is_empty() {
            return Err(ApiError::bad_request("脉络标题不能为空"));
        }
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let Some(arc) = ontology.outline_arcs.iter_mut().find(|a| a.arc_id == params.arc_id) else {
        return Err(ApiError::not_found("脉络不存在"));
    };
    if let Some(title) = params.title {
        arc.title = title;
    }
    if let Some(desc) = params.description {
        arc.description = desc;
    }
    if let Some(start) = chapter_start {
        arc.chapter_start = start;
    }
    if let Some(end) = chapter_end {
        arc.chapter_end = end;
    }
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除大纲脉络
pub async fn delete_arc(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let arc_id = params.arc_id.ok_or(ApiError::bad_request("缺少 arc_id"))?;
    let len_before = ontology.outline_arcs.len();
    ontology.outline_arcs.retain(|a| a.arc_id != arc_id);
    if ontology.outline_arcs.len() == len_before {
        return Err(ApiError::not_found("脉络不存在"));
    }

    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 更新章节（状态机门控 + 非空校验）
pub async fn update_chapter(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateChapterParams>,
) -> Result<String, ApiError> {
    if let Some(title) = &params.title {
        if title.trim().is_empty() {
            return Err(ApiError::bad_request("章节标题不能为空"));
        }
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let Some(chapter) = ontology
        .chapters
        .iter_mut()
        .find(|c| c.chapter_id.to_string() == params.chapter_id)
    else {
        return Err(ApiError::not_found("章节不存在"));
    };

    if let Some(title) = params.title {
        chapter.title = title;
    }
    if let Some(summary) = params.summary {
        chapter.summary = summary;
    }
    if let Some(status_str) = params.status {
        let next_status = match status_str.as_str() {
            "Draft" => ChapterStatus::Draft,
            "Reviewing" => ChapterStatus::Reviewing,
            "Reviewed" => ChapterStatus::Reviewed,
            "Polished" => ChapterStatus::Polished,
            "Published" => ChapterStatus::Published,
            _ => return Err(ApiError::bad_request(format!("未知章节状态: {status_str}"))),
        };
        if !chapter.status.can_transition_to(&next_status) {
            return Err(ApiError::bad_request(format!(
                "章节状态不能从 {:?} 直接变为 {:?}（流程：草稿 → 审阅中 → 已审阅 → 已润色 → 已发布，或回退到草稿）",
                chapter.status, next_status
            )));
        }
        chapter.status = next_status;
    }
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除章节（保留编号，不静默重排；新建章节号取最大值 + 1）
pub async fn delete_chapter(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let chapter_id = params
        .chapter_id
        .ok_or(ApiError::bad_request("缺少 chapter_id"))?;
    let len_before = ontology.chapters.len();
    ontology
        .chapters
        .retain(|c| c.chapter_id.to_string() != chapter_id);
    if ontology.chapters.len() == len_before {
        return Err(ApiError::not_found("章节不存在"));
    }

    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 保存章节内容：走 update_content（版本+修订历史），然后过 on_chapter_saved 集成层
pub async fn save_chapter_content(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<SaveContentParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    {
        let ontology = state
            .ontology
            .as_mut()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let Some(chapter) = ontology
            .chapters
            .iter_mut()
            .find(|c| c.chapter_id.to_string() == params.chapter_id)
        else {
            return Err(ApiError::not_found("章节不存在"));
        };
        chapter.update_content(params.content);
    }

    state.rebuild_derived();
    state.on_chapter_saved(&params.chapter_id);
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 获取章节正文与修订历史
pub async fn get_chapter_content(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<GetChapterContentParams>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let Some(chapter) = ontology
        .chapters
        .iter()
        .find(|c| c.chapter_id.to_string() == params.chapter_id)
    else {
        return Err(ApiError::not_found("章节不存在"));
    };

    let result = serde_json::json!({
        "chapter_id": chapter.chapter_id.to_string(),
        "content": chapter.content,
        "word_count": chapter.word_count,
        "version": chapter.version,
        "consistency_score": chapter.consistency_score,
        "annotations": chapter.annotations,
        "revision_count": chapter.revisions.len(),
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}
