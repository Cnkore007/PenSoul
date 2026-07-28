use std::collections::{HashMap, HashSet, VecDeque};

use crate::edge::EdgeRelation;
use crate::graph::AffectedItem;
use crate::node::{ImpactNode, ImpactSeverity};

/// BFS 变更传播算法
///
/// 从 source_chapter 中变更的实体出发，沿反向边向外传播，
/// 收集所有受影响的节点并标记影响等级。
///
/// "反向边"的含义：若图中有 A → B（A 引用/依赖 B），
/// 则当 B 变更时，A 受影响。reverse_edges 存储的是
/// `B → [(A, relation, weight)]` 的映射。
pub fn bfs_find_affected(
    reverse_edges: &HashMap<String, Vec<(String, EdgeRelation, f64)>>,
    nodes: &HashMap<String, ImpactNode>,
    source_chapter: u32,
    changed_entity_ids: &[String],
    max_depth: u32,
) -> Vec<AffectedItem> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut results: Vec<AffectedItem> = Vec::new();

    // BFS 队列：(node_id, current_depth, reason)
    let mut queue: VecDeque<(String, u32, String)> = VecDeque::new();

    // 第一步：将变更实体标记为已访问（不加入结果），并作为 BFS 种子
    for entity_id in changed_entity_ids {
        if nodes.contains_key(entity_id) && visited.insert(entity_id.clone()) {
            queue.push_back((entity_id.clone(), 0, format!("直接变更: {entity_id}")));
        }
    }

    // 第二步：BFS 向外传播
    while let Some((current_id, depth, reason)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        // 获取反向邻居（指向 current_id 的节点，即引用/依赖 current_id 的节点）
        if let Some(edges) = reverse_edges.get(&current_id) {
            for (neighbor_id, relation, weight) in edges {
                if visited.contains(neighbor_id) {
                    continue;
                }

                let neighbor_node = match nodes.get(neighbor_id) {
                    Some(n) => n,
                    None => continue,
                };

                let next_depth = depth + 1;

                // 分级：depth=0 → Direct, depth<=2 → Indirect, depth>2 → Cascading
                // 紧邻章节（|chapter - source| <= 2）标记为 Direct
                let severity =
                    compute_severity(next_depth, source_chapter, neighbor_node.chapter_id);

                let action = suggest_action_for_neighbor(
                    neighbor_id,
                    neighbor_node,
                    &current_id,
                    relation,
                    *weight,
                );

                let item = AffectedItem {
                    node_id: neighbor_id.clone(),
                    chapter_id: neighbor_node.chapter_id,
                    severity,
                    reason: format!(
                        "{reason} → {relation:?} → {neighbor_id} (depth={next_depth})"
                    ),
                    suggested_action: action,
                };

                results.push(item);

                if visited.insert(neighbor_id.clone()) {
                    queue.push_back((
                        neighbor_id.clone(),
                        next_depth,
                        format!("{reason} → {neighbor_id}"),
                    ));
                }
            }
        }
    }

    // 按 chapter_id 然后 severity 排序
    results.sort_by(|a, b| {
        a.chapter_id
            .cmp(&b.chapter_id)
            .then_with(|| a.severity.cmp(&b.severity))
    });

    results
}

/// 计算影响严重程度
///
/// - depth=0 → Direct（初始种子）
/// - depth=1..=2 → Indirect
/// - depth>2 → Cascading
/// - 紧邻章节（|chapter - source| <= 2）且 depth <= 2 → Direct
fn compute_severity(depth: u32, source_chapter: u32, target_chapter: u32) -> ImpactSeverity {
    let chapter_distance = source_chapter.abs_diff(target_chapter);

    // 紧邻章节且深度较浅时，标记为 Direct
    if chapter_distance <= 2 && depth <= 2 {
        return ImpactSeverity::Direct;
    }

    match depth {
        1..=2 => ImpactSeverity::Indirect,
        _ => ImpactSeverity::Cascading,
    }
}

