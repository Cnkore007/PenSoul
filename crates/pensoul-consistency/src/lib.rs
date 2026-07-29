pub mod checker;
/// PenSoul Consistency crate
/// 增量一致性检查模块
pub mod entity_state;
pub mod report;
pub mod rules;
pub mod scope;

pub use checker::IncrementalChecker;
pub use entity_state::{EntityState, EntityStateManager, EntityType};
pub use report::{ConsistencyReport, ConsistencyViolation, ViolationSeverity};
pub use rules::ConsistencyRule;
pub use scope::{ConsistencyCheckScope, determine_scope};
