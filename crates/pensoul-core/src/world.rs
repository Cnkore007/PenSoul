/// Layer 1 世界层类型定义
use crate::id::{EventId, LocationId, SettingId, WorldId, ChapterId};

/// 世界层
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldLayer {
    /// 世界 ID
    pub world_id: WorldId,
    /// 世界名称
    pub name: String,
    /// 空间模型
    pub spatial_model: SpatialModel,
    /// 时间线
    pub timeline: Timeline,
    /// 世界设定规则
    pub setting_rules: Vec<SettingRule>,
    /// 术语表
    pub glossary: Vec<TerminologyEntry>,
    /// 物品图
    pub item_graph: Vec<ItemNode>,
}

/// 空间模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpatialModel {
    /// 位置列表
    pub locations: Vec<Location>,
    /// 层级关系
    pub hierarchy: Vec<(LocationId, LocationId)>,
}

/// 位置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    /// 位置 ID
    pub id: LocationId,
    /// 位置名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 空间标签
    pub spatial_tags: Vec<String>,
}

/// 时间线
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Timeline {
    /// 时间线事件
    pub events: Vec<TimelineEvent>,
    /// 纪元标记
    pub epoch_markers: Vec<EpochMarker>,
}

/// 时间线事件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    /// 事件 ID
    pub event_id: EventId,
    /// 故事时间
    pub story_time: String,
    /// 所属章节 ID
    #[serde(deserialize_with = "crate::id::flexible_id::deserialize_chapter_id")]
    pub chapter_id: ChapterId,
    /// 事件描述
    pub description: String,
    /// 参与者
    pub participants: Vec<String>,
}

/// 纪元标记
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EpochMarker {
    /// 纪元名称
    pub name: String,
    /// 故事时间
    pub story_time: String,
    /// 纪元描述
    pub description: String,
}

/// 世界设定规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingRule {
    /// 规则 ID
    pub rule_id: SettingId,
    /// 规则分类
    pub category: String,
    /// 规则标题
    pub title: String,
    /// 规则描述
    pub description: String,
    /// 约束条件
    pub constraints: Vec<String>,
}

/// 术语表条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologyEntry {
    /// 术语
    pub term: String,
    /// 定义
    pub definition: String,
    /// 别名
    pub aliases: Vec<String>,
    /// 分类
    pub category: String,
}

/// 物品节点
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemNode {
    /// 物品 ID
    pub item_id: String,
    /// 物品名称
    pub name: String,
    /// 物品描述
    pub description: String,
    /// 物品属性
    pub properties: std::collections::HashMap<String, String>,
    /// 当前所有者
    pub owner: String,
}
