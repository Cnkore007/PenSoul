// constraint.rs — 约束类型定义
// 硬约束、软约束、弹性区域

use crate::id::*;
use serde::{Deserialize, Serialize};

/// 约束类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// 硬约束：必须满足（实体一致性、时间顺序、设定规则）
    Hard,
    /// 软约束：建议满足（风格一致性、节奏韵律、伏笔平衡）
    Soft,
    /// 弹性区域：允许违反（创作偏离、风格实验、情节转折）
    Flexibility,
}

/// 约束作用域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintScope {
    Global,
    Chapter(i64),
    Volume(String),
    Entity(String),
}

/// 约束定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: RuleId,
    pub name: String,
    pub kind: ConstraintKind,
    pub description: String,
    pub priority: u32,
    pub scope: ConstraintScope,
}

impl Constraint {
    pub fn new(
        name: impl Into<String>,
        kind: ConstraintKind,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: RuleId::default(),
            name: name.into(),
            kind,
            description: description.into(),
            priority: 50,
            scope: ConstraintScope::Global,
        }
    }

    /// 使用稳定的规则 ID（便于审计报告追溯）
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = RuleId::new(id);
        self
    }

    pub fn with_scope(mut self, scope: ConstraintScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// 违规严重度
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Error,
    Warning,
    Info,
}

/// 约束违规
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    pub constraint_id: RuleId,
    pub severity: ViolationSeverity,
    pub message: String,
    pub entity_id: Option<String>,
    pub chapter_range: Option<(i64, i64)>,
    pub suggestion: Option<String>,
}

/// 约束检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCheckResult {
    pub passed: bool,
    pub violations: Vec<ConstraintViolation>,
}

impl ConstraintCheckResult {
    pub fn pass() -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
        }
    }

    pub fn fail(violations: Vec<ConstraintViolation>) -> Self {
        Self {
            passed: false,
            violations,
        }
    }

    /// 是否有错误级别的违规（应阻止修改）
    pub fn has_errors(&self) -> bool {
        self.violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error)
    }
}
