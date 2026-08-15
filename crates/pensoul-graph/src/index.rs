// index.rs — 双向索引查询
// 正向索引（角色→事件）和反向索引（事件→角色）

use crate::graph::EntityGraph;
use pensoul_domain::entity::EntityType;
use pensoul_domain::relation::RelationType;

impl EntityGraph {
    /// 获取与指定实体直接相连的所有实体 ID（双向）
    pub fn connected_entities(&self, entity_id: &str) -> Vec<String> {
        let mut connected = Vec::new();

        // 正向：从该实体出发
        for rel in self.relations_from(entity_id) {
            connected.push(rel.to.entity_id.clone());
        }

        // 反向：指向该实体
        for rel in self.relations_to(entity_id) {
            connected.push(rel.from.entity_id.clone());
        }

        connected.sort();
        connected.dedup();
        connected
    }

    /// 获取指定实体的邻居（按关系类型过滤）
    pub fn neighbors_by_relation(
        &self,
        entity_id: &str,
        relation_type: &RelationType,
    ) -> Vec<String> {
        let mut neighbors = Vec::new();

        for rel in self.relations_from(entity_id) {
            if &rel.relation_type == relation_type {
                neighbors.push(rel.to.entity_id.clone());
            }
        }

        for rel in self.relations_to(entity_id) {
            if &rel.relation_type == relation_type {
                neighbors.push(rel.from.entity_id.clone());
            }
        }

        neighbors.sort();
        neighbors.dedup();
        neighbors
    }

    /// 获取指定实体的指定类型邻居
    pub fn neighbors_by_entity_type(
        &self,
        entity_id: &str,
        target_type: EntityType,
    ) -> Vec<String> {
        self.connected_entities(entity_id)
            .into_iter()
            .filter(|id| {
                self.get_entity(id)
                    .map(|e| e.entity_type() == target_type)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// 判断两个实体是否直接相连
    pub fn are_connected(&self, entity_a: &str, entity_b: &str) -> bool {
        self.relations_from(entity_a)
            .iter()
            .any(|r| r.to.entity_id == entity_b)
            || self.relations_to(entity_a)
                .iter()
                .any(|r| r.from.entity_id == entity_b)
    }
}
