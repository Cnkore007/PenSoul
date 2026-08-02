//! LLM 调用共享辅助模块
//!
//! 所有后端 LLM 调用统一走这里，避免重复逻辑：
//! 1. 自动从磁盘加载 API Key
//! 2. 从 providers.json / models.json 解析供应商配置
//! 3. 支持 OpenAI 兼容格式 + Anthropic 格式
//! 4. 提供模型→供应商解析、单次 LLM 调用等通用方法

use crate::state::AppState;
use futures_util::StreamExt;
use std::collections::HashMap;

// ── 配置加载 ──

/// 从磁盘加载供应商列表
pub(crate) fn load_providers(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("providers.json");
    if file.exists()
        && let Ok(data) = std::fs::read_to_string(&file)
        && let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data)
    {
        return list;
    }
    Vec::new()
}

/// 从磁盘加载模型列表
pub(crate) fn load_models(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("models.json");
    if file.exists()
        && let Ok(data) = std::fs::read_to_string(&file)
        && let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data)
    {
        return list;
    }
    Vec::new()
}

/// 从磁盘加载 API Key 到内存（幂等，已加载则不会重复读盘）
pub(crate) fn ensure_api_keys_loaded(state: &AppState) {
    let _ = state.load_api_keys();
}

/// 构建 provider_id → api_base 映射
pub(crate) fn build_provider_api_bases(providers: &[serde_json::Value]) -> HashMap<String, String> {
    providers
        .iter()
        .filter_map(|p| {
            let pid = p.get("provider_id")?.as_str()?.to_string();
            let api_base = p.get("api_base")?.as_str()?.to_string();
            Some((pid, api_base))
        })
        .collect()
}

/// 构建 model_id → provider_id 映射
pub(crate) fn build_model_to_provider(models: &[serde_json::Value]) -> HashMap<String, String> {
    models
        .iter()
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
/// 2. 从 providers.json 取 api_base 和 api_key
/// 3. 从模型名回退推断供应商
/// 4. 推断供应商没有 API Key 时，遍历所有供应商找任意一个有 Key 的兜底
/// 5. 全部找不到才报错
pub(crate) fn resolve_provider(
    model_id: &str,
    model_to_provider: &HashMap<String, String>,
    provider_api_bases: &HashMap<String, String>,
    api_keys: &HashMap<String, String>,
) -> Result<(String, String, String), String> {
    // 优先从 models.json 查找
    let inferred = model_to_provider.get(model_id).map(|s| s.as_str());

    // 找不到则从模型名回退推断
    let provider_id = inferred.or_else(|| infer_provider_from_model(model_id));

    if let Some(pid) = provider_id {
        // 推断出了供应商，尝试取 api_base 和 api_key
        let api_base = provider_api_bases
            .get(pid)
            .cloned()
            .unwrap_or_else(|| match pid {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "deepseek" => "https://api.deepseek.com".to_string(),
                "moonshot" => "https://api.moonshot.cn/v1".to_string(),
                "local" => "http://localhost:11434/v1".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            });

        if let Some(key) = api_keys.get(pid) {
            return Ok((pid.to_string(), key.clone(), api_base));
        }
        // 有供应商但没 Key，不立即报错，继续兜底
    }

    // 兜底：遍历所有在 api_keys 中有 Key 的供应商
    for (pid, api_key) in api_keys.iter() {
        if let Some(api_base) = provider_api_bases.get(pid) {
            return Ok((pid.clone(), api_key.clone(), api_base.clone()));
        }
    }

    // 实在找不到，报错
    Err("未配置任何可用的 API Key。请先在「模型设置」中添加供应商并配置 API Key。".to_string())
}

/// 遍历 providers 找第一个有 API Key 的供应商（用于不需要指定模型的场景）
pub(crate) fn find_any_available_provider(
    providers: &[serde_json::Value],
    api_keys: &HashMap<String, String>,
) -> Option<(String, String, String)> {
    providers
        .iter()
        .filter_map(|p| {
            let pid = p.get("provider_id")?.as_str()?.to_string();
            let key = api_keys.get(&pid)?.clone();
            let base = p.get("api_base")?.as_str()?.to_string();
            Some((pid, key, base))
        })
        .next()
}

// ── LLM 调用 ──

/// 供应商认证信息（聚合 provider_id / api_key / api_base 三个关联参数）
pub(crate) struct ProviderAuth<'a> {
    pub provider_id: &'a str,
    pub api_key: &'a str,
    pub api_base: &'a str,
}

