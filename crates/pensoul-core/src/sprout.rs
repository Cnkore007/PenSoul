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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_empty() {
        let s = SproutData::new();
        assert!(s.idea_description.is_empty());
        assert!(s.agents.is_empty());
    }

    #[test]
    fn test_default_matches_new() {
        let a = SproutData::new();
        let b = SproutData::default();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
    }

    #[test]
    fn test_sprout_serde_round_trip_with_agents() {
        let mut s = SproutData::new();
        s.idea_description = "一个关于时间循环的故事".to_string();
        s.agents.push(AgentDiscussionConfig {
            id: "agent-1".to_string(),
            name: "逻辑评审".to_string(),
            model: "kimi".to_string(),
            prompt: "检查逻辑漏洞".to_string(),
            perspective: "逻辑".to_string(),
            enabled: true,
        });
        let json = serde_json::to_string(&s).unwrap();
        let back: SproutData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.idea_description, "一个关于时间循环的故事");
        assert_eq!(back.agents.len(), 1);
        assert!(back.agents[0].enabled);
        assert_eq!(back.agents[0].perspective, "逻辑");
    }
}
