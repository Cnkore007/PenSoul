//! PenSoul 智能体消息类型与 JSON Schema 定义
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
    pub fn signal(from: AgentId, to: AgentId, signal: SignalPayload) -> Self {
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
    pub fn report(from: AgentId, to: AgentId, content: String) -> Self {
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

/// 消息 JSON Schema — 用于验证和文档
pub fn agent_message_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "AgentMessage",
        "description": "PenSoul 智能体消息格式",
        "type": "object",
        "required": ["msg_id", "channel", "from_agent", "to_agent", "timestamp"],
        "properties": {
            "msg_id": {
                "type": "string",
                "description": "消息唯一 ID"
            },
            "channel": {
                "type": "string",
                "enum": ["signal", "report"],
                "description": "通道类型：signal=引擎可见，report=用户可见"
            },
            "from_agent": {
                "type": "string",
                "description": "发送方 Agent ID"
            },
            "to_agent": {
                "type": "string",
                "description": "接收方 Agent ID"
            },
            "signal": {
                "$ref": "#/definitions/SignalPayload",
                "description": "信号载荷 — 仅 signal 通道有效"
            },
            "report": {
                "type": "string",
                "description": "报告内容 — 仅 report 通道有效"
            },
            "metadata": {
                "$ref": "#/definitions/MessageMetadata"
            },
            "timestamp": {
                "type": "number",
                "description": "Unix 时间戳"
            }
        },
        "definitions": {
            "SignalPayload": {
                "type": "object",
                "required": ["pass", "retry"],
                "properties": {
                    "pass": {
                        "type": "boolean",
                        "description": "是否通过审查"
                    },
                    "score": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "综合评分 (0.0-1.0)"
                    },
                    "severity_levels": {
                        "$ref": "#/definitions/SeverityLevels"
                    },
                    "retry": {
                        "type": "boolean",
                        "description": "是否需要重试"
                    },
                    "extra": {
                        "type": "object",
                        "description": "扩展字段"
                    }
                }
            },
            "SeverityLevels": {
                "type": "object",
                "properties": {
                    "critical": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "严重问题数量"
                    },
                    "warning": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "警告数量"
                    },
                    "info": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "信息提示数量"
                    }
                }
            },
            "MessageMetadata": {
                "type": "object",
                "properties": {
                    "model_used": {
                        "type": "string",
                        "description": "使用的模型名称"
                    },
                    "duration_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "处理耗时（毫秒）"
                    }
                }
            }
        }
    })
}

/// 验证消息是否符合 schema 基本结构
pub fn validate_message(msg: &AgentMessage) -> bool {
    // 基本字段检查
    if msg.msg_id.is_empty() {
        return false;
    }
    if msg.timestamp <= 0.0 {
        return false;
    }
    // 通道与载荷一致性
    match msg.channel {
        ChannelType::Signal => msg.signal.is_some(),
        ChannelType::Report => msg.report.is_some(),
    }
}

/// 将消息序列化为 JSON 字符串
pub fn message_to_json(msg: &AgentMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(msg)
}

/// 从 JSON 字符串反序列化消息
pub fn message_from_json(json: &str) -> Result<AgentMessage, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::AgentId;

    #[test]
    fn test_schema_is_valid_json() {
        let schema = agent_message_schema();
        // 确保 schema 可以序列化为有效 JSON
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("$schema"));
        assert!(json.contains("AgentMessage"));
    }

    #[test]
    fn test_validate_signal_message() {
        let msg = AgentMessage::signal(
            AgentId::new("agent_a"),
            AgentId::new("engine"),
            SignalPayload::passed(0.95),
        );
        assert!(validate_message(&msg));
    }

    #[test]
    fn test_validate_report_message() {
        let msg = AgentMessage::report(
            AgentId::new("agent_a"),
            AgentId::new("ui"),
            "审查通过".into(),
        );
        assert!(validate_message(&msg));
    }

    #[test]
    fn test_validate_invalid_message() {
        let msg = AgentMessage {
            msg_id: String::new(), // 空 ID
            channel: ChannelType::Signal,
            from_agent: AgentId::new("agent_a"),
            to_agent: AgentId::new("engine"),
            signal: Some(SignalPayload::passed(0.9)),
            report: None,
            metadata: MessageMetadata::default(),
            timestamp: 1000.0,
        };
        assert!(!validate_message(&msg));
    }

    #[test]
    fn test_round_trip_json() {
        let msg = AgentMessage::signal(
            AgentId::new("agent_a"),
            AgentId::new("engine"),
            SignalPayload {
                pass: true,
                score: Some(0.92),
                severity_levels: Some(SeverityLevels {
                    critical: 0,
                    warning: 2,
                    info: 5,
                }),
                retry: false,
                extra: serde_json::json!({"details": "test"}),
            },
        );

        let json = message_to_json(&msg).unwrap();
        let deserialized = message_from_json(&json).unwrap();

        assert_eq!(msg.msg_id, deserialized.msg_id);
        assert_eq!(msg.channel, deserialized.channel);
        assert_eq!(msg.from_agent, deserialized.from_agent);
        assert_eq!(msg.to_agent, deserialized.to_agent);
        assert_eq!(
            msg.signal.as_ref().unwrap().pass,
            deserialized.signal.as_ref().unwrap().pass
        );
        assert_eq!(
            msg.signal.as_ref().unwrap().score,
            deserialized.signal.as_ref().unwrap().score
        );
    }
}