/// 单次调用的结果分类（驱动外层重试决策）
enum CallOutcome {
    /// 成功拿到非空文本
    Ok(String),
    /// 可重试的瞬态故障：空响应体、发送失败、空内容、5xx、429 限流
    Retryable(String),
    /// 客户端错误（4xx）：参数不兼容、认证失败等。
    /// 参数类错误（400/404/422）会在 call_llm_once 内用降级请求体重试一次
    ClientError(u16, String),
    /// 输出预算耗尽：推理型模型在 reasoning 阶段用完 max_tokens，
    /// 正文为空（finish_reason=length）。需要加大预算重试
    TokenExhausted,
    /// 不可重试：非空响应体格式错误、超时
    Fatal(String),
}

/// 调用 LLM API（自动处理 OpenAI / Anthropic 认证格式）
///
/// - `auth.provider_id`: 用于判断认证格式（anthropic 用 x-api-key，其他用 Bearer）
/// - `model_id`: 实际发送给 API 的模型 ID
/// - `system_prompt` / `user_prompt`: 对话内容
/// - `temperature` / `max_tokens`: 生成参数（经模型档案自动适配：
///   预算字段名、输出上限夹取、固定采样参数模型的 temperature 剔除）
///
/// 深度创作任务的入口；轻量结构任务请用 `call_llm_task` + `LlmTask::Light`。
pub(crate) async fn call_llm(
    auth: &ProviderAuth<'_>,
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<String, String> {
    call_llm_task(
        auth,
        model_id,
        system_prompt,
        user_prompt,
        temperature,
        max_tokens,
        crate::llm_profile::LlmTask::Deep,
    )
    .await
}

/// 带任务语义的 LLM 调用：Light 任务会按模型档案关闭或降低思考强度
///
/// 三类自动重试：
/// 1. 系统代理（Clash 等）瞬断、5xx 抖动、429 限流，重试一次；
/// 2. 推理型模型 reasoning 耗尽输出预算导致正文为空，预算翻倍（封顶见档案）重试；
/// 3. 中转代理不透传扩展参数（thinking/reasoning_effort/max_completion_tokens）
///    导致 400/404/422 时，自动剔扩展参数降级重试（在 call_llm_once 内部）。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn call_llm_task(
    auth: &ProviderAuth<'_>,
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    task: crate::llm_profile::LlmTask,
) -> Result<String, String> {
    let mut budget = max_tokens;
    let mut last_err = String::new();
    for attempt in 1..=2 {
        match call_llm_once(
            auth,
            model_id,
            system_prompt,
            user_prompt,
            temperature,
            budget,
            task,
        )
        .await
        {
            CallOutcome::Ok(text) => return Ok(text),
            CallOutcome::TokenExhausted => {
                last_err = format!(
                    "输出因预算不足被截断（{budget} tokens），已自动翻倍重试仍未完成，请稍后重试"
                );
                // 预算翻倍再试（封顶取模型档案硬上限与 65536 的较小者）
                budget = budget
                    .saturating_mul(2)
                    .min(crate::llm_profile::doubled_budget_cap(model_id));
            }
            CallOutcome::Retryable(e) => {
                last_err = format!("{e}（第 {attempt} 次尝试）");
            }
            CallOutcome::ClientError(_, e) | CallOutcome::Fatal(e) => return Err(e),
        }
        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }
    }
    Err(last_err)
}

/// 非 2xx 响应分类：5xx / 429 可重试，其余 4xx 记为客户端错误（携带状态码，
/// 供 call_llm_once 判断是否用降级请求体重试）
fn classify_http_failure(status: reqwest::StatusCode, body_text: &str) -> CallOutcome {
    let preview: String = body_text.chars().take(200).collect();
    if status.is_server_error() {
        CallOutcome::Retryable(format!("LLM API 服务端错误 ({status}): {preview}"))
    } else if status.as_u16() == 429 {
        CallOutcome::Retryable("LLM API 限流 (429)，稍后自动重试".to_string())
    } else {
        CallOutcome::ClientError(
            status.as_u16(),
            format!("LLM API 错误 ({status}): {preview}"),
        )
    }
}

