// project.rs — 项目管理 API

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::id::ProjectId;
use pensoul_domain::ontology::NovelOntology;
use pensoul_infra::persistence::project::is_valid_project_id;

#[derive(Deserialize)]
pub struct CreateProjectParams {
    pub project_id: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct OpenProjectParams {
    pub project_id: String,
}

#[derive(Deserialize)]
pub struct DeleteProjectParams {
    pub project_id: String,
}

/// 创建新项目（已存在则拒绝，避免静默覆盖）
pub async fn create_project(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CreateProjectParams>,
) -> Result<String, ApiError> {
    if !is_valid_project_id(&params.project_id) {
        return Err(ApiError::bad_request(
            "非法项目 ID（仅允许字母、数字、下划线、连字符，长度 1-64）",
        ));
    }

    let state = state.read().await;
    let store = pensoul_infra::persistence::ProjectStore::new(&state.base_dir);
    let store_path = std::path::Path::new(&state.base_dir)
        .join("projects")
        .join(&params.project_id);
    if store_path.join("pensoul-project.json").exists() {
        return Err(ApiError::conflict(format!(
            "项目 {} 已存在，不能重复创建",
            params.project_id
        )));
    }

    let ontology = NovelOntology::new(ProjectId::new(&params.project_id), &params.title);
    store
        .save(&ontology)
        .map_err(|e| ApiError::internal(format!("保存项目失败: {e}")))?;

    Ok("ok".to_string())
}

/// 打开项目
pub async fn open_project(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<OpenProjectParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    state
        .load_project(&params.project_id)
        .map_err(ApiError::not_found)?;
    // 记住上次打开的项目：后端重启后自动恢复，避免「没有打开的项目」
    crate::state::save_last_project_id(&state.base_dir, &params.project_id)
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 列出所有项目
pub async fn list_projects(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let store = pensoul_infra::persistence::ProjectStore::new(&state.base_dir);
    let projects = store.list_projects();
    serde_json::to_string(&projects).map_err(|e| ApiError::internal(e.to_string()))
}

/// 保存当前项目
pub async fn save_project(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    state
        .save_project()
        .map_err(ApiError::bad_request)?;
    Ok("ok".to_string())
}

/// 删除项目（ID 校验后路径安全；删除后关闭当前项目状态）
pub async fn delete_project(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteProjectParams>,
) -> Result<String, ApiError> {
    if !is_valid_project_id(&params.project_id) {
        return Err(ApiError::bad_request(
            "非法项目 ID（仅允许字母、数字、下划线、连字符）",
        ));
    }

    let mut state = state.write().await;
    let project_dir = std::path::Path::new(&state.base_dir)
        .join("projects")
        .join(&params.project_id);

    if !project_dir.exists() {
        return Err(ApiError::not_found("项目不存在"));
    }

    std::fs::remove_dir_all(&project_dir)
        .map_err(|e| ApiError::internal(format!("删除项目失败: {e}")))?;

    // 若删除的是当前打开的项目，同步清理内存状态
    if state
        .ontology
        .as_ref()
        .map(|o| o.project_id.as_str() == params.project_id)
        .unwrap_or(false)
    {
        state.close_project();
    }
    // 删除的是上次打开的项目时，同步清除持久化记录
    if crate::state::load_last_project_id(&state.base_dir).as_deref() == Some(params.project_id.as_str()) {
        crate::state::clear_last_project_id(&state.base_dir);
    }

    Ok("ok".to_string())
}
