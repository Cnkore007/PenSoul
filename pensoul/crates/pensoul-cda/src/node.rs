use serde::{Deserialize, Serialize};

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
    pub metadata: std::collections::HashMap<String, String>,
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
            metadata: std::collections::HashMap::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
