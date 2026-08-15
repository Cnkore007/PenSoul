// stats.rs — 图统计信息

use crate::graph::EntityGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 图统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub entities_by_type: HashMap<String, usize>,
    pub avg_relations_per_entity: f64,
}

impl EntityGraph {
    /// 计算图统计信息
    pub fn stats(&self) -> GraphStats {
        let total_entities = self.entity_count();
        let total_relations = self.relation_count();

        let mut entities_by_type: HashMap<String, usize> = HashMap::new();
        for entity in self.all_entities() {
            let type_name = format!("{:?}", entity.entity_type());
            *entities_by_type.entry(type_name).or_insert(0) += 1;
        }

        let avg_relations = if total_entities > 0 {
            (total_relations as f64) / (total_entities as f64)
        } else {
            0.0
        };

        GraphStats {
            total_entities,
            total_relations,
            entities_by_type,
            avg_relations_per_entity: avg_relations,
        }
    }
}
