// setting.rs — 设定规则一致性规则

use pensoul_domain::constraint::*;
use pensoul_domain::entity::Setting;
use std::collections::HashSet;

/// 设定规则一致性规则
pub struct SettingConsistencyRule;

impl SettingConsistencyRule {
    pub fn check(settings: &[Setting]) -> ConstraintCheckResult {
        let mut violations = Vec::new();
        let mut names = HashSet::new();

        for setting in settings {
            if setting.name.trim().is_empty() {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("setting-rule"),
                    severity: ViolationSeverity::Warning,
                    message: "存在未命名设定".to_string(),
                    entity_id: Some(setting.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("为设定命名".to_string()),
                });
            }
            if setting.category.trim().is_empty() {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("setting-rule"),
                    severity: ViolationSeverity::Warning,
                    message: format!("设定 {} 缺少类别", setting.name),
                    entity_id: Some(setting.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("补充设定类别".to_string()),
                });
            }
            if !names.insert(setting.name.trim().to_string()) {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("setting-rule"),
                    severity: ViolationSeverity::Error,
                    message: format!("存在重名设定：{}", setting.name),
                    entity_id: Some(setting.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("设定名称必须唯一".to_string()),
                });
            }
        }

        if violations.is_empty() {
            ConstraintCheckResult::pass()
        } else {
            ConstraintCheckResult::fail(violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_domain::entity::Setting;

    #[test]
    fn duplicate_setting_name_is_error() {
        let a = Setting::new("灵力体系", "力量");
        let b = Setting::new("灵力体系", "力量");
        let result = SettingConsistencyRule::check(&[a, b]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }
}
