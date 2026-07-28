/// 项目管理命令
use std::path::PathBuf;

use crate::state::AppState;
use pensoul_core::{ProjectId, NovelOntology};

/// 创建新项目
#[tauri::command]
pub async fn create_project(
    state: tauri::State<'_, AppState>,
    title: String,
) -> Result<String, String> {
    let project_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
    let ontology = NovelOntology::new(project_id.clone(), title);

    {
        let mut ont = state.ontology.write();
        *ont = ontology;
    }

    state.save().map_err(|e| e.to_string())?;
    Ok(state.project_dir.display().to_string())
}

/// 打开项目
#[tauri::command]
pub async fn open_project(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    
    // 安全检查：防止目录遍历
    if path_buf.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("路径包含 '..' 目录遍历，已拒绝".to_string());
    }
    
    let new_state = AppState::load(&path_buf).map_err(|e| e.to_string())?;

    // 更新本体（核心项目数据）
    let mut ont = state.ontology.write();
    *ont = new_state.ontology.read().clone();
    drop(ont);
    // 其他组件（harness、记忆、模型等）在打开项目时重建
    // 因为它们的类型不支持 Clone

    Ok(())
}

/// 保存项目
#[tauri::command]
pub async fn save_project(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.save().map_err(|e| e.to_string())
}
