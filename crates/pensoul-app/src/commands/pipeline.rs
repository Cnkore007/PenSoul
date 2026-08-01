//! 连写管线 IPC 命令 —— 造化工坊的后端入口。
//!
//! `run_chapter_pipeline` 是 async 长跑命令：前端 invoke 挂起期间，
//! 进度通过 `harness-event` 事件实时推送；暂停/继续/停止是独立的短命令。
use crate::pipeline;
use crate::state::AppState;
use std::sync::atomic::Ordering;

/// 启动章节连写管线（写作 → 审查 → 回灌，逐章自动推进）
///
/// - `chapter_ids`：指定章节（缺省 = 所有「有梗概且正文为空」的章节，按序号升序）
/// - `writing_model` / `review_model`：缺省自动选第一个可用模型；审查模型尽量与写作不同
#[tauri::command]
pub async fn run_chapter_pipeline(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    chapter_ids: Option<Vec<String>>,
    writing_model: Option<String>,
    review_model: Option<String>,
) -> Result<serde_json::Value, String> {
    pipeline::run_pipeline(
        app_handle,
        state.inner().clone(),
        chapter_ids,
        writing_model,
        review_model,
    )
    .await
}

/// 暂停管线（当前阶段完成后停住）
#[tauri::command]
pub async fn pause_pipeline(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if !state.pipeline.running.load(Ordering::SeqCst) {
        return Err("没有正在运行的写作管线".to_string());
    }
    state.pipeline.paused.store(true, Ordering::SeqCst);
    Ok(())
}

/// 继续管线
#[tauri::command]
pub async fn resume_pipeline(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.pipeline.paused.store(false, Ordering::SeqCst);
    Ok(())
}

/// 停止管线（LLM 调用立即中断；已落库章节进度保留）
#[tauri::command]
pub async fn stop_pipeline(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if !state.pipeline.running.load(Ordering::SeqCst) {
        return Err("没有正在运行的写作管线".to_string());
    }
    state.pipeline.stop.store(true, Ordering::SeqCst);
    state.pipeline.paused.store(false, Ordering::SeqCst);
    state.pipeline.notify.notify_waiters();
    Ok(())
}

/// 查询管线状态：`{running, paused, current_chapter}`
#[tauri::command]
pub async fn get_pipeline_state(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(state.pipeline.snapshot())
}
