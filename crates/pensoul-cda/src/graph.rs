use std::collections::HashMap;

/// 反向邻接表：(目标节点ID, 关系类型, 权重)
type ReverseAdjacency = HashMap<String, Vec<(String, crate::types::EdgeRelation, f64)>>;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

use crate::types::ImpactEdge;
use crate::types::ImpactNode;
use crate::propagation::bfs_find_affected;
use crate::types::{Stats, compute_stats};

/// 受影响项——传播算法的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedItem {
    /// 受影响节点 ID
    pub node_id: String,
    /// 受影响节点所在章节
    pub chapter_id: u32,
    /// 影响严重程度
    pub severity: crate::types::ImpactSeverity,
    /// 影响原因描述
    pub reason: String,
    /// 建议操作
    pub suggested_action: String,
}

/// 一致性驱动架构——影响图
pub struct ImpactGraph {
    /// petgraph 有向图
    graph: DiGraph<ImpactNode, ImpactEdge>,
    /// ID → NodeIndex 映射
    index_map: HashMap<String, NodeIndex>,
}

impl ImpactGraph {
    /// 创建空的影响图
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index_map: HashMap::new(),
        }
    }

    /// 添加节点，返回是否新增（false 表示已存在）
    pub fn add_node(&mut self, node: ImpactNode) -> bool {
        if self.index_map.contains_key(&node.id) {
            return false;
        }
        let idx = self.graph.add_node(node);
        // SAFETY: borrow was released before insert
        let id = self.graph[idx].id.clone();
        self.index_map.insert(id, idx);
        true
    }

    /// 添加边——节点不存在时返回错误
    pub fn add_edge(
        &mut self,
        edge: ImpactEdge,
    ) -> std::result::Result<(), pensoul_core::PensoulError> {
        let from_idx = self.index_map.get(&edge.from_id).ok_or_else(|| {
            pensoul_core::PensoulError::Internal(format!("源节点不存在: {}", edge.from_id))
        })?;
        let to_idx = self.index_map.get(&edge.to_id).ok_or_else(|| {
            pensoul_core::PensoulError::Internal(format!("目标节点不存在: {}", edge.to_id))
        })?;

        self.graph.add_edge(*from_idx, *to_idx, edge);
        Ok(())
    }

    /// 查找受影响项
    ///
    /// 从 source_chapter 中变更的实体出发，沿反向边传播，收集受影响节点。
    pub fn find_affected(
        &self,
        source_chapter: u32,
        changed_entity_ids: &[String],
        max_depth: u32,
    ) -> Vec<AffectedItem> {
        let (reverse_edges, node_map) = self.build_adjacency();
        bfs_find_affected(
            &reverse_edges,
            &node_map,
            source_chapter,
            changed_entity_ids,
            max_depth,
        )
    }

    /// 计算图统计信息
    pub fn stats(&self) -> Stats {
        let node_map: HashMap<String, ImpactNode> = self
            .graph
            .node_indices()
            .map(|idx| {
                let node = &self.graph[idx];
                (node.id.clone(), node.clone())
            })
            .collect();

        let edges: Vec<(String, String)> = self
            .graph
            .edge_indices()
            .map(|eidx| {
                let (from, to) = self.graph.edge_endpoints(eidx).unwrap();
                (self.graph[from].id.clone(), self.graph[to].id.clone())
            })
            .collect();

        compute_stats(&node_map, &edges)
    }

    /// 获取图中的节点数
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 获取图中的边数
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// 检查节点是否存在
    pub fn has_node(&self, id: &str) -> bool {
        self.index_map.contains_key(id)
    }

    /// 获取节点引用
    pub fn get_node(&self, id: &str) -> Option<&ImpactNode> {
        self.index_map.get(id).map(|&idx| &self.graph[idx])
    }

    /// 构建反向邻接表和节点映射，供 BFS 使用
    fn build_adjacency(&self) -> (ReverseAdjacency, HashMap<String, ImpactNode>) {
        let mut reverse_edges: HashMap<String, Vec<(String, crate::types::EdgeRelation, f64)>> =
            HashMap::new();
        let mut node_map: HashMap<String, ImpactNode> = HashMap::new();

        // 填充节点
        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            node_map.insert(node.id.clone(), node.clone());
        }

        // 填充反向边：对于 A → B 的边，在 B 的邻接列表里记录 A
        for eidx in self.graph.edge_indices() {
            let edge_data = &self.graph[eidx];
            let (from_idx, to_idx) = self.graph.edge_endpoints(eidx).unwrap();
            let from_id = self.graph[from_idx].id.clone();
            let to_id = self.graph[to_idx].id.clone();

            // 反向：B 的反向列表里包含 A
            reverse_edges.entry(to_id).or_default().push((
                from_id,
                edge_data.relation.clone(),
                edge_data.weight,
            ));
        }

        (reverse_edges, node_map)
    }
}

