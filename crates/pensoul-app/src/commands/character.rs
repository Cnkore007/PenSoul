/// 角色管理命令
use crate::state::AppState;

/// 获取所有角色
#[tauri::command]
pub async fn get_characters(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    serde_json::to_value(&ontology.characters).map_err(|e| e.to_string())
}

/// 保存角色列表
#[tauri::command]
pub async fn save_characters(
    state: tauri::State<'_, AppState>,
    characters: serde_json::Value,
) -> Result<(), String> {
    let layer: pensoul_core::CharacterLayer =
        serde_json::from_value(characters).map_err(|e| e.to_string())?;
    {
        let mut ontology = state.ontology.write();
        ontology.characters = layer;
    }
    state.save().map_err(|e| e.to_string())
}
