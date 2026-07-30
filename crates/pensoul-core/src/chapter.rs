/// 章节结构类型定义
use crate::id::{ChapterId, VolumeId};

/// 章节
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chapter {
    /// 章节 ID
    pub chapter_id: ChapterId,
    /// 卷 ID
    pub volume_id: VolumeId,
    /// 章节标题
    pub title: String,
    /// 章节梗概（大纲层信息，非正文）
    #[serde(default)]
    pub summary: String,
    /// 章节内容
    pub content: String,
    /// 字数
    pub word_count: u32,
    /// 版本号
    pub version: i32,
    /// 章节状态
    pub status: ChapterStatus,
    /// 一致性分数
    pub consistency_score: f32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 章节状态
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChapterStatus {
    /// 草稿
    Draft,
    /// 审阅中
    Reviewing,
    /// 已审阅
    Reviewed,
    /// 已润色
    Polished,
    /// 已发布
    Published,
}

/// 卷
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Volume {
    /// 卷 ID
    pub volume_id: VolumeId,
    /// 卷标题
    pub title: String,
    /// 章节 ID 列表
    pub chapter_ids: Vec<ChapterId>,
    /// 卷摘要
    pub summary: String,
}
