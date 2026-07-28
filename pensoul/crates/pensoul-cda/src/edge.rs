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
}
