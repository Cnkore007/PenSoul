// config.rs — 全局 LLM 配置存储（唯一配置模块）
// 所有供应商/模型/密钥/参数统一由本模块管理，文件位于 data/_config/llm-config.json

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 思考模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingMode {
    None,
    Always,
    Toggleable,
}

/// 供应商
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openai,
    Moonshot,
    Deepseek,
    Anthropic,
    Custom,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Openai => "openai",
            Provider::Moonshot => "moonshot",
            Provider::Deepseek => "deepseek",
            Provider::Anthropic => "anthropic",
            Provider::Custom => "custom",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "openai" => Some(Provider::Openai),
            "moonshot" => Some(Provider::Moonshot),
            "deepseek" => Some(Provider::Deepseek),
            "anthropic" => Some(Provider::Anthropic),
            "custom" => Some(Provider::Custom),
            _ => None,
        }
    }
}

/// 单条模型/供应商配置（含密钥与详细参数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    /// 显示名称
    pub name: String,
    pub provider: Provider,
    /// 模型 ID（如 deepseek-chat）
    pub model_id: String,
    /// 中转/官方地址（形如 https://api.deepseek.com）
    pub base_url: String,
    /// API Key（本地明文存储，接口返回时脱敏）
    pub api_key: String,
    /// 上下文窗口（token）
    pub context_window: u32,
    /// 最大输出 token
    pub max_output_tokens: u32,
    pub thinking_mode: ThinkingMode,
    pub supports_streaming: bool,
    /// 默认生成参数
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    /// 停止序列（逗号分隔，发送时转为数组）
    pub stop_sequences: Option<String>,
    /// 强制 JSON 输出
    pub json_mode: Option<bool>,
    /// 思考预算（DeepSeek reasoner）
    pub thinking_budget: Option<u32>,
    /// 请求超时（秒）
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u32,
    /// 模型文档链接
    pub doc_url: Option<String>,
    pub notes: Option<String>,
    pub enabled: bool,
}

fn default_timeout_seconds() -> u32 {
    120
}

impl ProviderConfig {
    pub fn new(name: impl Into<String>, provider: Provider, model_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            provider,
            model_id: model_id.into(),
            base_url: String::new(),
            api_key: String::new(),
            context_window: 0,
            max_output_tokens: 4096,
            thinking_mode: ThinkingMode::None,
            supports_streaming: true,
            temperature: Some(0.7),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
            json_mode: None,
            thinking_budget: None,
            timeout_seconds: 120,
            doc_url: None,
            notes: None,
            enabled: true,
        }
    }

    /// 输入预算 = (上下文窗口 - 输出上限) × 90%
    pub fn input_budget(&self) -> u32 {
        let window = self.context_window.max(self.max_output_tokens);
        ((window - self.max_output_tokens) as f64 * 0.9) as u32
    }

    /// 是否已配置密钥
    pub fn has_key(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    /// 密钥脱敏：只保留头尾 4 位
    pub fn masked_key(&self) -> String {
        if self.api_key.is_empty() {
            return String::new();
        }
        if self.api_key.len() <= 8 {
            return "***".to_string();
        }
        format!(
            "{}***{}",
            &self.api_key[..4],
            &self.api_key[self.api_key.len() - 4..]
        )
    }

    /// 对外视图：绝不包含完整密钥
    pub fn to_public(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "provider": self.provider.as_str(),
            "model_id": self.model_id,
            "base_url": self.base_url,
            "has_key": self.has_key(),
            "api_key_masked": self.masked_key(),
            "context_window": self.context_window,
            "max_output_tokens": self.max_output_tokens,
            "input_budget": self.input_budget(),
            "thinking_mode": self.thinking_mode,
            "supports_streaming": self.supports_streaming,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "frequency_penalty": self.frequency_penalty,
            "presence_penalty": self.presence_penalty,
            "stop_sequences": self.stop_sequences,
            "json_mode": self.json_mode,
            "thinking_budget": self.thinking_budget,
            "timeout_seconds": self.timeout_seconds,
            "doc_url": self.doc_url,
            "notes": self.notes,
            "enabled": self.enabled,
        })
    }
}

/// 全局 LLM 配置集合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 默认配置 ID（测试/上下文检测等场景默认使用）
    pub default_provider_id: Option<String>,
    pub providers: Vec<ProviderConfig>,
}

impl LlmConfig {
    pub fn get(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ProviderConfig> {
        self.providers.iter_mut().find(|p| p.id == id)
    }

    pub fn default_provider(&self) -> Option<&ProviderConfig> {
        self.default_provider_id
            .as_deref()
            .and_then(|id| self.get(id))
    }
}

/// 配置存储：只读写 data/_config/llm-config.json
pub struct LlmConfigStore {
    file: PathBuf,
}

impl LlmConfigStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            file: base_dir.into().join("_config").join("llm-config.json"),
        }
    }

    pub fn config_dir(&self) -> &Path {
        self.file
            .parent()
            .expect("配置文件路径必须有父目录")
    }

    /// 加载配置；文件不存在或损坏时返回默认空配置
    pub fn load(&self) -> LlmConfig {
        std::fs::read_to_string(&self.file)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    /// 原子保存（tmp + rename）
    pub fn save(&self, config: &LlmConfig) -> std::io::Result<()> {
        std::fs::create_dir_all(self.config_dir())?;
        let tmp = self.file.with_extension(format!("json.tmp-{}", uuid::Uuid::new_v4()));
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_key_keeps_head_and_tail() {
        let mut config = ProviderConfig::new("测试", Provider::Deepseek, "deepseek-chat");
        config.api_key = "sk-abcdefgh12345678".to_string();
        assert_eq!(config.masked_key(), "sk-a***5678");
        assert!(!config.to_public().get("api_key").is_some());
    }

    #[test]
    fn input_budget_respects_window() {
        let mut config = ProviderConfig::new("测试", Provider::Openai, "gpt-4o");
        config.context_window = 128_000;
        config.max_output_tokens = 16_384;
        assert_eq!(config.input_budget(), ((128_000 - 16_384) as f64 * 0.9) as u32);
    }

    #[test]
    fn store_roundtrip_preserves_config() {
        let dir = std::env::temp_dir().join(format!("pensoul-llmcfg-{}", uuid::Uuid::new_v4()));
        let store = LlmConfigStore::new(&dir);
        let mut config = LlmConfig::default();
        config
            .providers
            .push(ProviderConfig::new("DeepSeek", Provider::Deepseek, "deepseek-chat"));
        config.default_provider_id = Some(config.providers[0].id.clone());
        store.save(&config).expect("保存配置失败");

        let loaded = store.load();
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].model_id, "deepseek-chat");
        assert_eq!(
            loaded.default_provider_id.as_deref(),
            Some(config.providers[0].id.as_str())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
