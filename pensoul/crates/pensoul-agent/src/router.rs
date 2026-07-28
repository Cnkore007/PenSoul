/// PenSoul 通道路由器 — signal/report 分离路由
use pensoul_core::PensoulError;
use pensoul_core::Result;
use std::collections::HashMap;

use crate::message::{AgentMessage, ChannelType};

/// 处理器类型 — 闭包 trait
type Handler = Box<dyn Fn(&AgentMessage) -> Result<()> + Send + Sync>;

/// 通道路由器 — signal 和 report 分离
pub struct ChannelRouter {
    /// 信号处理器映射（引擎端）
    signal_handlers: HashMap<String, Handler>,
    /// 报告处理器映射（UI 端）
    report_handlers: HashMap<String, Handler>,
    /// 消息日志
    message_log: Vec<AgentMessage>,
}

impl Default for ChannelRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelRouter {
    pub fn new() -> Self {
        Self {
            signal_handlers: HashMap::new(),
            report_handlers: HashMap::new(),
            message_log: Vec::new(),
        }
    }

    /// 注册信号处理器（引擎端）
    pub fn register_signal_handler<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.signal_handlers
            .insert(agent_id.to_string(), Box::new(handler));
    }

    /// 注册报告处理器（UI 端）
    pub fn register_report_handler<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.report_handlers
            .insert(agent_id.to_string(), Box::new(handler));
    }

    /// 发送消息 — 根据 channel 类型路由到对应处理器
    pub fn send(&mut self, msg: AgentMessage) -> Result<()> {
        self.message_log.push(msg.clone());
        match msg.channel {
            ChannelType::Signal => {
                if let Some(handler) = self.signal_handlers.get(msg.to_agent.as_str()) {
                    handler(&msg)
                } else {
                    Err(PensoulError::Internal(format!(
                        "信号处理器未注册: {}",
                        msg.to_agent
                    )))
                }
            }
            ChannelType::Report => {
                if let Some(handler) = self.report_handlers.get(msg.to_agent.as_str()) {
                    handler(&msg)
                } else {
                    Err(PensoulError::Internal(format!(
                        "报告处理器未注册: {}",
                        msg.to_agent
                    )))
                }
            }
        }
    }

    /// 获取所有信号消息
    pub fn get_signal_messages(&self) -> Vec<&AgentMessage> {
        self.message_log
            .iter()
            .filter(|m| m.channel == ChannelType::Signal)
            .collect()
    }

    /// 获取所有报告消息
    pub fn get_report_messages(&self) -> Vec<&AgentMessage> {
        self.message_log
            .iter()
            .filter(|m| m.channel == ChannelType::Report)
            .collect()
    }

    /// 检查信号处理器是否已注册
    pub fn has_signal_handler(&self, agent_id: &str) -> bool {
        self.signal_handlers.contains_key(agent_id)
    }

    /// 检查报告处理器是否已注册
    pub fn has_report_handler(&self, agent_id: &str) -> bool {
        self.report_handlers.contains_key(agent_id)
    }

    /// 获取消息总数
    pub fn message_count(&self) -> usize {
        self.message_log.len()
    }
}
