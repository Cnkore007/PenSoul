// pensoul-constraints: 约束边界引擎
// 硬/软约束、弹性区域、修改前检查、修改后验证、定期审计

pub mod engine;
pub mod evaluator;
pub mod rules;
pub mod report;

pub use engine::ConstraintEngine;
pub use report::AuditReport;
