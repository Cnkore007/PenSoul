//! 记忆系统命令
use crate::state::AppState;
use pensoul_memory::EditingMode;

/// 构建记忆包（走统一的 MemoryPipeline，按编辑模式分配预算）
#[tauri::command]
pub async fn build_memory_packet(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    mode: Option<String>,
) -> Result<serde_json::Value, String> {
    let chapter_num: i64 = chapter_id
        .parse()
        .map_err(|_| format!("无效的章节 ID: {}", chapter_id))?;

    // 按需切换编辑模式（影响预算分配比例）
    if let Some(mode_str) = mode {
        let editing_mode = match mode_str.as_str() {
            "drafting" => EditingMode::Drafting,
            "revising" => EditingMode::Revising,
            "reviewing" => EditingMode::Reviewing,
            other => return Err(format!("未知的编辑模式: {other}")),
        };
        state.memory.write().mode = editing_mode;
    }

    let packet = {
        let memory = state.memory.read();
        memory.build_packet(chapter_num)
    };

    serde_json::to_value(&packet).map_err(|e| e.to_string())
}

/// 获取热记忆概况
#[tauri::command]
pub async fn get_hot_memory(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let memory = state.memory.read();

    Ok(serde_json::json!({
        "is_empty": memory.hot.is_empty(),
        "window_size": memory.hot.window_size(),
    }))
}

/// 获取温记忆概况
#[tauri::command]
pub async fn get_warm_memory(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let memory = state.memory.read();

    Ok(serde_json::json!({
        "chapter_count": memory.warm.chapter_count(),
    }))
}