/// 单次 LLM 调用实现
#[allow(clippy::too_many_arguments)]
async fn call_llm_once(
    auth: &ProviderAuth<'_>,
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    task: crate::llm_profile::LlmTask,
) -> CallOutcome {
    let ProviderAuth {
        provider_id,
        api_key,
        api_base,
    } = *auth;
    let client = match reqwest::Client::builder()
        // 推理型模型 + 大输出预算（16384 tokens）生成可能需要数分钟，
        // 超时给足 10 分钟，避免长生成被客户端掐断
        .timeout(std::time::Duration::from_secs(600))
        .build()
    {
        Ok(c) => c,
        Err(e) => return CallOutcome::Fatal(format!("创建 HTTP 客户端失败: {e}")),
    };

    if provider_id == "anthropic" {
        // Anthropic Messages API（非流式，用量小暂不改造）
        let url = format!("{}/v1/messages", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model_id,
            "max_tokens": max_tokens,
            "system": system_prompt,
            "messages": [{ "role": "user", "content": user_prompt }],
            "temperature": temperature,
        });
        let response = match client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                return CallOutcome::Fatal(format!(
                    "LLM 请求超时（600 秒）: {e}。生成量过大或网络过慢，请稍后重试"
                ));
            }
            Err(e) => return CallOutcome::Retryable(format!("LLM 请求发送失败: {e}")),
        };
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return classify_http_failure(status, &body_text);
        }
        if body_text.trim().is_empty() {
            return CallOutcome::Retryable(format!(
                "LLM 返回了空响应体（HTTP {status}），通常是网络代理瞬断或服务端抖动"
            ));
        }
        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(j) => j,
            Err(e) => {
                let preview: String = body_text.chars().take(200).collect();
                return CallOutcome::Fatal(format!(
                    "解析 LLM 响应失败: {e}（{status}，{} 字节）: {preview}",
                    body_text.len()
                ));
            }
        };
        let text = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        // stop_reason=max_tokens 表示输出被预算截断（可能只剩半篇），不可用
        if json["stop_reason"].as_str() == Some("max_tokens") {
            return CallOutcome::TokenExhausted;
        }
        if text.trim().is_empty() {
            return CallOutcome::Retryable(
                "LLM 响应中没有文本内容（可能被安全策略拦截或模型拒答）".to_string(),
            );
        }
        return CallOutcome::Ok(text);
    }

    // OpenAI 兼容格式：SSE 流式接收，长生成期间持续有字节流动，
    // 避免部分聚合代理在缓冲完整响应时网关超时（504）。
    // 请求体由模型档案规划（预算字段名 / 输出上限 / 固定采样参数 / 推理控制）
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let body = crate::llm_profile::plan_request(
        model_id,
        system_prompt,
        user_prompt,
        temperature,
        max_tokens,
        task,
    );
    let fallback = crate::llm_profile::plan_fallback_request(
        model_id,
        system_prompt,
        user_prompt,
        temperature,
        max_tokens,
    );
    let first = call_openai_stream(&client, &url, api_key, body.clone()).await;
    match first {
        // 参数类 4xx：中转代理可能不透传扩展参数
        //（thinking / reasoning_effort / max_completion_tokens），
        // 换成剔除扩展参数的降级请求体重试一次
        CallOutcome::ClientError(status, _)
            if matches!(status, 400 | 404 | 422) && body != fallback =>
        {
            call_openai_stream(&client, &url, api_key, fallback).await
        }
        other => other,
    }
}

/// OpenAI 兼容的 SSE 流式调用：逐行解析 data: 块，累加 delta.content
async fn call_openai_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> CallOutcome {
    let response = match client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) if e.is_timeout() => {
            return CallOutcome::Fatal(format!(
                "LLM 请求超时（600 秒）: {e}。生成量过大或网络过慢，请稍后重试"
            ));
        }
        Err(e) => return CallOutcome::Retryable(format!("LLM 请求发送失败: {e}")),
    };
    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return classify_http_failure(status, &body_text);
    }

    // 逐块读取。按 0x0A 字节切行：换行字节不会出现在 UTF-8 多字节序列内，
    // 跨块的中文也不会被切坏
    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut content = String::new();
    let mut finish_reason = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => return CallOutcome::Retryable(format!("LLM 流式响应中断: {e}")),
        };
        buf.extend_from_slice(&bytes);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line).trim().to_string();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(j) = serde_json::from_str::<serde_json::Value>(data) else {
                continue;
            };
            if let Some(c) = j["choices"][0]["delta"]["content"].as_str() {
                content.push_str(c);
            }
            if let Some(f) = j["choices"][0]["finish_reason"].as_str() {
                finish_reason = f.to_string();
            }
        }
    }

    // finish_reason=length 表示输出被预算截断（可能只剩半篇，比如缺失
    // 结束标记的长文），不可用：统一走预算翻倍重试；空正文只是推理
    // 烧光预算的特例
    if finish_reason == "length" {
        return CallOutcome::TokenExhausted;
    }
    if content.trim().is_empty() {
        return CallOutcome::Retryable(
            "LLM 流式响应中没有文本内容（可能被安全策略拦截或模型拒答）".to_string(),
        );
    }
    CallOutcome::Ok(content)
}
