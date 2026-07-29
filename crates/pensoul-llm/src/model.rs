/// PenSoul LLM 模型配置和类型定义
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
