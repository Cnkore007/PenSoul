/// PenSoul LLM 提供商抽象
use pensoul_core::Result;

use crate::model::ModelConfig;

/// LLM 提供商 trait
pub trait LlmProvider: Send + Sync {
    /// 提供商名称
    fn name(&self) -> &str;

    /// 调用模型
    fn call(&self, model: &ModelConfig, prompt: &str) -> Result<String>;
}

/// OpenAI 兼容提供商（支持 OpenAI、DeepSeek、Moonshot、TokenHub 等）
pub struct OpenAiProvider {
    pub api_key: String,
}

impl OpenAiProvider {
    /// 创建新的 OpenAI 提供商
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    /// 构建完整的 API URL
    fn build_url(model: &ModelConfig) -> String {
        let base = model
            .endpoint
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{}/chat/completions", base.trim_end_matches('/'))
    }
}

impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn call(&self, model: &ModelConfig, prompt: &str) -> Result<String> {
        let url = Self::build_url(model);

        let client = reqwest::blocking::Client::new();
        let body = serde_json::json!({
            "model": model.model_id,
            "messages": [
                {
                    "role": "system",
                    "content": "你是一位资深的小说创作顾问。请根据用户的需求，提供有创意、有深度的写作建议。回答简洁明了，直接给出建议本身。每条建议用一句话概括。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.85,
            "max_tokens": 1024
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| pensoul_core::PensoulError::Internal(format!("API 请求失败: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp
                .text()
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(pensoul_core::PensoulError::Internal(format!(
                "API 返回错误 ({}): {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .map_err(|e| pensoul_core::PensoulError::Internal(format!("解析 API 响应失败: {e}")))?;

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

/// Anthropic 提供商
pub struct AnthropicProvider {
    pub api_key: String,
}

impl AnthropicProvider {
    /// 创建新的 Anthropic 提供商
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn call(&self, _model: &ModelConfig, _prompt: &str) -> Result<String> {
        Err(pensoul_core::PensoulError::Internal(
            "Anthropic 提供商尚未实现".to_string(),
        ))
    }
}
