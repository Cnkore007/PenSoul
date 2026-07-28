/// PenSoul 双通道实现 — 信号通道与报告通道的硬隔离
use pensoul_core::PensoulError;
use pensoul_core::Result;
use std::collections::HashMap;

use crate::message::{AgentMessage, ChannelType};

/// 通道处理器类型
type Handler = Box<dyn Fn(&AgentMessage) -> Result<()> + Send + Sync>;

/// 信号通道 — 仅引擎可见，处理结构化审查结果
pub struct SignalChannel {
    handlers: HashMap<String, Handler>,
}

impl Default for SignalChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalChannel {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// 注册信号处理器
    pub fn register<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.handlers.insert(agent_id.to_string(), Box::new(handler));
    }

    /// 分发信号消息
    pub fn dispatch(&self, msg: &AgentMessage) -> Result<()> {
        let handler = self.handlers.get(msg.to_agent.as_str()).ok_or_else(|| {
            PensoulError::Internal(format!("信号处理器未注册: {}", msg.to_agent))
        })?;
        handler(msg)
    }

    /// 检查处理器是否已注册
    pub fn has_handler(&self, agent_id: &str) -> bool {
        self.handlers.contains_key(agent_id)
    }
}

/// 报告通道 — 仅用户可见，处理自然语言报告
pub struct ReportChannel {
    handlers: HashMap<String, Handler>,
}

impl Default for ReportChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportChannel {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// 注册报告处理器
    pub fn register<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.handlers.insert(agent_id.to_string(), Box::new(handler));
    }

    /// 分发报告消息
    pub fn dispatch(&self, msg: &AgentMessage) -> Result<()> {
        let handler = self.handlers.get(msg.to_agent.as_str()).ok_or_else(|| {
            PensoulError::Internal(format!("报告处理器未注册: {}", msg.to_agent))
        })?;
        handler(msg)
    }

    /// 检查处理器是否已注册
    pub fn has_handler(&self, agent_id: &str) -> bool {
        self.handlers.contains_key(agent_id)
    }
}

/// 双通道管理器 — 统一管理信号通道和报告通道
pub struct DualChannel {
    pub signal: SignalChannel,
    pub report: ReportChannel,
    message_log: Vec<AgentMessage>,
}

impl Default for DualChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DualChannel {
    pub fn new() -> Self {
        Self {
            signal: SignalChannel::new(),
            report: ReportChannel::new(),
            message_log: Vec::new(),
        }
    }

    /// 发送消息 — 根据通道类型路由
    pub fn send(&mut self, msg: AgentMessage) -> Result<()> {
        self.message_log.push(msg.clone());
        match msg.channel {
            ChannelType::Signal => self.signal.dispatch(&msg),
            ChannelType::Report => self.report.dispatch(&msg),
        }
    }

    /// 获取所有信号消息
    pub fn signal_messages(&self) -> Vec<&AgentMessage> {
        self.message_log
            .iter()
            .filter(|m| m.channel == ChannelType::Signal)
            .collect()
    }

    /// 获取所有报告消息
    pub fn report_messages(&self) -> Vec<&AgentMessage> {
        self.message_log
            .iter()
            .filter(|m| m.channel == ChannelType::Report)
            .collect()
    }

    /// 获取消息总数
    pub fn message_count(&self) -> usize {
        self.message_log.len()
    }
}
