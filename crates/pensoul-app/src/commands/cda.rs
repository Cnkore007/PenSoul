/// CDA 影响图命令
use crate::state::AppState;

/// 查找受影响的章节
#[tauri::command]
pub async fn find_affected_chapters(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    changed_entities: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let graph = state.impact_graph.read();

    // 解析 chapter_id 为 u32
    let chapter_num: u32 = chapter_id
        .parse()
        .map_err(|_| format!("无效的章节 ID: {}", chapter_id))?;

    let affected = graph.find_affected(chapter_num, &changed_entities, 5);

    let results: Vec<serde_json::Value> = affected
        .into_iter()
        .filter_map(|item| serde_json::to_value(&item).ok())
        .collect();

    Ok(results)
}

/// 获取影响图
#[tauri::command]
pub async fn get_impact_graph(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let graph = state.impact_graph.read();
    let stats = graph.stats();

    serde_json::to_value(&stats).map_err(|e| e.to_string())
}
