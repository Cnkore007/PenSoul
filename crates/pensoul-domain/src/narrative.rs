// narrative.rs — 情节脉络类型定义
// 情节节点、伏笔、冲突、情感弧线

use crate::id::*;
use serde::{Deserialize, Serialize};

/// 情节节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotNode {
    pub event_id: EventId,
    pub chapter_id: ChapterId,
    pub title: String,
    pub description: String,
    pub causality_from: Vec<EventId>,
    pub causality_to: Vec<EventId>,
}

/// 情节脉络节点（覆盖章节范围的大纲）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineArc {
    pub arc_id: String,
    pub title: String,
    pub description: String,
    pub chapter_start: i64,
    pub chapter_end: i64,
    pub volume_id: String,
    pub expanded_until: i64,
}

impl OutlineArc {
    pub fn new(
        title: impl Into<String>,
        chapter_start: i64,
        chapter_end: i64,
    ) -> Self {
        Self {
            arc_id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            chapter_start,
            chapter_end,
            volume_id: String::new(),
            expanded_until: 0,
        }
    }

    /// 覆盖的章节数
    pub fn chapter_count(&self) -> i64 {
        self.chapter_end - self.chapter_start + 1
    }
}

/// 冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub conflict_id: String,
    pub parties: Vec<String>,
    pub chapter_id: ChapterId,
    pub description: String,
    pub resolution: Option<String>,
    pub resolution_chapter: Option<ChapterId>,
}

/// 情感弧线数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalArc {
    pub character_id: CharacterId,
    pub data_points: Vec<(i64, f32)>, // (章节号, 情感值)
}

/// 写作经验教训
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingLesson {
    pub lesson_id: String,
    pub category: String,
    pub problem: String,
    pub fix: String,
    pub example: String,
    pub count: u32,
    pub created_at: String,
    pub scope: String,
}
