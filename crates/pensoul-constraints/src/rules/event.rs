// event.rs — 事件连续性规则

use pensoul_domain::constraint::*;
use pensoul_domain::entity::Event;
use std::collections::HashSet;

/// 事件连续性规则
pub struct EventContinuityRule;

impl EventContinuityRule {
    pub fn check(events: &[Event], known_ids: &HashSet<String>) -> ConstraintCheckResult {
        let mut violations = Vec::new();

        for event in events {
            if event.name.trim().is_empty() {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("event-continuity"),
                    severity: ViolationSeverity::Warning,
                    message: "存在未命名事件".to_string(),
                    entity_id: Some(event.id.to_string()),
                    chapter_range: Some((event.chapter_id, event.chapter_id)),
                    suggestion: Some("为事件命名".to_string()),
                });
            }

            // 参与者/后果引用了不存在的实体 = 悬空引用
            for reference in event.participants.iter().chain(event.consequences.iter()) {
                if !known_ids.contains(&reference.entity_id) {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("event-continuity"),
                        severity: ViolationSeverity::Error,
                        message: format!("事件 {} 引用了不存在的实体 {}", event.name, reference.entity_id),
                        entity_id: Some(event.id.to_string()),
                        chapter_range: Some((event.chapter_id, event.chapter_id)),
                        suggestion: Some("修复引用或先创建被引用的实体".to_string()),
                    });
                }
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
    use pensoul_domain::entity::{Event, EntityRef, EntityType};
    use std::collections::HashSet;

    #[test]
    fn dangling_reference_is_error() {
        let mut event = Event::new("决战", 5);
        event
            .participants
            .push(EntityRef::new(EntityType::Character, "ghost-id"));

        let known_ids: HashSet<String> = HashSet::new();
        let result = EventContinuityRule::check(&[event], &known_ids);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }
}
