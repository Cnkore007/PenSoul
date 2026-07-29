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
    // 先克隆数据，释放锁后再做异步操作
    let api_keys: std::collections::HashMap<String, String> = {
        let keys = state.api_keys.read();
        keys.clone()
    };

    // 尝试找到可用的 API key
    let providers = [("openai", "https://api.openai.com/v1"), ("deepseek", "https://api.deepseek.com"), ("moonshot", "https://api.moonshot.cn/v1")];
    let mut api_key = String::new();
    let mut api_base = String::new();
    let mut model_id = "gpt-4o".to_string();

    for (provider, base) in &providers {
        if let Some(key) = api_keys.get(*provider) {
            api_key = key.clone();
            api_base = base.to_string();
            // 根据供应商选择模型
            model_id = match *provider {
                "openai" => "gpt-4o".to_string(),
                "deepseek" => "deepseek-chat".to_string(),
                "moonshot" => "moonshot-v1-8k".to_string(),
                _ => "gpt-4o".to_string(),
            };
            break;
        }
    }

    if api_key.is_empty() {
        return Err("未配置任何 LLM API Key，请在「模型设置」中配置".to_string());
    }

    // 构建系统提示词
    let system_prompt = format!(
        "你是 PenSoul 创作引擎的 AI Agent。当前正在执行「{}」阶段。\n\n{}\n\n项目上下文：\n{}\n\n请根据以上信息完成当前阶段的任务。输出要具体、可操作。",
        stage_name, stage_prompt, project_context
    );

    // 调用 LLM
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": "请执行当前阶段的任务。" }
        ],
        "temperature": 0.7,
        "max_tokens": 2048
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM 请求失败: {}", e))?;

    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("LLM API 错误 ({}): {}", status, body_text));
    }

    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;

    let output = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(无响应)")
        .to_string();

    Ok(HarnessStepResult {
        stage_name: stage_name.clone(),
        thinking: format!("已完成「{}」阶段的 LLM 调用", stage_name),
        output,
    })
}
