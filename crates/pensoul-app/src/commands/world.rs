/// 世界观管理命令
use crate::state::AppState;

/// 获取世界观数据
#[tauri::command]
pub async fn get_world(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    serde_json::to_value(&ontology.world).map_err(|e| e.to_string())
}

/// 保存世界观数据
#[tauri::command]
pub async fn save_world(
    state: tauri::State<'_, AppState>,
    world: serde_json::Value,
) -> Result<(), String> {
    let layer: pensoul_core::WorldLayer =
        serde_json::from_value(world).map_err(|e| e.to_string())?;
    {
        let mut ontology = state.ontology.write();
        ontology.world = layer;
    }
    state.save().map_err(|e| e.to_string())
}
