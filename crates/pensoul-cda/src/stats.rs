use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 图统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// 节点总数
    pub total_nodes: usize,
    /// 边总数
    pub total_edges: usize,
    /// 按节点类型分组的计数
    pub nodes_by_type: HashMap<String, usize>,
    /// 出现的章节数
    pub chapters: Vec<u32>,
    /// 平均每节点的边数
    pub avg_edges_per_node: f64,
}

/// 从节点和边数据计算统计信息
pub fn compute_stats(
    nodes: &HashMap<String, crate::node::ImpactNode>,
    edges: &[(String, String)],
) -> Stats {
    let total_nodes = nodes.len();
    let total_edges = edges.len();

    // 按类型分组
    let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
    for node in nodes.values() {
        let key = format!("{:?}", node.node_type);
        *nodes_by_type.entry(key).or_insert(0) += 1;
    }

    // 收集章节数
    let mut chapters: Vec<u32> = nodes.values().map(|n| n.chapter_id).collect();
    chapters.sort();
    chapters.dedup();

    // 平均每节点的边数
    let avg_edges_per_node = if total_nodes > 0 {
        (total_edges as f64) / (total_nodes as f64)
    } else {
        0.0
    };

    Stats {
        total_nodes,
        total_edges,
        nodes_by_type,
        chapters,
        avg_edges_per_node,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{ImpactNode, NodeType};

    #[test]
    fn test_empty_stats() {
        let nodes = HashMap::new();
        let edges: Vec<(String, String)> = vec![];
        let stats = compute_stats(&nodes, &edges);
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
        assert!(stats.nodes_by_type.is_empty());
        assert!(stats.chapters.is_empty());
        assert!((stats.avg_edges_per_node - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_populated_stats() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "e1".into(),
            ImpactNode::new("e1", NodeType::Entity, 1, "h1"),
        );
        nodes.insert(
            "e2".into(),
            ImpactNode::new("e2", NodeType::Entity, 2, "h2"),
        );
        nodes.insert("e3".into(), ImpactNode::new("e3", NodeType::Event, 1, "h3"));

        let edges = vec![
            ("e1".to_string(), "e2".to_string()),
            ("e1".to_string(), "e3".to_string()),
        ];

        let stats = compute_stats(&nodes, &edges);
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.total_edges, 2);
        assert_eq!(*stats.nodes_by_type.get("Entity").unwrap(), 2);
        assert_eq!(*stats.nodes_by_type.get("Event").unwrap(), 1);
        assert_eq!(stats.chapters, vec![1, 2]);
        assert!((stats.avg_edges_per_node - (2.0 / 3.0)).abs() < f64::EPSILON);
    }
}
