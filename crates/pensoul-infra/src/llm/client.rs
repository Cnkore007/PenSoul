// client.rs — LLM 统一调用客户端
// 同步调用、重试策略、超时控制

use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP 错误: {status:?}: {message}")]
    Http { status: Option<u16>, message: String },
    #[error("解析错误: {0}")]
    Parse(String),
    #[error("超时")]
    Timeout,
    #[error("所有模型失败: {0}")]
    AllModelsFailed(String),
}

pub type LlmResult<T> = std::result::Result<T, LlmError>;

/// 最大重试次数（不含首次请求）
const MAX_RETRIES: u32 = 2;

/// LLM 调用请求
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    /// 停止序列（OpenAI 兼容）
    pub stop_sequences: Option<Vec<String>>,
    /// 强制 JSON 输出（OpenAI 兼容 response_format）
    pub json_mode: Option<bool>,
    /// 思考预算（DeepSeek reasoner 的 thinking.budget_tokens）
    pub thinking_budget: Option<u32>,
    pub system_prompt: Option<String>,
}

/// 消息
#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

/// LLM 调用响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

/// Token 使用量
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// LLM 统一客户端
pub struct LlmClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

/// 构造 chat/completions 请求体（可独立测试）
pub fn build_chat_body(request: &LlmRequest) -> serde_json::Value {
    let mut messages = Vec::new();
    if let Some(system_prompt) = &request.system_prompt {
        messages.push(serde_json::json!({
            "role": "system",
            "content": system_prompt,
        }));
    }
    for message in &request.messages {
        messages.push(serde_json::json!({
            "role": message.role,
            "content": message.content,
        }));
    }

    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), serde_json::json!(request.model));
    body.insert("messages".to_string(), serde_json::json!(messages));
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".to_string(), serde_json::json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".to_string(), serde_json::json!(top_p));
    }
    if let Some(frequency_penalty) = request.frequency_penalty {
        body.insert("frequency_penalty".to_string(), serde_json::json!(frequency_penalty));
    }
    if let Some(presence_penalty) = request.presence_penalty {
        body.insert("presence_penalty".to_string(), serde_json::json!(presence_penalty));
    }
    if let Some(stop) = &request.stop_sequences {
        if !stop.is_empty() {
            body.insert("stop".to_string(), serde_json::json!(stop));
        }
    }
    if request.json_mode == Some(true) {
        body.insert(
            "response_format".to_string(),
            serde_json::json!({ "type": "json_object" }),
        );
    }
    if let Some(budget) = request.thinking_budget {
        body.insert(
            "thinking".to_string(),
            serde_json::json!({ "type": "enabled", "budget_tokens": budget }),
        );
    }
    serde_json::Value::Object(body)
}

impl LlmClient {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self::with_timeout(api_key, base_url, 120)
    }

    /// 自定义超时（秒）
    pub fn with_timeout(api_key: String, base_url: String, timeout_seconds: u64) -> Self {
        let timeout_seconds = timeout_seconds.clamp(5, 600);
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("构建 HTTP 客户端失败，程序不应继续");
        Self {
            api_key,
            base_url,
            http_client,
        }
    }

    /// 同步调用（非流式）
    pub async fn complete(&self, request: LlmRequest) -> LlmResult<LlmResponse> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let body = build_chat_body(&request);

        let mut last_error: Option<LlmError> = None;
        for attempt in 0..=MAX_RETRIES {
            match self.try_complete(&url, &body, &request).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if !error.is_retryable() {
                        return Err(error);
                    }
                    last_error = Some(error);
                    let backoff = Duration::from_millis(300 * 2u64.pow(attempt));
                    tokio::time::sleep(backoff).await;
                }
            }
        }

        Err(last_error.unwrap_or(LlmError::AllModelsFailed(
            "请求重试耗尽".to_string(),
        )))
    }

    /// 单次请求（供重试逻辑复用）
    async fn try_complete(
        &self,
        url: &str,
        body: &serde_json::Value,
        request: &LlmRequest,
    ) -> LlmResult<LlmResponse> {
        let resp = self
            .http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout
                } else {
                    LlmError::Http {
                        status: None,
                        message: e.to_string(),
                    }
                }
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            // 按字符截断，避免字节切片落在 UTF-8 多字节字符中间导致 panic
            let message = if text.chars().count() > 500 {
                format!(
                    "{}（截断）",
                    text.chars().take(500).collect::<String>()
                )
            } else {
                text
            };
            return Err(LlmError::Http {
                status: Some(status.as_u16()),
                message,
            });
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::Parse(e.to_string()))?;
        // 防御式解析：部分模型/网关 200 响应可能没有 choices 或结构不同，
        // 直接索引 json["choices"][0] 会对 null 触发 panic，这里全程安全取值
        let content = parse_content(&json);

        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                prompt_tokens: u.get("prompt_tokens")?.as_u64()? as u32,
                completion_tokens: u.get("completion_tokens")?.as_u64()? as u32,
                total_tokens: u.get("total_tokens")?.as_u64()? as u32,
            })
        });

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            usage,
        })
    }
}

impl LlmError {
    /// 是否需要重试：超时或服务端 5xx
    fn is_retryable(&self) -> bool {
        match self {
            LlmError::Timeout => true,
            LlmError::Http { status: Some(status), .. } => (500..600).contains(status),
            _ => false,
        }
    }
}

/// 从 OpenAI 兼容响应中安全提取正文内容
/// 防御式：choices 缺失 / 为 null / 结构不同时返回空串，绝不 panic
fn parse_content(json: &serde_json::Value) -> String {
    json.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> LlmRequest {
        LlmRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![LlmMessage {
                role: "user".to_string(),
                content: "写一段开头".to_string(),
            }],
            max_tokens: Some(128),
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            json_mode: None,
            thinking_budget: None,
            system_prompt: Some("你是小说助手".to_string()),
        }
    }

    #[test]
    fn body_contains_all_messages_as_array() {
        let body = build_chat_body(&sample_request());
        let messages = body["messages"].as_array().expect("messages 必须是数组");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "写一段开头");
    }

    #[test]
    fn parse_content_normal_response() {
        let json = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "正文内容"}}]
        });
        assert_eq!(parse_content(&json), "正文内容");
    }

    #[test]
    fn parse_content_missing_choices_does_not_panic() {
        // 网关/模型返回 200 但没有 choices（或结构不同）时不得 panic，应返回空串
        assert_eq!(parse_content(&serde_json::json!({})), "");
        assert_eq!(parse_content(&serde_json::json!({"error": "boom"})), "");
        assert_eq!(parse_content(&serde_json::json!({"choices": null})), "");
        assert_eq!(parse_content(&serde_json::json!({"choices": []})), "");
        assert_eq!(parse_content(&serde_json::json!({"choices": [{}]})), "");
    }

    #[test]
    fn missing_option_fields_are_omitted() {
        let body = build_chat_body(&sample_request());
        assert!(body.get("temperature").is_none(), "None 字段不应序列化为 null");
        assert_eq!(body["max_tokens"], 128);
    }

    #[test]
    fn detailed_params_appear_in_body() {
        let mut request = sample_request();
        request.stop_sequences = Some(vec!["</s>".to_string(), "再见".to_string()]);
        request.json_mode = Some(true);
        request.thinking_budget = Some(2048);
        let body = build_chat_body(&request);
        assert_eq!(body["stop"][1], "再见");
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["thinking"]["budget_tokens"], 2048);
    }
}
