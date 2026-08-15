// report.rs — 审计报告

use pensoul_domain::constraint::ConstraintViolation;
use serde::{Deserialize, Serialize};

/// 审计报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub checked_entities: usize,
    pub violations: Vec<ConstraintViolation>,
}

impl AuditReport {
    /// 是否有问题
    pub fn has_issues(&self) -> bool {
        !self.violations.is_empty()
    }

    /// 错误数量
    pub fn error_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Error)
            .count()
    }

    /// 警告数量
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Warning)
            .count()
    }
}
