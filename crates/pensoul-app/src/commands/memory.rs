/// 记忆系统命令
use crate::state::AppState;
use pensoul_memory::estimate_tokens;

/// 构建记忆包
#[tauri::command]
pub async fn build_memory_packet(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<serde_json::Value, String> {
    let chapter_num: i64 = chapter_id
        .parse()
        .map_err(|_| format!("无效的章节 ID: {}", chapter_id))?;

    let total_budget = 8000;

    // 构建各层记忆
    let hot = {
        let hot_mem = state.hot_memory.read();
        hot_mem.build(chapter_num, total_budget / 2)
    };

    let warm = {
        let warm_mem = state.warm_memory.read();
        warm_mem.build(chapter_num, total_budget / 4)
    };

    let cold = {
        let cold_mem = state.cold_memory.read();
        cold_mem.retrieve(chapter_num, total_budget / 5)
    };

    let narrative = {
        let narrative_mem = state.narrative_memory.read();
        narrative_mem.retrieve(chapter_num, total_budget / 10)
    };

    // 计算总 token 数
    let hot_tokens: usize = hot.iter().map(|s| estimate_tokens(s)).sum();
    let warm_tokens = estimate_tokens(&warm.volume_summary);
    let cold_tokens: usize = cold.iter().map(|s| estimate_tokens(s)).sum();
    let narrative_tokens: usize = narrative.iter().map(|d| estimate_tokens(&d.content)).sum();

    let packet = serde_json::json!({
        "hot": hot,
        "warm": warm,
        "cold": cold,
        "narrative": narrative,
        "total_tokens": hot_tokens + warm_tokens + cold_tokens + narrative_tokens,
    });

    Ok(packet)
}

/// 获取热记忆
#[tauri::command]
pub async fn get_hot_memory(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let hot = state.hot_memory.read();
    let is_empty = hot.is_empty();

    Ok(serde_json::json!({
        "is_empty": is_empty,
        "window_size": 2,
    }))
}

/// 获取温记忆
#[tauri::command]
pub async fn get_warm_memory(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let warm = state.warm_memory.read();
    let chapter_count = warm.chapter_count();

    Ok(serde_json::json!({
        "chapter_count": chapter_count,
    }))
}
