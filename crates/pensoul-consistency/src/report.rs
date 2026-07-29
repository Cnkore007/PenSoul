/// 一致性检查报告模块
use crate::entity_state::EntityType;
use pensoul_core::id::ChapterId;

/// 违反严重度
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViolationSeverity {
    /// 错误
    Error,
    /// 警告
    Warning,
    /// 信息
    Info,
}

impl std::fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationSeverity::Error => write!(f, "Error"),
            ViolationSeverity::Warning => write!(f, "Warning"),
            ViolationSeverity::Info => write!(f, "Info"),
        }
    }
}

/// 一致性违反记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsistencyViolation {
    /// 违反 ID
    pub violation_id: String,
    /// 实体 ID
    pub entity_id: String,
    /// 实体类型
    pub entity_type: EntityType,
    /// 章节 A
    pub chapter_a: ChapterId,
    /// 章节 B
    pub chapter_b: ChapterId,
    /// 描述
    pub description: String,
    /// 严重度
    pub severity: ViolationSeverity,
    /// 规则名称
    pub rule_name: String,
    /// 建议修复
    pub suggested_fix: Option<String>,
}

impl ConsistencyViolation {
    /// 创建新的违反记录
    pub fn new(
        entity_id: String,
        entity_type: EntityType,
        chapter_a: ChapterId,
        chapter_b: ChapterId,
        description: String,
        severity: ViolationSeverity,
        rule_name: String,
    ) -> Self {
        let violation_id = format!("{}_{}_{}_{}", entity_id, chapter_a, chapter_b, rule_name);
        Self {
            violation_id,
            entity_id,
            entity_type,
            chapter_a,
            chapter_b,
            description,
            severity,
            rule_name,
            suggested_fix: None,
        }
    }

    /// 设置建议修复
    pub fn with_suggested_fix(mut self, fix: String) -> Self {
        self.suggested_fix = Some(fix);
        self
    }
}

/// 一致性检查报告
#[derive(Debug, Clone, Default)]
pub struct ConsistencyReport {
    /// 违反列表
    pub violations: Vec<ConsistencyViolation>,
    /// 检查的实体总数
    pub total_entities_checked: usize,
    /// 违反总数
    pub total_violations: usize,
    /// 检查耗时（毫秒）
    pub check_duration_ms: u64,
}

impl ConsistencyReport {
    /// 创建新的报告
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加违反记录
    pub fn add_violation(&mut self, violation: ConsistencyViolation) {
        self.violations.push(violation);
        self.total_violations = self.violations.len();
    }

    /// 获取指定严重度的违反列表
    pub fn get_violations_by_severity(
        &self,
        severity: &ViolationSeverity,
    ) -> Vec<&ConsistencyViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == *severity)
            .collect()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .count()
    }

    /// 获取警告数量
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .count()
    }

    /// 获取信息数量
    pub fn info_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Info)
            .count()
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// 是否有警告
    pub fn has_warnings(&self) -> bool {
        self.warning_count() > 0
    }
}

impl std::fmt::Display for ConsistencyReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Consistency Report ===")?;
        writeln!(f, "Entities checked: {}", self.total_entities_checked)?;
        writeln!(f, "Violations found: {}", self.total_violations)?;
        writeln!(f, "  Errors: {}", self.error_count())?;
        writeln!(f, "  Warnings: {}", self.warning_count())?;
        writeln!(f, "  Info: {}", self.info_count())?;
        writeln!(f, "Duration: {}ms", self.check_duration_ms)?;
        if !self.violations.is_empty() {
            writeln!(f, "\n--- Violations ---")?;
            for v in &self.violations {
                writeln!(
                    f,
                    "[{}] {} ({}, ch{}-ch{}): {}",
                    v.severity, v.rule_name, v.entity_id, v.chapter_a, v.chapter_b, v.description
                )?;
                if let Some(fix) = &v.suggested_fix {
                    writeln!(f, "  Suggested fix: {}", fix)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_violation(severity: ViolationSeverity, rule_name: &str) -> ConsistencyViolation {
        ConsistencyViolation::new(
            "entity_1".to_string(),
            EntityType::Character,
            ChapterId::new("1"),
            ChapterId::new("2"),
            "Test violation".to_string(),
            severity,
            rule_name.to_string(),
        )
    }

    #[test]
    fn test_violation_creation() {
        let v = make_violation(ViolationSeverity::Error, "TestRule");
        assert_eq!(v.entity_id, "entity_1");
        assert_eq!(v.severity, ViolationSeverity::Error);
        assert!(v.violation_id.contains("entity_1"));
    }

    #[test]
    fn test_violation_with_suggested_fix() {
        let v = make_violation(ViolationSeverity::Error, "TestRule")
            .with_suggested_fix("Fix this".to_string());
        assert_eq!(v.suggested_fix, Some("Fix this".to_string()));
    }

    #[test]
    fn test_report_counts() {
        let mut report = ConsistencyReport::new();
        report.add_violation(make_violation(ViolationSeverity::Error, "Rule1"));
        report.add_violation(make_violation(ViolationSeverity::Error, "Rule2"));
        report.add_violation(make_violation(ViolationSeverity::Warning, "Rule3"));

        assert_eq!(report.total_violations, 3);
        assert_eq!(report.error_count(), 2);
        assert_eq!(report.warning_count(), 1);
        assert!(report.has_errors());
        assert!(report.has_warnings());
    }

    #[test]
    fn test_report_get_by_severity() {
        let mut report = ConsistencyReport::new();
        report.add_violation(make_violation(ViolationSeverity::Error, "Rule1"));
        report.add_violation(make_violation(ViolationSeverity::Warning, "Rule2"));

        let errors = report.get_violations_by_severity(&ViolationSeverity::Error);
        assert_eq!(errors.len(), 1);
    }
}
