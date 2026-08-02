//! Harness 流程引擎与连写管线命令
//!
//! 含三类入口：
//! - 阶段执行（start/complete/approve/inject/get，旧模拟器兼容）
//! - 连写管线控制（run/pause/resume/stop/get_state，造化工坊主入口）
//! - 记忆包构建（build_memory_packet / 热温记忆概况）
use crate::pipeline;
use crate::state::AppState;
use pensoul_memory::EditingMode;
use serde::Serialize;
use std::sync::atomic::Ordering;

#[derive(Debug, Serialize)]
pub struct HarnessStepResult {
    pub stage_name: String,
    pub thinking: String,
    pub output: String,
}

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

/// 执行造化工坊的一个阶段步骤（旧模拟器入口，兼容保留）
#[tauri::command]
pub async fn execute_harness_step(
    state: tauri::State<'_, AppState>,
    stage_name: String,
    project_context: String,
    stage_prompt: String,
) -> Result<HarnessStepResult, String> {
    use super::llm_helper as lh;

    // 确保 API Key 已从磁盘加载
    lh::ensure_api_keys_loaded(&state);

    let api_keys = { state.api_keys.read().clone() };
    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);

    // 缺省模型：全局默认优先，其次任意可用模型
    let model_id = lh::pick_default_model(&saved_models, &api_keys)
        .ok_or_else(|| "未配置任何 LLM API Key，请在「模型设置」中配置".to_string())?;
    let (provider_id, api_key, api_base) = lh::resolve_provider(
        &model_id,
        &lh::build_model_to_provider(&saved_models),
        &lh::build_provider_api_bases(&saved_providers),
        &api_keys,
    )?;

    // 构建系统提示词
    let system_prompt = format!(
        "你是 PenSoul 创作引擎的 AI Agent。当前正在执行「{}」阶段。\n\n{}\n\n项目上下文：\n{}\n\n请根据以上信息完成当前阶段的任务。输出要具体、可操作。",
        stage_name, stage_prompt, project_context
    );

    let output = lh::call_llm(
        &lh::ProviderAuth {
            provider_id: &provider_id,
            api_key: &api_key,
            api_base: &api_base,
        },
        &model_id,
        &system_prompt,
        "请执行当前阶段的任务。",
        0.7,
        // 推理型模型 reasoning 会占用大量预算，2048 极易耗尽
        8192,
    )
    .await?;

    Ok(HarnessStepResult {
        stage_name: stage_name.clone(),
        thinking: format!("已完成「{}」阶段的 LLM 调用", stage_name),
        output,
    })
}

/// 启动章节连写管线（写作 → 审查 → 回灌，逐章自动推进）
///
/// - `chapter_ids`：指定章节（缺省 = 所有「有梗概且正文为空」的章节，按序号升序）
/// - `writing_model` / `review_model`：缺省自动选第一个可用模型；审查模型尽量与写作不同
/// - `writing_cards` / `review_cards`：工作流为写作/审查环节绑定的技法卡 SKILL.md 路径（可空）
#[tauri::command]
pub async fn run_chapter_pipeline(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    chapter_ids: Option<Vec<String>>,
    writing_model: Option<String>,
    review_model: Option<String>,
    writing_cards: Option<Vec<String>>,
    review_cards: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    pipeline::run_pipeline(
        app_handle,
        state.inner().clone(),
        chapter_ids,
        writing_model,
        review_model,
        writing_cards,
        review_cards,
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

/// 构建记忆包（走统一的 MemoryPipeline，按编辑模式分配预算）
///
/// chapter_id 为本体的字符串主键（如 `ch-<ts>-<rand>`），
/// 内部通过本体查出章节序号（chapter_no）再构建记忆包。
#[tauri::command]
pub async fn build_memory_packet(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    mode: Option<String>,
) -> Result<serde_json::Value, String> {
    let chapter_num = {
        let ontology = state.ontology.read();
        ontology
            .get_chapter(&pensoul_core::ChapterId::new(&chapter_id))
            .map(|ch| ch.chapter_no)
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("章节不存在或未分配序号: {}", chapter_id))?
    };

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
