// graph.rs — 实体图谱核心结构
// 双向索引、实体存储、基础增删改查

use pensoul_domain::entity::{Entity, EntityType};
use pensoul_domain::relation::Relation;
use std::collections::HashMap;

/// 实体图谱
#[derive(Clone)]
pub struct EntityGraph {
    /// 实体存储：entity_id -> Entity
    entities: HashMap<String, Entity>,
    /// 正向索引：entity_id -> 从该实体出发的关系 ID 列表
    from_index: HashMap<String, Vec<String>>,
    /// 反向索引：entity_id -> 指向该实体的关系 ID 列表
    to_index: HashMap<String, Vec<String>>,
    /// 关系存储：relation_id -> Relation
    relations: HashMap<String, Relation>,
}

impl Default for EntityGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            from_index: HashMap::new(),
            to_index: HashMap::new(),
            relations: HashMap::new(),
        }
    }

    // ---- 实体管理 ----

    /// 添加实体
    pub fn add_entity(&mut self, entity: Entity) -> bool {
        let id = entity.entity_id().to_string();
        self.entities.insert(id, entity).is_none()
    }

    /// 移除实体（同时移除相关关系）
    pub fn remove_entity(&mut self, entity_id: &str) -> Option<Entity> {
        // 移除相关关系
        let from_rels: Vec<String> = self
            .from_index
            .remove(entity_id)
            .unwrap_or_default();
        let to_rels: Vec<String> = self
            .to_index
            .remove(entity_id)
            .unwrap_or_default();

        for rel_id in from_rels.iter().chain(to_rels.iter()) {
            self.relations.remove(rel_id);
        }

        self.entities.remove(entity_id)
    }

    /// 更新实体数据
    pub fn update_entity(
        &mut self,
        entity_id: &str,
        updater: impl FnOnce(&mut Entity),
    ) -> bool {
        if let Some(entity) = self.entities.get_mut(entity_id) {
            updater(entity);
            true
        } else {
            false
        }
    }

    /// 获取实体
    pub fn get_entity(&self, entity_id: &str) -> Option<&Entity> {
        self.entities.get(entity_id)
    }

    /// 获取实体（可变引用）
    pub fn get_entity_mut(&mut self, entity_id: &str) -> Option<&mut Entity> {
        self.entities.get_mut(entity_id)
    }

    /// 获取所有实体
    pub fn all_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    /// 按类型筛选实体
    pub fn entities_by_type(&self, entity_type: EntityType) -> Vec<&Entity> {
        self.entities
            .values()
            .filter(|e| e.entity_type() == entity_type)
            .collect()
    }

    /// 实体数量
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // ---- 关系管理 ----

    /// 添加关系
    pub fn add_relation(&mut self, relation: Relation) -> bool {
        let id = relation.id.as_str().to_string();
        let from_id = relation.from.entity_id.clone();
        let to_id = relation.to.entity_id.clone();

        self.from_index
            .entry(from_id)
            .or_default()
            .push(id.clone());
        self.to_index
            .entry(to_id)
            .or_default()
            .push(id.clone());

        self.relations.insert(id, relation).is_none()
    }

    /// 移除关系
    pub fn remove_relation(&mut self, relation_id: &str) -> Option<Relation> {
        let relation = self.relations.remove(relation_id)?;
        if let Some(v) = self.from_index.get_mut(&relation.from.entity_id) {
            v.retain(|id| id != relation_id);
        }
        if let Some(v) = self.to_index.get_mut(&relation.to.entity_id) {
            v.retain(|id| id != relation_id);
        }
        Some(relation)
    }

    /// 获取从某实体出发的关系
    pub fn relations_from(&self, entity_id: &str) -> Vec<&Relation> {
        self.from_index
            .get(entity_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.relations.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取指向某实体的关系
    pub fn relations_to(&self, entity_id: &str) -> Vec<&Relation> {
        self.to_index
            .get(entity_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.relations.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取与某实体相关的所有关系（双向）
    pub fn all_relations(&self, entity_id: &str) -> Vec<&Relation> {
        let mut from = self.relations_from(entity_id);
        let mut to = self.relations_to(entity_id);
        from.append(&mut to);
        from
    }

    /// 关系数量
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}
