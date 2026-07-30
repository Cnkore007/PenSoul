//! 创作设定 IPC 命令
use crate::state::AppState;
use pensoul_core::ProjectSettings;

/// 保存创作设定到后端
#[tauri::command]
pub async fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: ProjectSettings,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.settings = settings;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取创作设定
#[tauri::command]
pub async fn load_settings(state: tauri::State<'_, AppState>) -> Result<ProjectSettings, String> {
    let ontology = state.ontology.read();
    Ok(ontology.settings.clone())
}

/// 保存核心概念到后端
#[tauri::command]
pub async fn save_concept(
    state: tauri::State<'_, AppState>,
    concept: pensoul_core::CoreConcept,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.core_concept = concept;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取核心概念
#[tauri::command]
pub async fn load_concept(
    state: tauri::State<'_, AppState>,
) -> Result<pensoul_core::CoreConcept, String> {
    let ontology = state.ontology.read();
    Ok(ontology.core_concept.clone())
}

/// 保存萌芽数据到后端
#[tauri::command]
pub async fn save_sprout(
    state: tauri::State<'_, AppState>,
    sprout: pensoul_core::SproutData,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.sprout = sprout;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取萌芽数据
#[tauri::command]
pub async fn load_sprout(
    state: tauri::State<'_, AppState>,
) -> Result<pensoul_core::SproutData, String> {
    let ontology = state.ontology.read();
    Ok(ontology.sprout.clone())
}
