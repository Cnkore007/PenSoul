// memory.rs — 动态记忆检索 API

use axum::extract::{Form, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::{EntityRef, EntityType};
use pensoul_memory::types::{EditingMode, RetrievalContext, WritingIntent};

#[derive(Deserialize, Default)]
pub struct RetrieveMemoryParams {
    pub current_chapter: i64,
    pub editing_mode: Option<String>,
    /// 逗号分隔的实体 ID 列表
    pub involved_entities: Option<String>,
}

/// 根据当前上下文检索记忆包
pub async fn retrieve_memory(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<RetrieveMemoryParams>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    if state.ontology.is_none() {
        return Err(ApiError::bad_request("没有打开的项目"));
    }
    if params.current_chapter < 1 {
        return Err(ApiError::bad_request("当前章节号必须 >= 1"));
    }

    let editing_mode = match params.editing_mode.as_deref() {
        Some("Drafting") | None => EditingMode::Drafting,
        Some("Revising") => EditingMode::Revising,
        Some("Reviewing") => EditingMode::Reviewing,
        Some(other) => return Err(ApiError::bad_request(format!("未知编辑模式: {other}"))),
    };

    let involved_entities = params
        .involved_entities
        .as_deref()
        .map(|ids| {
            ids.split(',')
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .map(|id| EntityRef::new(EntityType::Character, id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let context = RetrievalContext {
        current_chapter: params.current_chapter,
        cursor_position: None,
        editing_mode,
        involved_entities,
        intent: WritingIntent::NewContent,
    };

    let packet = state.memory.retrieve(&context);
    serde_json::to_string(&packet).map_err(|e| ApiError::internal(e.to_string()))
}
