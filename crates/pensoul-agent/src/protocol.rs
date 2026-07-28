/// PenSoul Agent 消息 JSON Schema 定义
#[allow(unused_imports)]
use crate::message::{AgentMessage, ChannelType, MessageMetadata, SeverityLevels, SignalPayload};

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
        assert_eq!(msg.signal.as_ref().unwrap().pass, deserialized.signal.as_ref().unwrap().pass);
        assert_eq!(msg.signal.as_ref().unwrap().score, deserialized.signal.as_ref().unwrap().score);
    }
}
