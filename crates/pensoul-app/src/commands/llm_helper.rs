/// LLM 调用共享辅助模块
///
/// 所有后端 LLM 调用统一走这里，避免重复逻辑：
/// 1. 自动从磁盘加载 API Key
/// 2. 从 providers.json / models.json 解析供应商配置
/// 3. 支持 OpenAI 兼容格式 + Anthropic 格式
/// 4. 提供模型→供应商解析、单次 LLM 调用等通用方法

use crate::state::AppState;
use std::collections::HashMap;

// ── 配置加载 ──

/// 从磁盘加载供应商列表
pub(crate) fn load_providers(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("providers.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                return list;
            }
        }
    }
    Vec::new()
}

/// 从磁盘加载模型列表
pub(crate) fn load_models(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("models.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                return list;
            }
        }
    }
    Vec::new()
}

/// 从磁盘加载 API Key 到内存（幂等，已加载则不会重复读盘）
pub(crate) fn ensure_api_keys_loaded(state: &AppState) {
    let _ = state.load_api_keys();
}

/// 构建 provider_id → api_base 映射
pub(crate) fn build_provider_api_bases(providers: &[serde_json::Value]) -> HashMap<String, String> {
    providers.iter()
        .filter_map(|p| {
            let pid = p.get("provider_id")?.as_str()?.to_string();
            let api_base = p.get("api_base")?.as_str()?.to_string();
            Some((pid, api_base))
        })
        .collect()
}

/// 构建 model_id → provider_id 映射
pub(crate) fn build_model_to_provider(models: &[serde_json::Value]) -> HashMap<String, String> {
    models.iter()
        .filter_map(|m| {
            let model_id = m.get("model_id")?.as_str()?.to_string();
            let provider_id = m.get("provider_id")?.as_str()?.to_string();
            Some((model_id, provider_id))
        })
        .collect()
}

// ── 供应商解析 ──

/// 从模型名回退推断供应商 ID
pub(crate) fn infer_provider_from_model(model: &str) -> Option<&'static str> {
    if model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3") {
        Some("openai")
    } else if model.starts_with("claude-") {
        Some("anthropic")
    } else if model.starts_with("deepseek") {
        Some("deepseek")
    } else if model.starts_with("moonshot") {
        Some("moonshot")
    } else {
        None
    }
}

/// 解析模型对应的 provider_id / api_key / api_base
///
/// 查找顺序：
/// 1. 从 models.json 找模型对应的 provider_id
/// 2. 从 providers.json 取 api_base
/// 3. 从 state.api_keys 取 API Key
/// 4. 都找不到时回退到硬编码推断
pub(crate) fn resolve_provider(
    model_id: &str,
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> Result<(String, String, String), String> {
    // 从 models.json 查找
    let provider_id = model_to_provider.get(model_id)
        .map(|s| s.as_str())
        .or_else(|| infer_provider_from_model(model_id))
        .ok_or_else(|| format!("无法确定模型「{}」所属的供应商，请先在「模型设置」中配置", model_id))?;

    // 从 providers.json 取 api_base，找不到则硬编码回退
    let api_base = provider_api_bases.get(provider_id)
        .cloned()
        .unwrap_or_else(|| match provider_id {
            "openai" => "https://api.openai.com/v1".to_string(),
            "anthropic" => "https://api.anthropic.com".to_string(),
            "deepseek" => "https://api.deepseek.com".to_string(),
            "moonshot" => "https://api.moonshot.cn/v1".to_string(),
            "local" => "http://localhost:11434/v1".to_string(),
            _ => "https://api.openai.com/v1".to_string(),
        });

    // 从内存取 API Key
    let api_key = api_keys.get(provider_id)
        .cloned()
        .ok_or_else(|| format!("未配置「{}」的 API Key，请在「模型设置」中配置", provider_id))?;

    Ok((provider_id.to_string(), api_key, api_base))
}

/// 遍历 providers 找第一个有 API Key 的供应商（用于不需要指定模型的场景）
pub(crate) fn find_any_available_provider(
    providers: &[serde_json::Value],
    api_keys: &HashMap<String, String>,
) -> Option<(String, String, String)> {
    providers.iter()
        .filter_map(|p| {
            let pid = p.get("provider_id")?.as_str()?.to_string();
            let key = api_keys.get(&pid)?.clone();
            let base = p.get("api_base")?.as_str()?.to_string();
            Some((pid, key, base))
        })
        .next()
}

// ── LLM 调用 ──

/// 调用 LLM API（自动处理 OpenAI / Anthropic 认证格式）
///
/// - `provider_id`: 用于判断认证格式（anthropic 用 x-api-key，其他用 Bearer）
/// - `model_id`: 实际发送给 API 的模型 ID
/// - `system_prompt` / `user_prompt`: 对话内容
/// - `temperature` / `max_tokens`: 生成参数
pub(crate) async fn call_llm(
    provider_id: &str,
    api_key: &str,
    api_base: &str,
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    if provider_id == "anthropic" {
        // Anthropic Messages API 格式
        let url = format!("{}/v1/messages", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model_id,
            "max_tokens": max_tokens,
            "system": system_prompt,
            "messages": [{ "role": "user", "content": user_prompt }],
            "temperature": temperature,
        });
        let response = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
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
        Ok(json["content"][0]["text"].as_str().unwrap_or("").to_string())
    } else {
        // OpenAI 兼容格式（OpenAI / DeepSeek / Moonshot / Ollama 等）
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model_id,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
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
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
