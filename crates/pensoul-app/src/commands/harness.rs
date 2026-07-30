//! Harness 流程引擎命令
use crate::state::AppState;

/// 启动 Harness 阶段
#[tauri::command]
pub async fn start_harness_stage(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut harness = state.harness.write();

    match harness.start_stage() {
        Ok(inst) => serde_json::to_string(&inst).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// 完成 Harness 阶段
#[tauri::command]
pub async fn complete_harness_stage(
    state: tauri::State<'_, AppState>,
    result: serde_json::Value,
) -> Result<(), String> {
    let mut harness = state.harness.write();

    harness.complete_stage(result).map_err(|e| e.to_string())
}

/// 人工批准指定阶段的 Manual 门控（带外确认通道）。
///
/// 批准后再调用 `complete_harness_stage` 才会放行；
/// AI 无法通过在阶段产出中写字段来自我批准。
#[tauri::command]
pub async fn approve_harness_stage(
    state: tauri::State<'_, AppState>,
    stage_name: String,
) -> Result<(), String> {
    let mut harness = state.harness.write();
    harness
        .approve_manual_gate(&pensoul_core::StageName::new(stage_name))
        .map_err(|e| e.to_string())
}

/// 注入备忘录
#[tauri::command]
pub async fn inject_memo(
    state: tauri::State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut harness = state.harness.write();

    let value_str = match value {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };

    harness
        .inject_memo(&key, &value_str)
        .map_err(|e| e.to_string())
}

/// 获取 Harness 状态
#[tauri::command]
pub async fn get_harness_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let harness = state.harness.read();
    let engine_state = harness.build_state();

    serde_json::to_value(&engine_state).map_err(|e| e.to_string())
}
