//! PenSoul LLM 模型配置、提供商抽象与对比
use pensoul_core::Result;
use serde::{Deserialize, Serialize};

/// 任务类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// 大纲生成
    Outline,
    /// 草稿写作
    Drafting,
    /// 修改润色
    Revision,
    /// 一致性检查
    Consistency,
    /// 文风分析
    Style,
    /// 通用
    General,
}

/// 模型配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型唯一标识符
    pub model_id: String,
    /// 提供商名称
    pub provider: String,
    /// 显示名称
    pub display_name: String,
    /// 最大令牌数
    pub max_tokens: u32,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 是否支持流式输出
    pub supports_streaming: bool,
    /// 每千令牌成本（美元）
    pub cost_per_1k_tokens: f64,
    /// 平均质量评分（0.0-1.0）
    pub avg_quality_score: f32,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: u32,
    /// 是否可用
    pub is_available: bool,
    /// 失败次数
    pub failure_count: u32,
    /// 最后失败时间（UNIX 时间戳秒）
    pub last_failure_time: f64,
    /// 冷却时间（秒）
    pub cooldown_seconds: u64,
    /// API 密钥（序列化时跳过）
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// API 端点（序列化时跳过）
    #[serde(skip_serializing)]
    pub endpoint: Option<String>,
}

/// 路由结果结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    /// 选择的模型
    pub chosen_model: ModelConfig,
    /// 是否使用了备用模型
    pub fallback_used: bool,
    /// 备用原因
    pub fallback_reason: String,
    /// 尝试链（所有尝试过的模型 ID）
    pub attempt_chain: Vec<String>,
    /// 路由耗时（毫秒）
    pub routing_time_ms: u64,
}

/// LLM 提供商 trait
pub trait LlmProvider: Send + Sync {
    /// 提供商名称
    fn name(&self) -> &str;

    /// 调用模型
    fn call(&self, model: &ModelConfig, prompt: &str) -> Result<String>;
}

/// OpenAI 兼容提供商（支持 OpenAI、DeepSeek、Moonshot 等）
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

use crate::router::ModelRouter;

/// 模型对比结果
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// 模型 ID
    pub model_id: String,
    /// 输出内容
    pub output: String,
    /// 延迟（毫秒）
    pub latency_ms: u64,
    /// 质量评分
    pub quality_score: f32,
}

/// 模型对比
#[derive(Debug, Clone)]
pub struct ModelComparison {
    /// 参与对比的模型
    pub models: Vec<ModelConfig>,
    /// 对比结果
    pub results: Vec<ComparisonResult>,
}

/// 对比多个模型
pub fn compare_models(router: &ModelRouter, task_type: TaskType, prompt: &str) -> ModelComparison {
    // 获取推荐模型列表
    let recommended_models = router.get_recommendation(task_type);

    // 这里只是示例实现，实际中应该调用每个模型并收集结果
    // 由于 provider 尚未实现，我们返回一个模拟的对比结果

    let models: Vec<ModelConfig> = recommended_models.into_iter().cloned().collect();

    // 模拟对比结果
    let results = models
        .iter()
        .map(|model| ComparisonResult {
            model_id: model.model_id.clone(),
            output: format!(
                "模型 {} 对提示 \"{}\" 的响应（模拟）",
                model.display_name, prompt
            ),
            latency_ms: model.avg_latency_ms as u64,
            quality_score: model.avg_quality_score,
        })
        .collect();

    ModelComparison { models, results }
}
