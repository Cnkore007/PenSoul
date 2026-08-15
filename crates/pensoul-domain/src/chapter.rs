// chapter.rs — 章节与卷类型定义

use crate::entity::Annotation;
use crate::id::*;
use serde::{Deserialize, Serialize};

/// 章节状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChapterStatus {
    Draft,
    Reviewing,
    Reviewed,
    Polished,
    Published,
}

impl ChapterStatus {
    /// 状态机门控：只允许按流程推进，或回退到草稿重写
    pub fn can_transition_to(&self, next: &Self) -> bool {
        use ChapterStatus::*;
        match (self, next) {
            (a, b) if a == b => true,
            (Draft, Reviewing) => true,
            (Reviewing, Reviewed) => true,
            (Reviewed, Polished) => true,
            (Polished, Published) => true,
            (_, Draft) => true,
            _ => false,
        }
    }
}

/// 章节版本快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRevision {
    pub version: i32,
    pub content: String,
    pub word_count: u32,
    pub created_at: String,
    pub reason: String,
}

/// 章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub chapter_id: ChapterId,
    /// 章节序号（顺序语义的唯一入口）
    pub chapter_no: i64,
    pub volume_id: VolumeId,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub word_count: u32,
    pub version: i32,
    pub status: ChapterStatus,
    pub consistency_score: f32,
    pub created_at: String,
    pub updated_at: String,
    pub annotations: Vec<Annotation>,
    pub revisions: Vec<ChapterRevision>,
}

impl Chapter {
    pub fn new(chapter_no: i64, title: impl Into<String>) -> Self {
        Self {
            chapter_id: ChapterId::default(),
            chapter_no,
            volume_id: VolumeId::default(),
            title: title.into(),
            summary: String::new(),
            content: String::new(),
            word_count: 0,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 1.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            annotations: Vec::new(),
            revisions: Vec::new(),
        }
    }

    /// 更新内容并递增版本
    pub fn update_content(&mut self, content: String) {
        let old_content = std::mem::replace(&mut self.content, content.clone());
        let old_word_count = self.word_count;
        self.word_count = content.chars().count() as u32;
        self.version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();

        // 保存修订历史（记录变更前的旧内容与旧字数）
        self.revisions.push(ChapterRevision {
            version: self.version - 1,
            content: old_content,
            word_count: old_word_count,
            created_at: self.updated_at.clone(),
            reason: "内容更新".to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_status_follows_workflow_gate() {
        assert!(ChapterStatus::Draft.can_transition_to(&ChapterStatus::Reviewing));
        assert!(ChapterStatus::Reviewing.can_transition_to(&ChapterStatus::Reviewed));
        assert!(ChapterStatus::Reviewed.can_transition_to(&ChapterStatus::Polished));
        assert!(ChapterStatus::Polished.can_transition_to(&ChapterStatus::Published));
        assert!(ChapterStatus::Published.can_transition_to(&ChapterStatus::Draft));
        assert!(!ChapterStatus::Draft.can_transition_to(&ChapterStatus::Published));
        assert!(!ChapterStatus::Reviewed.can_transition_to(&ChapterStatus::Reviewing));
    }

    #[test]
    fn update_content_records_revisions_and_version() {
        let mut chapter = Chapter::new(1, "第一章");
        chapter.update_content("旧正文".to_string());
        chapter.update_content("新正文内容".to_string());

        assert_eq!(chapter.version, 3);
        assert_eq!(chapter.word_count, 5);
        assert_eq!(chapter.revisions.len(), 2);
        // 第一次修订记录的是更新前的空内容与 0 字数
        assert_eq!(chapter.revisions[0].content, "");
        assert_eq!(chapter.revisions[0].word_count, 0);
        // 第二次修订记录第一次更新后的内容
        assert_eq!(chapter.revisions[1].content, "旧正文");
        assert_eq!(chapter.revisions[1].word_count, 3);
    }
}

/// 卷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub volume_id: VolumeId,
    pub title: String,
    pub chapter_ids: Vec<ChapterId>,
    pub summary: String,
    pub expanded: bool,
}

impl Volume {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            volume_id: VolumeId::default(),
            title: title.into(),
            chapter_ids: Vec::new(),
            summary: String::new(),
            expanded: false,
        }
    }
}
