// graph.rs — 图谱查询 API

use axum::extract::{Form, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::EntityRef;
use pensoul_domain::entity::EntityType;

#[derive(Deserialize)]
pub struct PredictImpactParams {
    pub entity_id: String,
    pub entity_type: String,
    pub max_depth: u32,
}

/// 获取图谱统计
pub async fn graph_stats(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let stats = state.graph.stats();
    serde_json::to_string(&stats).map_err(|e| ApiError::internal(e.to_string()))
}

/// 预测影响（走 POST：调用 LLM 产生外部成本，属副作用操作）
pub async fn predict_impact(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<PredictImpactParams>,
) -> Result<String, ApiError> {
    let state = state.read().await;

    // 类型解析失败显式报错，禁止静默映射
    let et = match params.entity_type.as_str() {
        "Character" => EntityType::Character,
        "Event" => EntityType::Event,
        "Setting" => EntityType::Setting,
        "Foreshadow" => EntityType::Foreshadow,
        _ => {
            return Err(ApiError::bad_request(format!(
                "未知实体类型: {}",
                params.entity_type
            )))
        }
    };

    let entity_ref = EntityRef::new(et, &params.entity_id);
    if state.graph.get_entity(&params.entity_id).is_none() {
        return Err(ApiError::not_found("实体不存在"));
    }
    let predictions = state.graph.predict_impact(&entity_ref, params.max_depth);

    let result: Vec<serde_json::Value> = predictions
        .iter()
        .map(|p| {
            serde_json::json!({
                "entity_id": p.entity.entity_id,
                "entity_name": p.entity.label,
                "severity": format!("{:?}", p.severity),
                "distance": p.distance,
                "reason": p.reason,
                "suggested_action": p.suggested_action,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 执行约束检查
pub async fn check_constraints(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let report = state.constraints.full_audit();
    let result = serde_json::json!({
        "checked_entities": report.checked_entities,
        "has_issues": report.has_issues(),
        "error_count": report.error_count(),
        "warning_count": report.warning_count(),
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}
