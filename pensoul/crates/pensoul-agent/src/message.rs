/// PenSoul 智能体消息类型定义
use pensoul_core::AgentId;
use serde::{Deserialize, Serialize};

/// 通道类型 — 信号通道仅引擎可见，报告通道仅用户可见
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// 信号通道 — 结构化 JSON，引擎读取
    Signal,
    /// 报告通道 — 自然语言，用户读取
    Report,
}

/// Agent 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// 消息唯一 ID
    pub msg_id: String,
    /// 通道类型
    pub channel: ChannelType,
    /// 发送方 Agent
    pub from_agent: AgentId,
    /// 接收方 Agent
    pub to_agent: AgentId,
    /// 信号载荷 — 仅 signal 通道有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalPayload>,
    /// 报告内容 — 仅 report 通道有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    /// 消息元数据
    #[serde(default)]
    pub metadata: MessageMetadata,
    /// 时间戳
    pub timestamp: f64,
}

/// 信号通道载荷 — 仅引擎读取，结构化审查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPayload {
    /// 是否通过
    pub pass: bool,
    /// 综合评分
    pub score: Option<f32>,
    /// 严重级别统计
    pub severity_levels: Option<SeverityLevels>,
    /// 是否需要重试
    pub retry: bool,
    /// 扩展字段
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// 严重级别统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeverityLevels {
    /// 严重问题数量
    pub critical: u32,
    /// 警告数量
    pub warning: u32,
    /// 信息提示数量
    pub info: u32,
}

/// 消息元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// 使用的模型
    pub model_used: Option<String>,
    /// 处理耗时（毫秒）
    pub duration_ms: Option<u64>,
}

impl AgentMessage {
    /// 创建信号消息
    pub fn signal(
        from: AgentId,
        to: AgentId,
        signal: SignalPayload,
    ) -> Self {
        Self {
            msg_id: uuid::Uuid::new_v4().to_string(),
            channel: ChannelType::Signal,
            from_agent: from,
            to_agent: to,
            signal: Some(signal),
            report: None,
            metadata: MessageMetadata::default(),
            timestamp: timestamp_now(),
        }
    }

    /// 创建报告消息
    pub fn report(
        from: AgentId,
        to: AgentId,
        content: String,
    ) -> Self {
        Self {
            msg_id: uuid::Uuid::new_v4().to_string(),
            channel: ChannelType::Report,
            from_agent: from,
            to_agent: to,
            signal: None,
            report: Some(content),
            metadata: MessageMetadata::default(),
            timestamp: timestamp_now(),
        }
    }
}

impl SignalPayload {
    /// 创建通过信号
    pub fn passed(score: f32) -> Self {
        Self {
            pass: true,
            score: Some(score),
            severity_levels: None,
            retry: false,
            extra: serde_json::Value::Null,
        }
    }

    /// 创建失败信号
    pub fn failed(severity_levels: SeverityLevels) -> Self {
        Self {
            pass: false,
            score: None,
            severity_levels: Some(severity_levels),
            retry: false,
            extra: serde_json::Value::Null,
        }
    }
}

/// 获取当前时间戳
fn timestamp_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
