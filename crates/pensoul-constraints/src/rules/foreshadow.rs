// foreshadow.rs — 伏笔跟踪完整性规则

use pensoul_domain::constraint::*;
use pensoul_domain::entity::{Foreshadow, ForeshadowStatus};
use std::collections::HashSet;

/// 伏笔跟踪完整性规则
pub struct ForeshadowTrackingRule;

impl ForeshadowTrackingRule {
    pub fn check(foreshadows: &[Foreshadow]) -> ConstraintCheckResult {
        let mut violations = Vec::new();
        let mut names = HashSet::new();

        for foreshadow in foreshadows {
            if foreshadow.planted_chapter <= 0 {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                    severity: ViolationSeverity::Error,
                    message: format!("伏笔 {} 的埋设章节必须是有效章节（>= 1）", foreshadow.name),
                    entity_id: Some(foreshadow.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("指定正确的埋设章节".to_string()),
                });
            }

            // 未埋先收：回收章节早于埋设章节
            if let Some(payoff) = foreshadow.expected_payoff {
                if payoff < foreshadow.planted_chapter {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                        severity: ViolationSeverity::Error,
                        message: format!(
                            "伏笔 {} 未埋先收：计划回收于第 {} 章，但第 {} 章才埋下",
                            foreshadow.name, payoff, foreshadow.planted_chapter
                        ),
                        entity_id: Some(foreshadow.id.to_string()),
                        chapter_range: Some((payoff, foreshadow.planted_chapter)),
                        suggestion: Some("调整埋设或回收章节".to_string()),
                    });
                }
            }
            if let Some(payoff) = foreshadow.actual_payoff {
                if payoff < foreshadow.planted_chapter {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                        severity: ViolationSeverity::Error,
                        message: format!(
                            "伏笔 {} 未埋先收：实际回收于第 {} 章，但第 {} 章才埋下",
                            foreshadow.name, payoff, foreshadow.planted_chapter
                        ),
                        entity_id: Some(foreshadow.id.to_string()),
                        chapter_range: Some((payoff, foreshadow.planted_chapter)),
                        suggestion: Some("调整埋设或回收章节".to_string()),
                    });
                }
            }

            // 已回收但未记录回收章节
            if foreshadow.status == ForeshadowStatus::Resolved
                && foreshadow.actual_payoff.is_none()
                && foreshadow.expected_payoff.is_none()
            {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                    severity: ViolationSeverity::Warning,
                    message: format!("伏笔 {} 已标记回收，但未记录回收章节", foreshadow.name),
                    entity_id: Some(foreshadow.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("记录实际回收章节".to_string()),
                });
            }

            if foreshadow.name.trim().is_empty() {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                    severity: ViolationSeverity::Warning,
                    message: "存在未命名伏笔".to_string(),
                    entity_id: Some(foreshadow.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("为伏笔命名".to_string()),
                });
            }

            if !names.insert(foreshadow.name.trim().to_string()) {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                    severity: ViolationSeverity::Warning,
                    message: format!("存在重名伏笔：{}", foreshadow.name),
                    entity_id: Some(foreshadow.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("为伏笔使用不重复的名称".to_string()),
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
    use pensoul_domain::entity::Foreshadow;

    #[test]
    fn payoff_before_planting_is_error() {
        let mut foreshadow = Foreshadow::new("神秘来信", 10);
        foreshadow.expected_payoff = Some(3);
        let result = ForeshadowTrackingRule::check(&[foreshadow]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }

    #[test]
    fn resolved_without_payoff_chapter_is_warning() {
        let mut foreshadow = Foreshadow::new("玉佩", 1);
        foreshadow.status = ForeshadowStatus::Resolved;
        let result = ForeshadowTrackingRule::check(&[foreshadow]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning));
    }
}
