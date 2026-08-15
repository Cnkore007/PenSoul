// scoring.rs — 相关性评分
// 根据实体与当前上下文的相关性打分

use crate::types::RetrievalContext;
use pensoul_domain::entity::{Entity, EntityRef};

/// 相关性评分器
pub struct RelevanceScorer;

impl RelevanceScorer {
    /// 计算实体与上下文的相关性分数 (0.0 ~ 1.0)
    pub fn score(entity: &Entity, context: &RetrievalContext) -> f32 {
        let mut score = 0.0f32;

        // 实体是否在涉及列表中
        let entity_ref = EntityRef::new(entity.entity_type(), entity.entity_id().to_string());
        if context.involved_entities.iter().any(|e| e.entity_id == entity_ref.entity_id) {
            score += 0.5;
        }

        // 基于实体类型的基础分数
        match entity {
            Entity::Character(_) => score += 0.2,
            Entity::Event(_) => score += 0.3,
            Entity::Foreshadow(_) => score += 0.1,
            Entity::Setting(_) => score += 0.1,
            Entity::Organization(_) => score += 0.1,
        }

        score.clamp(0.0, 1.0)
    }
}
