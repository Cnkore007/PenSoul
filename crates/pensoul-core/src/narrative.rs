/// Layer 3 叙事层类型定义
use crate::id::{ChapterId, CharacterId, EventId, ForeshadowId};

/// 叙事层
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeLayer {
    /// 情节图
    pub plot_graph: Vec<PlotNode>,
    /// 伏笔列表
    pub foreshadows: Vec<Foreshadow>,
    /// 冲突列表
    pub conflicts: Vec<Conflict>,
    /// 情感曲线
    pub emotional_arcs: Vec<EmotionalArc>,
}

/// 情节节点
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotNode {
    /// 事件 ID
    pub event_id: EventId,
    /// 所属章节 ID
    pub chapter_id: ChapterId,
    /// 节点标题
    pub title: String,
    /// 节点描述
    pub description: String,
    /// 因果来源
    pub causality_from: Vec<EventId>,
    /// 因果去向
    pub causality_to: Vec<EventId>,
}

/// 伏笔
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Foreshadow {
    /// 伏笔 ID
    pub id: ForeshadowId,
    /// 伏笔名称
    pub name: String,
    /// 伏笔描述
    pub description: String,
    /// 伏笔状态
    pub status: ForeshadowStatus,
    /// 埋设章节
    pub planted_chapter: ChapterId,
    /// 预期解决章节
    pub expected_resolve_chapter: Option<ChapterId>,
    /// 实际解决章节
    pub actual_resolve_chapter: Option<ChapterId>,
    /// 相关角色
    pub related_characters: Vec<CharacterId>,
    /// 相关物品
    pub related_items: Vec<String>,
}

/// 伏笔状态
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ForeshadowStatus {
    /// 计划中
    Planned,
    /// 已埋设
    Planted,
    /// 进展中
    Progressing,
    /// 已解决
    Resolved,
    /// 已放弃
    Abandoned,
    /// 已过期
    Overdue,
}

/// 冲突
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conflict {
    /// 冲突 ID
    pub conflict_id: String,
    /// 冲突方
    pub parties: Vec<String>,
    /// 所属章节 ID
    pub chapter_id: ChapterId,
    /// 冲突描述
    pub description: String,
    /// 解决方案
    pub resolution: Option<String>,
    /// 解决章节
    pub resolution_chapter: Option<ChapterId>,
}

/// 情感曲线
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalArc {
    /// 角色 ID
    pub character_id: CharacterId,
    /// 数据点 (章节, 情感强度)
    pub data_points: Vec<(i64, f32)>,
}

/// 情节脉络节点 —— 大纲规划层
///
/// 一个节点覆盖一个章节范围（如「第1-200章」），描述该故事段的
/// 整体剧情规划。它不是章节本身：通过「展开细纲」按范围逐批生成
/// 真正的逐章梗概（章节实体），造化工坊再按章节梗概写作正文。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlineArc {
    /// 节点 ID
    pub arc_id: String,
    /// 节点标题（如「枯井边的勘验」）
    pub title: String,
    /// 该故事段的剧情规划
    #[serde(default)]
    pub description: String,
    /// 覆盖的起始章号（含，从 1 开始）
    #[serde(default)]
    pub chapter_start: i64,
    /// 覆盖的结束章号（含）
    #[serde(default)]
    pub chapter_end: i64,
    /// 已展开细纲到第几章（0 表示尚未展开；>= chapter_end 表示全部展开）
    #[serde(default)]
    pub expanded_until: i64,
}
