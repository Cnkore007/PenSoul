/// PenSoul Consistency crate
/// 增量一致性检查模块
pub mod entity_state;
pub mod scope;
pub mod report;
pub mod rules;
pub mod checker;

pub use entity_state::{EntityType, EntityState, EntityStateManager};
pub use scope::{ConsistencyCheckScope, determine_scope};
pub use report::{ConsistencyViolation, ConsistencyReport, ViolationSeverity};
pub use rules::ConsistencyRule;
pub use checker::IncrementalChecker;