impl Default for ImpactGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EdgeRelation;
    use crate::types::{ImpactSeverity, NodeType};

    fn make_node(id: &str, chapter: u32) -> ImpactNode {
        ImpactNode::new(id, NodeType::Entity, chapter, format!("hash_{id}"))
    }

    #[test]
    fn test_add_node() {
        let mut graph = ImpactGraph::new();
        assert!(graph.add_node(make_node("n1", 1)));
        assert!(!graph.add_node(make_node("n1", 1))); // duplicate
        assert_eq!(graph.node_count(), 1);
        assert!(graph.has_node("n1"));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ImpactGraph::new();
        graph.add_node(make_node("n1", 1));
        graph.add_node(make_node("n2", 2));

        let edge = ImpactEdge::new("n1", "n2", EdgeRelation::References, 0.9);
        assert!(graph.add_edge(edge).is_ok());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_missing_node() {
        let mut graph = ImpactGraph::new();
        graph.add_node(make_node("n1", 1));

        let edge = ImpactEdge::new("n1", "missing", EdgeRelation::References, 0.9);
        let result = graph.add_edge(edge);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_affected_basic() {
        let mut graph = ImpactGraph::new();
        // chapter 1: entity e1 → chapter 2: entity e2
        // The edge e1 → e2 means e1 references e2.
        // If e2 changes, then e1 (which references e2) is affected.
        graph.add_node(make_node("e1", 1));
        graph.add_node(make_node("e2", 2));
        graph
            .add_edge(ImpactEdge::new("e1", "e2", EdgeRelation::References, 1.0))
            .unwrap();

        // e2 changes in chapter 2
        let affected = graph.find_affected(2, &["e2".into()], 5);
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].node_id, "e1");
        assert_eq!(affected[0].chapter_id, 1);
    }

    #[test]
    fn test_find_affected_multi_hop() {
        let mut graph = ImpactGraph::new();
        // e1 → e2 → e3 → e4
        // e4 changes → e3 affected → e2 affected → e1 affected
        graph.add_node(make_node("e1", 1));
        graph.add_node(make_node("e2", 2));
        graph.add_node(make_node("e3", 3));
        graph.add_node(make_node("e4", 4));
        graph
            .add_edge(ImpactEdge::new("e1", "e2", EdgeRelation::References, 1.0))
            .unwrap();
        graph
            .add_edge(ImpactEdge::new("e2", "e3", EdgeRelation::References, 1.0))
            .unwrap();
        graph
            .add_edge(ImpactEdge::new("e3", "e4", EdgeRelation::References, 1.0))
            .unwrap();

        let affected = graph.find_affected(4, &["e4".into()], 5);
        let ids: Vec<&str> = affected.iter().map(|a| a.node_id.as_str()).collect();
        assert!(ids.contains(&"e3"));
        assert!(ids.contains(&"e2"));
        assert!(ids.contains(&"e1"));
    }

    #[test]
    fn test_find_affected_depth_limit() {
        let mut graph = ImpactGraph::new();
        graph.add_node(make_node("e1", 1));
        graph.add_node(make_node("e2", 2));
        graph.add_node(make_node("e3", 3));
        graph.add_node(make_node("e4", 4));
        graph
            .add_edge(ImpactEdge::new("e1", "e2", EdgeRelation::References, 1.0))
            .unwrap();
        graph
            .add_edge(ImpactEdge::new("e2", "e3", EdgeRelation::References, 1.0))
            .unwrap();
        graph
            .add_edge(ImpactEdge::new("e3", "e4", EdgeRelation::References, 1.0))
            .unwrap();

        // e4 changes, max_depth=1: only e3 (depth=1) should be found
        let affected = graph.find_affected(4, &["e4".into()], 1);
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].node_id, "e3");
    }

    #[test]
    fn test_severity_levels() {
        let mut graph = ImpactGraph::new();
        // All in chapter 5 so chapter distance is 0 for all
        for i in 1..=6 {
            graph.add_node(ImpactNode::new(
                format!("e{i}"),
                NodeType::Entity,
                5,
                format!("h{i}"),
            ));
        }
        // Chain: e1 → e2 → e3 → e4 → e5 → e6
        for i in 1..=5 {
            graph
                .add_edge(ImpactEdge::new(
                    format!("e{i}"),
                    format!("e{}", i + 1),
                    EdgeRelation::References,
                    1.0,
                ))
                .unwrap();
        }

        // e6 changes → e5(d=1), e4(d=2), e3(d=3), e2(d=4), e1(d=5)
        // chapter_distance=0 for all, so:
        //   d=1, d=2: Direct (邻近章节 + depth<=2)
        //   d=3, d=4, d=5: Cascading (depth>2)
        let affected = graph.find_affected(5, &["e6".into()], 10);
        assert_eq!(affected.len(), 5);
        for item in &affected {
            match item.node_id.as_str() {
                "e5" => assert_eq!(item.severity, ImpactSeverity::Direct),
                "e4" => assert_eq!(item.severity, ImpactSeverity::Direct),
                "e3" => assert_eq!(item.severity, ImpactSeverity::Cascading),
                "e2" => assert_eq!(item.severity, ImpactSeverity::Cascading),
                "e1" => assert_eq!(item.severity, ImpactSeverity::Cascading),
                other => panic!("unexpected node: {other}"),
            }
        }
    }

    #[test]
    fn test_suggested_action_not_empty() {
        let mut graph = ImpactGraph::new();
        graph.add_node(make_node("e1", 1));
        graph.add_node(make_node("e2", 2));
        graph
            .add_edge(ImpactEdge::new("e1", "e2", EdgeRelation::Causes, 0.8))
            .unwrap();

        let affected = graph.find_affected(2, &["e2".into()], 5);
        for item in &affected {
            assert!(!item.suggested_action.is_empty());
        }
    }

    #[test]
    fn test_stats() {
        let mut graph = ImpactGraph::new();
        graph.add_node(ImpactNode::new("e1", NodeType::Entity, 1, "h1"));
        graph.add_node(ImpactNode::new("e2", NodeType::Event, 2, "h2"));
        graph
            .add_edge(ImpactEdge::new("e1", "e2", EdgeRelation::References, 1.0))
            .unwrap();

        let stats = graph.stats();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_edges, 1);
        assert_eq!(stats.chapters, vec![1, 2]);
    }

    #[test]
    fn test_get_node() {
        let mut graph = ImpactGraph::new();
        graph.add_node(make_node("n1", 1));
        assert!(graph.get_node("n1").is_some());
        assert!(graph.get_node("missing").is_none());
    }
}
