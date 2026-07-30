/// 造化工坊执行命令 — 调用真实 LLM 执行阶段任务
use crate::state::AppState;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HarnessStepResult {
    pub stage_name: String,
    pub thinking: String,
    pub output: String,
}

/// 执行造化工坊的一个阶段步骤
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

    let saved_providers = lh::load_providers(&state);
    let api_keys = { state.api_keys.read().clone() };

    // 找第一个有 API Key 的供应商
    let (provider_id, api_key, api_base) =
        lh::find_any_available_provider(&saved_providers, &api_keys)
            .ok_or_else(|| "未配置任何 LLM API Key，请在「模型设置」中配置".to_string())?;

    // 从 models.json 找该供应商的模型；找不到则用默认
    let saved_models = lh::load_models(&state);
    let model_id = saved_models
        .iter()
        .find(|m| {
            m.get("provider_id").and_then(|v| v.as_str()) == Some(&provider_id)
                && m.get("is_available")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .and_then(|m| m.get("model_id").and_then(|v| v.as_str()))
        .unwrap_or("gpt-4o")
        .to_string();

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
        2048,
    )
    .await?;

    Ok(HarnessStepResult {
        stage_name: stage_name.clone(),
        thinking: format!("已完成「{}」阶段的 LLM 调用", stage_name),
        output,
    })
}
