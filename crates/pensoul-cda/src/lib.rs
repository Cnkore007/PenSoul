//! pensoul-cda — 一致性驱动架构（Consistency-Driven Architecture）
//!
//! 基于影响图的变更传播分析引擎。当小说内容发生修改时，
//! 通过 BFS 遍历影响图，自动识别受影响的叙事元素并生成联动建议。
pub mod graph;
pub mod propagation;
pub mod types;

// 重新导出所有公有类型
pub use graph::{AffectedItem, ImpactGraph};
pub use propagation::bfs_find_affected;
pub use types::{EdgeRelation, ImpactEdge, ImpactNode, ImpactSeverity, NodeType, Stats, compute_stats};
