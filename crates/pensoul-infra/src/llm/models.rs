// models.rs — 模型能力档案

use serde::{Deserialize, Serialize};

/// 思考模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingMode {
    None,
    Always,
    Toggleable,
}

/// 模型能力档案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub thinking_mode: ThinkingMode,
    pub supports_streaming: bool,
    /// 官方/厂商文档链接
    pub doc_url: String,
    /// 模型说明
    pub description: String,
    /// 默认生成参数（可被配置覆盖）
    pub default_temperature: Option<f32>,
    pub default_top_p: Option<f32>,
}

impl ModelProfile {
    /// 输入预算 = (上下文窗口 - 输出上限) × 90%
    pub fn input_budget(&self) -> u32 {
        ((self.context_window as f64 - self.max_output_tokens as f64) * 0.9) as u32
    }

    /// 供应商前缀（用于生成默认 base_url）
    pub fn provider(&self) -> &'static str {
        if self.model_id.starts_with("gpt") {
            "openai"
        } else if self.model_id.starts_with("moonshot") || self.model_id.starts_with("kimi") {
            "moonshot"
        } else if self.model_id.starts_with("deepseek") {
            "deepseek"
        } else if self.model_id.starts_with("claude") {
            "anthropic"
        } else {
            "custom"
        }
    }
}

/// 预置模型档案（按供应商分组）
pub fn builtin_profiles() -> Vec<ModelProfile> {
    vec![
        // Moonshot
        ModelProfile {
            model_id: "moonshot-v1-128k".to_string(),
            display_name: "Moonshot v1 128K".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            doc_url: "https://platform.moonshot.cn/docs/guide/start-using-kimi-api".to_string(),
            description: "月之暗面 Kimi 长上下文模型，128K 窗口，适合长文理解与续写".to_string(),
            default_temperature: Some(0.7),
            default_top_p: None,
        },
        ModelProfile {
            model_id: "kimi-k2".to_string(),
            display_name: "Kimi K2".to_string(),
            context_window: 256_000,
            max_output_tokens: 8192,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            doc_url: "https://platform.moonshot.cn/docs".to_string(),
            description: "月之暗面 Kimi K2，256K 上下文，强化推理与创作能力".to_string(),
            default_temperature: Some(0.6),
            default_top_p: None,
        },
        // DeepSeek
        ModelProfile {
            model_id: "deepseek-chat".to_string(),
            display_name: "DeepSeek Chat".to_string(),
            context_window: 64_000,
            max_output_tokens: 4096,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            doc_url: "https://api-docs.deepseek.com/zh-cn/".to_string(),
            description: "DeepSeek 通用对话模型，兼顾速度与质量".to_string(),
            default_temperature: Some(0.7),
            default_top_p: None,
        },
        ModelProfile {
            model_id: "deepseek-reasoner".to_string(),
            display_name: "DeepSeek Reasoner".to_string(),
            context_window: 64_000,
            max_output_tokens: 8192,
            thinking_mode: ThinkingMode::Always,
            supports_streaming: true,
            doc_url: "https://api-docs.deepseek.com/zh-cn/".to_string(),
            description: "DeepSeek 推理模型，长链思考，适合复杂逻辑任务".to_string(),
            default_temperature: None,
            default_top_p: None,
        },
        // OpenAI
        ModelProfile {
            model_id: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            context_window: 128_000,
            max_output_tokens: 16384,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            doc_url: "https://platform.openai.com/docs/models".to_string(),
            description: "OpenAI 多模态旗舰模型".to_string(),
            default_temperature: Some(0.7),
            default_top_p: None,
        },
        ModelProfile {
            model_id: "gpt-4.1".to_string(),
            display_name: "GPT-4.1".to_string(),
            context_window: 1_047_576,
            max_output_tokens: 32_768,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            doc_url: "https://platform.openai.com/docs/models/gpt-4.1".to_string(),
            description: "OpenAI GPT-4.1，百万级上下文，长文创作与检索增强".to_string(),
            default_temperature: Some(0.7),
            default_top_p: None,
        },
        // Anthropic
        ModelProfile {
            model_id: "claude-sonnet-4-20250514".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            context_window: 200_000,
            max_output_tokens: 8192,
            thinking_mode: ThinkingMode::Toggleable,
            supports_streaming: true,
            doc_url: "https://docs.anthropic.com/en/docs/about-claude/models/overview".to_string(),
            description: "Anthropic Claude Sonnet 4，200K 上下文，创作与审校并重".to_string(),
            default_temperature: Some(0.7),
            default_top_p: None,
        },
    ]
}
