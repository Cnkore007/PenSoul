/// Agent 讨论配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentDiscussionConfig {
    /// Agent 唯一 ID
    pub id: String,
    /// Agent 显示名称
    pub name: String,
    /// 使用的模型 ID
    pub model: String,
    /// 评审提示词
    pub prompt: String,
    /// 评审维度名称
    pub perspective: String,
    /// 是否启用
    pub enabled: bool,
}

/// 萌芽数据 — 核心想法 + 讨论 Agent 配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SproutData {
    /// 用户对故事想法的自由描述
    pub idea_description: String,
    /// 讨论 Agent 配置列表
    pub agents: Vec<AgentDiscussionConfig>,
}

impl SproutData {
    pub fn new() -> Self {
        Self {
            idea_description: String::new(),
            agents: Vec::new(),
        }
    }
}

impl Default for SproutData {
    fn default() -> Self {
        Self::new()
    }
}
