// blueprint.rs — 蓝图管理 API

use axum::extract::State;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;

/// 获取蓝图概览
pub async fn get_blueprint(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let bp = &ontology.blueprint;
    let result = serde_json::json!({
        "settled": bp.settled,
        "settled_at": bp.settled_at,
        "commitment_count": bp.commitments.len(),
        "volume_count": bp.volumes.len(),
        "character_count": bp.character_matrix.len(),
        "foreshadow_count": bp.foreshadows.len(),
        "subplot_count": bp.subplots.len(),
        "resource_count": bp.resources.len(),
        "commitments": bp.commitments.iter().map(|c| {
            serde_json::json!({
                "id": c.commitment_id,
                "statement": c.statement,
                "kind": c.kind,
                "priority": c.priority,
                "status": c.status,
            })
        }).collect::<Vec<_>>(),
        "volumes": bp.volumes.iter().map(|v| {
            serde_json::json!({
                "volume_no": v.volume_no,
                "title": v.title,
                "one_line": v.one_line,
                "function": v.function,
                "chapter_start": v.chapter_start,
                "chapter_end": v.chapter_end,
                "status": v.status,
            })
        }).collect::<Vec<_>>(),
    });

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}
