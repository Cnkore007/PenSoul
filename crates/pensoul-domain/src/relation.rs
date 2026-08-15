// relation.rs — 关系类型定义
// 实体间关系，支持双向索引

use crate::entity::EntityRef;
use crate::id::*;
use serde::{Deserialize, Serialize};

/// 关系类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    /// 角色-角色（朋友、敌人、恋人等）
    CharacterToCharacter,
    /// 角色-事件（参与、触发等）
    CharacterToEvent,
    /// 事件-事件（因果、时序等）
    EventToEvent,
    /// 设定-角色（约束、影响等）
    SettingToCharacter,
    /// 伏笔-事件（铺垫、回收等）
    ForeshadowToEvent,
    /// 自定义关系
    Custom(String),
}

/// 关系变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationChange {
    pub chapter_id: i64,
    pub old_type: String,
    pub new_type: String,
    pub reason: String,
}

/// 实体间关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub from: EntityRef,
    pub to: EntityRef,
    pub relation_type: RelationType,
    pub strength: f32,
    pub history: Vec<RelationChange>,
}

impl Relation {
    pub fn new(
        from: EntityRef,
        to: EntityRef,
        relation_type: RelationType,
    ) -> Self {
        Self {
            id: RelationId::default(),
            from,
            to,
            relation_type,
            strength: 1.0,
            history: Vec::new(),
        }
    }

    /// 设置关系强度（0.0 ~ 1.0）
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }
}
