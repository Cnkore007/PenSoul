// sprout.rs — 灵魂萌芽会话（对话式创作工作台）
// 对话历史与待确认提案属于创作过程数据，随正典一起持久化

use serde::{Deserialize, Serialize};

/// 单条对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SproutMessage {
    /// user / assistant
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// 提案中的世界观设定
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SproutSettingProposal {
    pub name: String,
    pub category: String,
    pub description: String,
}

/// 提案中的大纲脉络
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SproutArcProposal {
    pub title: String,
    pub description: String,
    pub chapter_start: i64,
    pub chapter_end: i64,
}

/// LLM 生成的项目提案（建议制，确认后才写入正典）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SproutProposal {
    pub high_concept: String,
    pub premise: String,
    pub protagonist_hint: String,
    pub tone: String,
    pub central_conflict: String,
    pub inspiration: String,
    pub genre: String,
    pub world_rules: Vec<String>,
    pub world_settings: Vec<SproutSettingProposal>,
    pub outline_arcs: Vec<SproutArcProposal>,
}

/// 灵魂萌芽会话
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoulSproutSession {
    /// 最近 100 条对话（防止项目文件无限膨胀）
    pub messages: Vec<SproutMessage>,
    /// 待用户确认的提案；确认后清空
    pub pending_proposal: Option<SproutProposal>,
}

impl SoulSproutSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加消息并截断到最近 100 条
    pub fn push_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(SproutMessage {
            role: role.into(),
            content: content.into(),
            created_at: chrono::Local::now().to_rfc3339(),
        });
        let overflow = self.messages.len().saturating_sub(100);
        if overflow > 0 {
            self.messages.drain(..overflow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ProjectId;
    use crate::ontology::NovelOntology;

    #[test]
    fn old_project_without_sprout_field_still_loads() {
        let mut ontology = NovelOntology::new(ProjectId::new("old"), "旧书");
        ontology.soul_sprout.push_message("user", "旧对话");
        let mut json = serde_json::to_value(&ontology).expect("序列化失败");
        json.as_object_mut()
            .expect("对象")
            .remove("soul_sprout");

        let loaded: NovelOntology = serde_json::from_value(json).expect("旧数据应可加载");
        assert!(loaded.soul_sprout.messages.is_empty());
        assert!(loaded.soul_sprout.pending_proposal.is_none());
    }

    #[test]
    fn message_history_truncates_to_100() {
        let mut session = SoulSproutSession::new();
        for i in 0..120 {
            session.push_message("user", format!("消息 {i}"));
        }
        assert_eq!(session.messages.len(), 100);
        assert_eq!(session.messages[0].content, "消息 20");
    }
}
