// entity.rs — 实体类型定义
// 角色、事件、设定、伏笔等核心实体

use crate::id::*;
use serde::{Deserialize, Serialize};

/// 实体类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Character,
    Event,
    Setting,
    Foreshadow,
    Organization,
}

/// 实体引用（用于关系连接）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub label: Option<String>,
}

impl EntityRef {
    pub fn new(entity_type: EntityType, entity_id: impl Into<String>) -> Self {
        Self {
            entity_type,
            entity_id: entity_id.into(),
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// 实体状态快照（时间感知）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// 状态所在章节
    pub chapter_id: i64,
    /// 故事内时间
    pub story_time: String,
    /// 状态数据（不同实体类型结构不同）
    pub data: serde_json::Value,
}

/// 批注/注释
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub annotation_id: String,
    pub kind: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

// ---- 角色实体 ----

/// 角色属性
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterProperties {
    pub age: Option<i32>,
    pub occupation: Option<String>,
    pub personality: Vec<(String, f32)>,
    pub appearance: Option<String>,
    pub backstory: Option<String>,
    pub wants: Option<String>,
    pub fears: Option<String>,
    pub secret: Option<String>,
    pub speech_style: Option<String>,
    // P0 档案化扩展（人物档案）
    /// 衣着/服饰
    pub attire: Option<String>,
    /// 功法/能力
    pub techniques: Vec<String>,
    /// 境界/修为
    pub realm: Option<String>,
    /// 法宝/随身物品
    pub items: Vec<String>,
}

/// 状态转换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: String,
    pub chapter_id: i64,
    pub story_time: String,
    pub causality: String,
}

/// 角色实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub properties: CharacterProperties,
    pub states: Vec<EntityState>,
    pub history: Vec<StateTransition>,
    pub annotations: Vec<Annotation>,
}

impl Character {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: CharacterId::default(),
            name: name.into(),
            properties: CharacterProperties::default(),
            states: Vec::new(),
            history: Vec::new(),
            annotations: Vec::new(),
        }
    }

    /// 获取指定章节时的角色状态
    pub fn state_at(&self, chapter_id: i64) -> Option<&EntityState> {
        self.states.iter().rev().find(|s| s.chapter_id <= chapter_id)
    }
}

// ---- 事件实体 ----

/// 事件实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub name: String,
    pub participants: Vec<EntityRef>,
    pub consequences: Vec<EntityRef>,
    pub chapter_id: i64,
    pub story_time: String,
    pub description: String,
}

impl Event {
    pub fn new(name: impl Into<String>, chapter_id: i64) -> Self {
        Self {
            id: EventId::default(),
            name: name.into(),
            participants: Vec::new(),
            consequences: Vec::new(),
            chapter_id,
            story_time: String::new(),
            description: String::new(),
        }
    }
}

// ---- 设定实体 ----

/// 设定实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub id: SettingId,
    pub name: String,
    pub category: String,
    pub rules: Vec<String>,
    pub constraints: Vec<String>,
    pub description: String,
}

impl Setting {
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: SettingId::default(),
            name: name.into(),
            category: category.into(),
            rules: Vec::new(),
            constraints: Vec::new(),
            description: String::new(),
        }
    }
}

// ---- 伏笔实体 ----

/// 伏笔状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForeshadowStatus {
    Planned,
    Planted,
    Progressing,
    Resolved,
    Abandoned,
    Overdue,
}

impl ForeshadowStatus {
    /// 伏笔状态机门控：按铺垫→推进→回收顺序推进，允许回退或废弃
    pub fn can_transition_to(&self, next: &Self) -> bool {
        use ForeshadowStatus::*;
        match (self, next) {
            (a, b) if a == b => true,
            (Planned, Planted) => true,
            (Planned, Progressing) => true,
            (Planned, Abandoned) => true,
            (Planted, Progressing) => true,
            (Planted, Resolved) => true,
            (Planted, Abandoned) => true,
            (Progressing, Resolved) => true,
            (Progressing, Abandoned) => true,
            (Resolved | Abandoned, Planned) => true,
            _ => false,
        }
    }
}

/// 伏笔实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foreshadow {
    pub id: ForeshadowId,
    pub name: String,
    pub description: String,
    pub status: ForeshadowStatus,
    pub planted_chapter: i64,
    pub expected_payoff: Option<i64>,
    pub actual_payoff: Option<i64>,
    pub related_entities: Vec<EntityRef>,
}

impl Foreshadow {
    pub fn new(name: impl Into<String>, planted_chapter: i64) -> Self {
        Self {
            id: ForeshadowId::default(),
            name: name.into(),
            description: String::new(),
            status: ForeshadowStatus::Planned,
            planted_chapter,
            expected_payoff: None,
            actual_payoff: None,
            related_entities: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreshadow_status_follows_payoff_flow() {
        assert!(ForeshadowStatus::Planned.can_transition_to(&ForeshadowStatus::Planted));
        assert!(ForeshadowStatus::Planted.can_transition_to(&ForeshadowStatus::Progressing));
        assert!(ForeshadowStatus::Progressing.can_transition_to(&ForeshadowStatus::Resolved));
        assert!(ForeshadowStatus::Planned.can_transition_to(&ForeshadowStatus::Abandoned));
        assert!(ForeshadowStatus::Resolved.can_transition_to(&ForeshadowStatus::Planned));
        assert!(!ForeshadowStatus::Planned.can_transition_to(&ForeshadowStatus::Resolved));
        assert!(!ForeshadowStatus::Progressing.can_transition_to(&ForeshadowStatus::Planted));
    }
}

// ---- 组织实体（组织档案，P0 新增） ----

/// 组织实体（势力/宗门/家族/帝国/商会等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    /// 势力类型：宗门 / 家族 / 帝国 / 商会 ...
    pub category: String,
    /// 等级结构描述
    pub structure: String,
    /// 组织目标
    pub goals: String,
    /// 组织规则
    pub rules: Vec<String>,
    /// 成员（人物引用）
    pub members: Vec<EntityRef>,
    pub description: String,
}

impl Organization {
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: OrganizationId::default(),
            name: name.into(),
            category: category.into(),
            structure: String::new(),
            goals: String::new(),
            rules: Vec::new(),
            members: Vec::new(),
            description: String::new(),
        }
    }
}

// ---- 统一实体枚举 ----

/// 所有可能的实体类型（用于图谱存储）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entity {
    Character(Character),
    Event(Event),
    Setting(Setting),
    Foreshadow(Foreshadow),
    Organization(Organization),
}

impl Entity {
    /// 获取实体类型
    pub fn entity_type(&self) -> EntityType {
        match self {
            Self::Character(_) => EntityType::Character,
            Self::Event(_) => EntityType::Event,
            Self::Setting(_) => EntityType::Setting,
            Self::Foreshadow(_) => EntityType::Foreshadow,
            Self::Organization(_) => EntityType::Organization,
        }
    }

    /// 获取实体 ID
    pub fn entity_id(&self) -> &str {
        match self {
            Self::Character(c) => c.id.as_str(),
            Self::Event(e) => e.id.as_str(),
            Self::Setting(s) => s.id.as_str(),
            Self::Foreshadow(f) => f.id.as_str(),
            Self::Organization(o) => o.id.as_str(),
        }
    }

    /// 获取实体名称
    pub fn name(&self) -> &str {
        match self {
            Self::Character(c) => &c.name,
            Self::Event(e) => &e.name,
            Self::Setting(s) => &s.name,
            Self::Foreshadow(f) => &f.name,
            Self::Organization(o) => &o.name,
        }
    }
}
