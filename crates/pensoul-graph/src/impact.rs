// impact.rs — 影响预测
// 基于 BFS 传播，预测修改某实体可能影响的其他实体

use crate::graph::EntityGraph;
use pensoul_domain::entity::EntityRef;
use serde::{Deserialize, Serialize};

/// 影响严重度
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactSeverity {
    /// 直接影响（1 跳）
    Direct,
    /// 间接影响（2-3 跳）
    Indirect,
    /// 级联影响（4+ 跳）
    Cascading,
}

/// 影响预测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactPrediction {
    pub entity: EntityRef,
    pub severity: ImpactSeverity,
    pub distance: u32,
    pub reason: String,
    pub suggested_action: String,
}

impl EntityGraph {
    /// 预测修改某实体可能影响的其他实体
    /// 使用 BFS 广度优先搜索，从变更实体出发向外传播
    pub fn predict_impact(
        &self,
        changed_entity: &EntityRef,
        max_depth: u32,
    ) -> Vec<ImpactPrediction> {
        let mut predictions = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<(String, u32)> = std::collections::VecDeque::new();

        // 起点入队
        queue.push_back((changed_entity.entity_id.clone(), 0));
        visited.insert(changed_entity.entity_id.clone());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            // 跳过起点本身
            if depth > 0 {
                if let Some(entity) = self.get_entity(&current_id) {
                    let severity = match depth {
                        1 => ImpactSeverity::Direct,
                        2..=3 => ImpactSeverity::Indirect,
                        _ => ImpactSeverity::Cascading,
                    };

                    predictions.push(ImpactPrediction {
                        entity: EntityRef::new(
                            entity.entity_type(),
                            current_id.clone(),
                        )
                        .with_label(entity.name().to_string()),
                        severity,
                        distance: depth,
                        reason: format!(
                            "通过 {} 跳关系连接到变更实体",
                            depth
                        ),
                        suggested_action: "检查一致性".to_string(),
                    });
                }
            }

            // 扩展邻居
            if depth < max_depth {
                for rel in self.all_relations(&current_id) {
                    let neighbor_id = if rel.from.entity_id == current_id {
                        &rel.to.entity_id
                    } else {
                        &rel.from.entity_id
                    };

                    if !visited.contains(neighbor_id) {
                        visited.insert(neighbor_id.to_string());
                        queue.push_back((neighbor_id.clone(), depth + 1));
                    }
                }
            }
        }

        // 按严重度和距离排序
        predictions.sort_by(|a, b| {
            let severity_ord = |s: &ImpactSeverity| match s {
                ImpactSeverity::Direct => 0,
                ImpactSeverity::Indirect => 1,
                ImpactSeverity::Cascading => 2,
            };
            severity_ord(&a.severity)
                .cmp(&severity_ord(&b.severity))
                .then(a.distance.cmp(&b.distance))
        });

        predictions
    }
}
