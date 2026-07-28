/// PenSoul 智能体通信系统
///
/// 实现双通道通信协议（signal/report）、6 个预置 Agent 定义、通道路由器。
/// 信号通道仅引擎可见（结构化 JSON），报告通道仅用户可见（自然语言）。
pub mod message;
pub mod channel;
pub mod router;
pub mod agents;
pub mod protocol;

pub use message::{AgentMessage, ChannelType, MessageMetadata, SeverityLevels, SignalPayload};
pub use channel::DualChannel;
pub use router::ChannelRouter;
pub use agents::{AgentDefinition, AgentType};
pub use protocol::{agent_message_schema, validate_message};

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::AgentId;
    use std::sync::{Arc, Mutex};

    // ─── 验收标准 #34: signal 通道路由到引擎 ────────────────────

    #[test]
    fn test_signal_channel_routes_to_engine() {
        let mut router = ChannelRouter::new();
        let engine_id = "engine";
        let received = Arc::new(Mutex::new(false));
        let received_clone = received.clone();

        router.register_signal_handler(engine_id, move |msg| {
            assert_eq!(msg.channel, ChannelType::Signal);
            assert!(msg.signal.is_some());
            *received_clone.lock().unwrap() = true;
            Ok(())
        });

        let msg = AgentMessage::signal(
            AgentId::new("consistency_auditor"),
            AgentId::new(engine_id),
            SignalPayload::passed(0.95),
        );

        router.send(msg).unwrap();
        assert!(*received.lock().unwrap());
    }

    // ─── 验收标准 #35: report 通道路由到 UI ──────────────────────

    #[test]
    fn test_report_channel_routes_to_ui() {
        let mut router = ChannelRouter::new();
        let ui_id = "ui";
        let received_content = Arc::new(Mutex::new(String::new()));
        let content_clone = received_content.clone();

        router.register_report_handler(ui_id, move |msg| {
            assert_eq!(msg.channel, ChannelType::Report);
            assert!(msg.report.is_some());
            *content_clone.lock().unwrap() = msg.report.clone().unwrap();
            Ok(())
        });

        let msg = AgentMessage::report(
            AgentId::new("style_analyzer"),
            AgentId::new(ui_id),
            "文风审查通过，评分 0.88".into(),
        );

        router.send(msg).unwrap();
        assert_eq!(*received_content.lock().unwrap(), "文风审查通过，评分 0.88");
    }

    // ─── 验收标准 #36: JSON 序列化/反序列化 round-trip ──────────

    #[test]
    fn test_json_round_trip_signal_message() {
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

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();

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

    #[test]
    fn test_json_round_trip_report_message() {
        let msg = AgentMessage::report(
            AgentId::new("agent_b"),
            AgentId::new("ui"),
            "审查报告内容".into(),
        );

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.msg_id, deserialized.msg_id);
        assert_eq!(msg.channel, deserialized.channel);
        assert_eq!(msg.report, deserialized.report);
    }

    #[test]
    fn test_json_round_trip_channel_type_enum() {
        let signal = ChannelType::Signal;
        let report = ChannelType::Report;

        let json_signal = serde_json::to_string(&signal).unwrap();
        let json_report = serde_json::to_string(&report).unwrap();

        assert_eq!(json_signal, "\"signal\"");
        assert_eq!(json_report, "\"report\"");

        let deserialized_signal: ChannelType = serde_json::from_str(&json_signal).unwrap();
        let deserialized_report: ChannelType = serde_json::from_str(&json_report).unwrap();

        assert_eq!(deserialized_signal, ChannelType::Signal);
        assert_eq!(deserialized_report, ChannelType::Report);
    }

    // ─── 验收标准 #37: 多消息路由 ────────────────────────────────

    #[test]
    fn test_multi_message_routing() {
        let mut router = ChannelRouter::new();
        let signal_count = Arc::new(Mutex::new(0u32));
        let report_count = Arc::new(Mutex::new(0u32));

        let signal_count_clone = signal_count.clone();
        router.register_signal_handler("engine", move |_| {
            *signal_count_clone.lock().unwrap() += 1;
            Ok(())
        });

        let report_count_clone = report_count.clone();
        router.register_report_handler("ui", move |_| {
            *report_count_clone.lock().unwrap() += 1;
            Ok(())
        });

        // 发送 3 条信号消息
        for _ in 0..3 {
            let msg = AgentMessage::signal(
                AgentId::new("auditor"),
                AgentId::new("engine"),
                SignalPayload::passed(0.9),
            );
            router.send(msg).unwrap();
        }

        // 发送 3 条报告消息
        for _ in 0..3 {
            let msg = AgentMessage::report(
                AgentId::new("analyzer"),
                AgentId::new("ui"),
                "报告内容".into(),
            );
            router.send(msg).unwrap();
        }

        assert_eq!(*signal_count.lock().unwrap(), 3);
        assert_eq!(*report_count.lock().unwrap(), 3);
        assert_eq!(router.message_count(), 6);
        assert_eq!(router.get_signal_messages().len(), 3);
        assert_eq!(router.get_report_messages().len(), 3);
    }

    // ─── 验收标准 #38: 6 个预置 Agent 定义完整 ──────────────────

    #[test]
    fn test_six_preset_agents_complete() {
        let all_agents = AgentDefinition::all_presets();
        assert_eq!(all_agents.len(), 6);

        // 检查每个 Agent 都有完整字段
        for agent in &all_agents {
            assert!(!agent.agent_id.as_str().is_empty(), "agent_id 不能为空");
            assert!(!agent.display_name.is_empty(), "display_name 不能为空");
            assert!(!agent.description.is_empty(), "description 不能为空");
            assert!(!agent.model_preference.is_empty(), "model_preference 不能为空");
            assert!(!agent.tools_allowed.is_empty(), "tools_allowed 不能为空");
            assert!(!agent.signal_fields.is_empty(), "signal_fields 不能为空");
            assert!(!agent.system_prompt.is_empty(), "system_prompt 不能为空");
        }
    }

    #[test]
    fn test_all_agent_types_have_presets() {
        let types = [
            AgentType::ConsistencyAuditor,
            AgentType::StyleAnalyzer,
            AgentType::ForeshadowTracker,
            AgentType::DialoguePolisher,
            AgentType::PlotArchitect,
            AgentType::WorldBuilder,
        ];

        for agent_type in types {
            let agent = AgentDefinition::preset(agent_type);
            assert_eq!(agent.agent_type, agent_type);
            assert!(!agent.agent_id.as_str().is_empty());
        }
    }

    #[test]
    fn test_agent_tool_access_control() {
        let agent = AgentDefinition::preset(AgentType::ConsistencyAuditor);
        assert!(agent.is_tool_allowed("read_chapter"));
        assert!(agent.is_tool_allowed("run_consistency_check"));
        assert!(!agent.is_tool_allowed("write_chapter"));
    }

    #[test]
    fn test_agent_signal_fields() {
        let agent = AgentDefinition::preset(AgentType::ConsistencyAuditor);
        assert!(agent.has_signal_field("pass"));
        assert!(agent.has_signal_field("score"));
        assert!(agent.has_signal_field("issues"));
        assert!(!agent.has_signal_field("invalid_field"));
    }

    // ─── 验收标准 #39: 信号与报告通道隔离 ──────────────────────

    #[test]
    fn test_signal_report_channel_isolation() {
        let mut router = ChannelRouter::new();
        let signal_received = Arc::new(Mutex::new(false));
        let report_received = Arc::new(Mutex::new(false));

        let signal_clone = signal_received.clone();
        router.register_signal_handler("engine", move |_| {
            *signal_clone.lock().unwrap() = true;
            Ok(())
        });

        let report_clone = report_received.clone();
        router.register_report_handler("ui", move |_| {
            *report_clone.lock().unwrap() = true;
            Ok(())
        });

        // 发送信号消息 — 只有信号处理器应该收到
        let signal_msg = AgentMessage::signal(
            AgentId::new("auditor"),
            AgentId::new("engine"),
            SignalPayload::passed(0.9),
        );
        router.send(signal_msg).unwrap();

        assert!(*signal_received.lock().unwrap());
        assert!(!*report_received.lock().unwrap());

        // 重置
        *signal_received.lock().unwrap() = false;

        // 发送报告消息 — 只有报告处理器应该收到
        let report_msg = AgentMessage::report(
            AgentId::new("analyzer"),
            AgentId::new("ui"),
            "报告".into(),
        );
        router.send(report_msg).unwrap();

        assert!(*report_received.lock().unwrap());
        assert!(!*signal_received.lock().unwrap());
    }

    #[test]
    fn test_signal_handler_not_receiving_report_messages() {
        let mut router = ChannelRouter::new();
        let engine_signal_count = Arc::new(Mutex::new(0u32));
        let _engine_report_count = Arc::new(Mutex::new(0u32));

        let signal_clone = engine_signal_count.clone();
        router.register_signal_handler("engine", move |_| {
            *signal_clone.lock().unwrap() += 1;
            Ok(())
        });

        // 注意：engine 没有注册 report handler

        // 发送报告消息到 engine — 应该失败
        let report_msg = AgentMessage::report(
            AgentId::new("analyzer"),
            AgentId::new("engine"),
            "报告".into(),
        );
        let result = router.send(report_msg);
        assert!(result.is_err());

        // 发送信号消息到 engine — 应该成功
        let signal_msg = AgentMessage::signal(
            AgentId::new("auditor"),
            AgentId::new("engine"),
            SignalPayload::passed(0.9),
        );
        router.send(signal_msg).unwrap();
        assert_eq!(*engine_signal_count.lock().unwrap(), 1);
    }

    // ─── DualChannel 测试 ────────────────────────────────────────

    #[test]
    fn test_dual_channel_basic() {
        let mut dual = DualChannel::new();
        let received = Arc::new(Mutex::new(String::new()));
        let clone = received.clone();

        dual.signal.register("engine", move |msg| {
            if let Some(signal) = &msg.signal {
                *clone.lock().unwrap() = format!("signal:pass={}", signal.pass);
            }
            Ok(())
        });

        let msg = AgentMessage::signal(
            AgentId::new("auditor"),
            AgentId::new("engine"),
            SignalPayload::passed(0.9),
        );
        dual.send(msg).unwrap();

        assert_eq!(*received.lock().unwrap(), "signal:pass=true");
        assert_eq!(dual.message_count(), 1);
        assert_eq!(dual.signal_messages().len(), 1);
    }

    // ─── 未注册处理器错误测试 ────────────────────────────────────

    #[test]
    fn test_unregistered_signal_handler_error() {
        let mut router = ChannelRouter::new();
        // 不注册任何处理器

        let msg = AgentMessage::signal(
            AgentId::new("auditor"),
            AgentId::new("nonexistent_engine"),
            SignalPayload::passed(0.9),
        );

        let result = router.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregistered_report_handler_error() {
        let mut router = ChannelRouter::new();
        // 不注册任何处理器

        let msg = AgentMessage::report(
            AgentId::new("analyzer"),
            AgentId::new("nonexistent_ui"),
            "报告".into(),
        );

        let result = router.send(msg);
        assert!(result.is_err());
    }

    // ─── 消息查询测试 ────────────────────────────────────────────

    #[test]
    fn test_message_log_queries() {
        let mut router = ChannelRouter::new();
        router.register_signal_handler("engine", |_| Ok(()));
        router.register_report_handler("ui", |_| Ok(()));

        // 混合发送消息
        for i in 0..5 {
            let signal_msg = AgentMessage::signal(
                AgentId::new("auditor"),
                AgentId::new("engine"),
                SignalPayload::passed(0.9),
            );
            router.send(signal_msg).unwrap();

            let report_msg = AgentMessage::report(
                AgentId::new("analyzer"),
                AgentId::new("ui"),
                format!("报告 {}", i),
            );
            router.send(report_msg).unwrap();
        }

        assert_eq!(router.message_count(), 10);
        assert_eq!(router.get_signal_messages().len(), 5);
        assert_eq!(router.get_report_messages().len(), 5);
    }
}
