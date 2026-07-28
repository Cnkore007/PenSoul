/// 插件配置解析
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 插件阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStage {
    /// 阶段名称
    pub name: String,
    /// 阶段使用的工具
    pub tool: String,
    /// 门控类型: auto / manual / conditional
    #[serde(default)]
    pub gate: String,
    /// 执行器类型: local / delegated
    #[serde(default = "default_runner")]
    pub runner: String,
    /// 提示模板
    #[serde(default)]
    pub prompt_template: String,
    /// 允许的工具白名单
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 超时秒数
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i32,
    /// 最大重试次数
    #[serde(default = "default_retries")]
    pub max_retries: i32,
}

fn default_runner() -> String {
    "local".into()
}

fn default_timeout() -> i32 {
    300
}

fn default_retries() -> i32 {
    3
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 插件唯一标识
    pub plugin_id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 插件阶段列表
    #[serde(default)]
    pub stages: Vec<PluginStage>,
    /// 扩展元数据
    #[serde(default)]
    pub metadata: Value,
}
