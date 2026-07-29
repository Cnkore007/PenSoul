/// 一致性检查命令
use crate::state::AppState;

/// 全书一致性检查
#[tauri::command]
pub async fn check_consistency(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let checker = state.consistency_checker.read();
    let report = checker.check_all();
    let violations: Vec<serde_json::Value> = report
        .violations
        .iter()
        .filter_map(|v| serde_json::to_value(v).ok())
        .collect();
    Ok(violations)
}
