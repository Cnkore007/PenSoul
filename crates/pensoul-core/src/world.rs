/// Layer 1 世界层类型定义
use crate::id::{ChapterId, EventId, LocationId, SettingId, WorldId};

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
    #[serde(default)]
    pub spatial_tags: Vec<String>,
}

/// 时间线
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Timeline {
    /// 时间线事件
    pub events: Vec<TimelineEvent>,
    /// 纪元标记
    #[serde(default)]
    pub epoch_markers: Vec<EpochMarker>,
}

/// 时间线事件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    /// 事件 ID
    pub event_id: EventId,
    /// 故事时间
    pub story_time: String,
    /// 所属章节 ID（世界观阶段的事件可不归属章节）
    #[serde(
        default = "default_event_chapter_id",
        deserialize_with = "crate::id::flexible_id::deserialize_chapter_id"
    )]
    pub chapter_id: ChapterId,
    /// 事件描述
    pub description: String,
    /// 参与者
    #[serde(default)]
    pub participants: Vec<String>,
}

/// 时间线事件缺失章节归属时的默认值
fn default_event_chapter_id() -> ChapterId {
    ChapterId::new("")
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
    #[serde(default)]
    pub category: String,
    /// 规则标题
    pub title: String,
    /// 规则描述
    pub description: String,
    /// 约束条件
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 前端讨论成果导入的数据只含核心字段，
    /// 可选集合/分类字段缺失时必须能反序列化（否则保存静默失败、重启丢失）。
    #[test]
    fn test_world_layer_deserialize_with_missing_optional_fields() {
        let json = r#"{
            "world_id": "default",
            "name": "default",
            "spatial_model": {
                "locations": [{"id": "loc-1", "name": "临淄城", "description": "都城"}],
                "hierarchy": []
            },
            "timeline": {
                "events": [{"event_id": "evt-1", "story_time": "元年春", "description": "开局"}]
            },
            "setting_rules": [{"rule_id": "rule-1", "title": "灵气守恒", "description": "灵气不可凭空产生"}],
            "glossary": [],
            "item_graph": []
        }"#;
        let layer: WorldLayer = serde_json::from_str(json).unwrap();
        assert_eq!(layer.spatial_model.locations.len(), 1);
        assert!(layer.spatial_model.locations[0].spatial_tags.is_empty());
        assert_eq!(layer.timeline.events.len(), 1);
        assert_eq!(layer.timeline.events[0].chapter_id.as_str(), "");
        assert!(layer.timeline.events[0].participants.is_empty());
        assert!(layer.timeline.epoch_markers.is_empty());
        assert_eq!(layer.setting_rules.len(), 1);
        assert!(layer.setting_rules[0].category.is_empty());
        assert!(layer.setting_rules[0].constraints.is_empty());
    }

    /// 序列化后必须能无损 round-trip，保证保存-重启链路一致。
    #[test]
    fn test_world_layer_serde_round_trip() {
        let layer = WorldLayer {
            world_id: WorldId::new("w1"),
            name: "测试世界".into(),
            spatial_model: SpatialModel {
                locations: vec![Location {
                    id: LocationId::new("loc-1"),
                    name: "山谷".into(),
                    description: "幽静".into(),
                    spatial_tags: vec!["山".into()],
                }],
                hierarchy: vec![],
            },
            timeline: Timeline {
                events: vec![TimelineEvent {
                    event_id: EventId::new("evt-1"),
                    story_time: "第三年".into(),
                    chapter_id: ChapterId::new("2"),
                    description: "大战".into(),
                    participants: vec!["甲".into()],
                }],
                epoch_markers: vec![],
            },
            setting_rules: vec![],
            glossary: vec![],
            item_graph: vec![],
        };
        let json = serde_json::to_string(&layer).unwrap();
        let back: WorldLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.timeline.events[0].chapter_id.as_str(), "2");
        assert_eq!(back.spatial_model.locations[0].spatial_tags, vec!["山"]);
    }
}