/// 为受影响的邻居节点生成建议操作
fn suggest_action_for_neighbor(
    neighbor_id: &str,
    neighbor_node: &ImpactNode,
    source_id: &str,
    relation: &EdgeRelation,
    weight: f64,
) -> String {
    let node_type = format!("{:?}", neighbor_node.node_type);

    match relation {
        EdgeRelation::References => {
            format!(
                "「{neighbor_id}」({node_type}) 引用了「{source_id}」，建议验证引用是否仍准确 (权重: {weight:.1})"
            )
        }
        EdgeRelation::Contradicts => {
            format!(
                "「{neighbor_id}」({node_type}) 与「{source_id}」存在矛盾关系，建议审查冲突内容 (权重: {weight:.1})"
            )
        }
        EdgeRelation::DependsOn => {
            format!(
                "「{neighbor_id}」({node_type}) 依赖于「{source_id}」，建议检查依赖链完整性 (权重: {weight:.1})"
            )
        }
        EdgeRelation::Causes => {
            format!(
                "「{neighbor_id}」({node_type}) 由「{source_id}」导致，建议评估连锁反应 (权重: {weight:.1})"
            )
        }
        EdgeRelation::Modifies => {
            format!(
                "「{neighbor_id}」({node_type}) 被「{source_id}」修改，建议同步更新 (权重: {weight:.1})"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeType;

    fn make_node(id: &str, node_type: NodeType, chapter: u32) -> ImpactNode {
        ImpactNode::new(id, node_type, chapter, format!("hash_{id}"))
    }

    #[test]
    fn test_empty_graph() {
        let nodes = HashMap::new();
        let reverse_edges = HashMap::new();
        let results = bfs_find_affected(&reverse_edges, &nodes, 1, &["e1".into()], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_direct_change_no_neighbors() {
        // 变更源没有被任何节点引用 → 结果为空（源本身不计入）
        let mut nodes = HashMap::new();
        nodes.insert("e1".into(), make_node("e1", NodeType::Entity, 1));

        let reverse_edges = HashMap::new();
        let results = bfs_find_affected(&reverse_edges, &nodes, 1, &["e1".into()], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_direct_change_one_hop() {
        // e1 → e2（e1 引用 e2）
        // 变更 e2 → e1 受影响
        let mut nodes = HashMap::new();
        nodes.insert("e1".into(), make_node("e1", NodeType::Entity, 1));
        nodes.insert("e2".into(), make_node("e2", NodeType::Entity, 2));

        let mut reverse_edges: HashMap<String, Vec<(String, EdgeRelation, f64)>> = HashMap::new();
        reverse_edges
            .entry("e2".into())
            .or_default()
            .push(("e1".into(), EdgeRelation::References, 1.0));

        // 变更 e2
        let results = bfs_find_affected(&reverse_edges, &nodes, 2, &["e2".into()], 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "e1");
        assert_eq!(results[0].chapter_id, 1);
        assert!(!results[0].suggested_action.is_empty());
    }

    #[test]
    fn test_bfs_depth_limit() {
        // 链: e1 → e2 → e3 → e4
        // reverse_edges: e2→[(e1)], e3→[(e2)], e4→[(e3)]
        // 变更 e4, max_depth=2
        // BFS: e4(d0) → e3(d1) → e2(d2) → e1 would be d3, stopped
        let mut nodes = HashMap::new();
        nodes.insert("e1".into(), make_node("e1", NodeType::Entity, 1));
        nodes.insert("e2".into(), make_node("e2", NodeType::Entity, 2));
        nodes.insert("e3".into(), make_node("e3", NodeType::Entity, 3));
        nodes.insert("e4".into(), make_node("e4", NodeType::Entity, 4));

        let mut reverse_edges: HashMap<String, Vec<(String, EdgeRelation, f64)>> = HashMap::new();
        reverse_edges
            .entry("e2".into())
            .or_default()
            .push(("e1".into(), EdgeRelation::References, 1.0));
        reverse_edges
            .entry("e3".into())
            .or_default()
            .push(("e2".into(), EdgeRelation::References, 1.0));
        reverse_edges
            .entry("e4".into())
            .or_default()
            .push(("e3".into(), EdgeRelation::References, 1.0));

        // max_depth=2: 应找到 e3(depth=1), e2(depth=2)，但不包含 e1(depth=3)
        let results = bfs_find_affected(&reverse_edges, &nodes, 4, &["e4".into()], 2);
        let ids: Vec<&str> = results.iter().map(|r| r.node_id.as_str()).collect();
        assert!(ids.contains(&"e3"));
        assert!(ids.contains(&"e2"));
        assert!(!ids.contains(&"e1"));
    }

    #[test]
    fn test_cycle_detection() {
        // e1 → e2 → e3 → e1 (cycle)
        let mut nodes = HashMap::new();
        nodes.insert("e1".into(), make_node("e1", NodeType::Entity, 1));
        nodes.insert("e2".into(), make_node("e2", NodeType::Entity, 2));
        nodes.insert("e3".into(), make_node("e3", NodeType::Entity, 3));

        let mut reverse_edges: HashMap<String, Vec<(String, EdgeRelation, f64)>> = HashMap::new();
        // e1 → e2
        reverse_edges
            .entry("e2".into())
            .or_default()
            .push(("e1".into(), EdgeRelation::Causes, 1.0));
        // e2 → e3
        reverse_edges
            .entry("e3".into())
            .or_default()
            .push(("e2".into(), EdgeRelation::Causes, 1.0));
        // e3 → e1 (cycle)
        reverse_edges
            .entry("e1".into())
            .or_default()
            .push(("e3".into(), EdgeRelation::Causes, 1.0));

        // 变更 e1, 足够深度
        let results = bfs_find_affected(&reverse_edges, &nodes, 1, &["e1".into()], 10);
        // visited 防止无限循环，应该只找到 e3 和 e2（共2个，不含源 e1）
        let ids: Vec<&str> = results.iter().map(|r| r.node_id.as_str()).collect();
        assert!(ids.contains(&"e3"));
        assert!(ids.contains(&"e2"));
        assert!(!ids.contains(&"e1"));
    }

    #[test]
    fn test_severity_nearby_chapter() {
        // source_chapter=5, target_chapter=6 (distance=1), depth=2 → Direct (邻近章节)
        let severity = compute_severity(2, 5, 6);
        assert_eq!(severity, ImpactSeverity::Direct);

        // source_chapter=5, target_chapter=10 (distance=5), depth=2 → Indirect
        let severity = compute_severity(2, 5, 10);
        assert_eq!(severity, ImpactSeverity::Indirect);
    }

    #[test]
    fn test_severity_ordering() {
        // depth=1, same chapter → Direct (邻近)
        assert_eq!(compute_severity(1, 1, 1), ImpactSeverity::Direct);
        // depth=1, far chapter → Indirect
        assert_eq!(compute_severity(1, 1, 5), ImpactSeverity::Indirect);
        // depth=3, far chapter → Cascading
        assert_eq!(compute_severity(3, 1, 10), ImpactSeverity::Cascading);
    }

    #[test]
    fn test_sorted_output() {
        // e1(ch=3) → e2(ch=1) → e3(ch=5)
        // reverse: e2→[(e1)], e3→[(e2)]
        // 变更 e3: 影响 e2(ch=1), e1(ch=3)
        // 结果应按 chapter_id 排序: e2(ch=1) 先于 e1(ch=3)
        let mut nodes = HashMap::new();
        nodes.insert("e1".into(), make_node("e1", NodeType::Entity, 3));
        nodes.insert("e2".into(), make_node("e2", NodeType::Entity, 1));
        nodes.insert("e3".into(), make_node("e3", NodeType::Entity, 5));

        let mut reverse_edges: HashMap<String, Vec<(String, EdgeRelation, f64)>> = HashMap::new();
        reverse_edges
            .entry("e2".into())
            .or_default()
            .push(("e1".into(), EdgeRelation::References, 1.0));
        reverse_edges
            .entry("e3".into())
            .or_default()
            .push(("e2".into(), EdgeRelation::References, 1.0));

        let results = bfs_find_affected(&reverse_edges, &nodes, 5, &["e3".into()], 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].node_id, "e2");
        assert_eq!(results[0].chapter_id, 1);
        assert_eq!(results[1].node_id, "e1");
        assert_eq!(results[1].chapter_id, 3);
    }
}
