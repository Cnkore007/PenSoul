//! 影响图的类型定义 —— 边、节点、图统计
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 边的语义关系
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeRelation {
    /// A 引用了 B
    References,
    /// A 与 B 矛盾
    Contradicts,
    /// A 依赖 B
    DependsOn,
    /// A 导致 B
    Causes,
    /// A 修改了 B
    Modifies,
}

/// 影响图中的有向边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEdge {
    /// 起始节点 ID
    pub from_id: String,
    /// 目标节点 ID
    pub to_id: String,
    /// 语义关系
    pub relation: EdgeRelation,
    /// 权重 (0.0 ~ 1.0)
    pub weight: f64,
}

impl ImpactEdge {
    /// 创建新边
    pub fn new(
        from_id: impl Into<String>,
        to_id: impl Into<String>,
        relation: EdgeRelation,
        weight: f64,
    ) -> Self {
        Self {
            from_id: from_id.into(),
            to_id: to_id.into(),
            relation,
            weight: weight.clamp(0.0, 1.0),
        }
    }

    /// 创建默认权重 (1.0) 的边
    pub fn with_default_weight(
        from_id: impl Into<String>,
        to_id: impl Into<String>,
        relation: EdgeRelation,
    ) -> Self {
        Self::new(from_id, to_id, relation, 1.0)
    }
}

/// 节点类型——影响图中的叙事元素分类
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Entity,
    Event,
    Setting,
    Foreshadow,
    Relationship,
    Knowledge,
}

/// 影响严重程度——按传播深度分级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ImpactSeverity {
    Direct,
    Indirect,
    Cascading,
}

/// 影响图中的节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    /// 全局唯一标识
    pub id: String,
    /// 叙事元素类型
    pub node_type: NodeType,
    /// 所属章节
    pub chapter_id: u32,
    /// 内容哈希（用于检测内容是否变更）
    pub content_hash: String,
    /// 影响严重程度（传播时由算法填充）
    pub severity: ImpactSeverity,
    /// 扩展元数据
    pub metadata: HashMap<String, String>,
}

impl ImpactNode {
    /// 创建新节点
    pub fn new(
        id: impl Into<String>,
        node_type: NodeType,
        chapter_id: u32,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            node_type,
            chapter_id,
            content_hash: content_hash.into(),
            severity: ImpactSeverity::Direct,
            metadata: HashMap::new(),
        }
    }

    /// 设置严重程度
    pub fn with_severity(mut self, severity: ImpactSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// 插入元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

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
    nodes: &HashMap<String, ImpactNode>,
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

    #[test]
    fn test_edge_creation() {
        let edge = ImpactEdge::new("n1", "n2", EdgeRelation::Causes, 0.8);
        assert_eq!(edge.from_id, "n1");
        assert_eq!(edge.to_id, "n2");
        assert_eq!(edge.relation, EdgeRelation::Causes);
        assert!((edge.weight - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_edge_weight_clamp() {
        let edge = ImpactEdge::new("a", "b", EdgeRelation::References, 2.0);
        assert!((edge.weight - 1.0).abs() < f64::EPSILON);

        let edge = ImpactEdge::new("a", "b", EdgeRelation::References, -0.5);
        assert!((edge.weight - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_weight() {
        let edge = ImpactEdge::with_default_weight("x", "y", EdgeRelation::DependsOn);
        assert!((edge.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_edge_serialization() {
        let edge = ImpactEdge::new("a", "b", EdgeRelation::Contradicts, 0.5);
        let json = serde_json::to_string(&edge).unwrap();
        let back: ImpactEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.relation, EdgeRelation::Contradicts);
    }

    #[test]
    fn test_node_creation() {
        let node = ImpactNode::new("entity-1", NodeType::Entity, 1, "hash_abc");
        assert_eq!(node.id, "entity-1");
        assert_eq!(node.node_type, NodeType::Entity);
        assert_eq!(node.chapter_id, 1);
        assert_eq!(node.content_hash, "hash_abc");
        assert_eq!(node.severity, ImpactSeverity::Direct);
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_node_builder() {
        let node = ImpactNode::new("evt-1", NodeType::Event, 3, "hash_def")
            .with_severity(ImpactSeverity::Indirect)
            .with_metadata("key", "value");
        assert_eq!(node.severity, ImpactSeverity::Indirect);
        assert_eq!(node.metadata.get("key").unwrap(), "value");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ImpactSeverity::Direct < ImpactSeverity::Indirect);
        assert!(ImpactSeverity::Indirect < ImpactSeverity::Cascading);
    }

    #[test]
    fn test_node_type_serialization() {
        let node = ImpactNode::new("n1", NodeType::Foreshadow, 5, "h");
        let json = serde_json::to_string(&node).unwrap();
        let back: ImpactNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_type, NodeType::Foreshadow);
        assert_eq!(back.chapter_id, 5);
    }

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
