/// 插件注册中心
use std::collections::HashMap;

use pensoul_core::{PensoulError, Result};

use crate::config::PluginConfig;
use crate::validator::PluginValidator;

/// 插件注册中心
pub struct PluginRegistry {
    plugins: HashMap<String, PluginConfig>,
}

impl PluginRegistry {
    /// 创建空注册中心
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// 注册插件（带验证）
    pub fn register(&mut self, config: PluginConfig) -> Result<()> {
        PluginValidator::validate(&config)?;
        self.plugins.insert(config.plugin_id.clone(), config);
        Ok(())
    }

    /// 获取插件配置
    pub fn get(&self, plugin_id: &str) -> Option<&PluginConfig> {
        self.plugins.get(plugin_id)
    }

    /// 列出所有已注册插件 ID
    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// 将插件配置导出为 JSON 字符串
    pub fn export_plugin(&self, plugin_id: &str) -> Result<String> {
        let config = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PensoulError::Internal(format!("插件 {} 不存在", plugin_id)))?;
        serde_json::to_string_pretty(config)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))
    }

    /// 从 JSON 字符串导入插件（带验证）
    pub fn import_plugin(&mut self, json_str: &str) -> Result<()> {
        let config: PluginConfig = serde_json::from_str(json_str)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))?;
        self.register(config)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
