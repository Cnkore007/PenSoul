// timeline.rs — 时间线一致性规则

use pensoul_domain::constraint::*;
use pensoul_domain::entity::Event;

/// 时间线一致性规则
pub struct TimelineConsistencyRule;

impl TimelineConsistencyRule {
    pub fn check(events: &[Event]) -> ConstraintCheckResult {
        let mut violations = Vec::new();

        for event in events {
            if event.chapter_id <= 0 {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("timeline-order"),
                    severity: ViolationSeverity::Warning,
                    message: format!("事件 {} 未分配到有效章节（章节号必须 >= 1）", event.name),
                    entity_id: Some(event.id.to_string()),
                    chapter_range: None,
                    suggestion: Some("为事件指定正确的章节号".to_string()),
                });
            }

            // 非空 story_time 应可解析为时间；解析不了则无法校验（显式提示，不静默通过）
            if !event.story_time.trim().is_empty() && parse_story_time(&event.story_time).is_none() {
                violations.push(ConstraintViolation {
                    constraint_id: pensoul_domain::id::RuleId::new("timeline-order"),
                    severity: ViolationSeverity::Warning,
                    message: format!("事件 {} 的 story_time 不是可解析的时间格式：{}", event.name, event.story_time),
                    entity_id: Some(event.id.to_string()),
                    chapter_range: Some((event.chapter_id, event.chapter_id)),
                    suggestion: Some("使用 ISO 8601 时间格式（如 2026-08-09T12:00:00+08:00）".to_string()),
                });
            }
        }

        // 同一章节内的事件，故事时间不能倒流
        let mut sorted: Vec<&Event> = events.iter().collect();
        sorted.sort_by_key(|e| e.chapter_id);
        for pair in sorted.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a.chapter_id == b.chapter_id {
                if let (Some(a_time), Some(b_time)) =
                    (parse_story_time(&a.story_time), parse_story_time(&b.story_time))
                {
                    if b_time < a_time {
                        violations.push(ConstraintViolation {
                            constraint_id: pensoul_domain::id::RuleId::new("timeline-order"),
                            severity: ViolationSeverity::Error,
                            message: format!(
                                "第 {} 章内事件顺序倒流：{}（{}）发生在 {}（{}）之后",
                                a.chapter_id, b.name, b.story_time, a.name, a.story_time
                            ),
                            entity_id: Some(b.id.to_string()),
                            chapter_range: Some((a.chapter_id, b.chapter_id)),
                            suggestion: Some("调整事件的故事时间".to_string()),
                        });
                    }
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

/// 解析 story_time（支持 RFC 3339 / ISO 8601）
fn parse_story_time(input: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_domain::entity::Event;

    #[test]
    fn same_chapter_time_reversal_is_error() {
        // 数组顺序代表记录顺序：先 21:00 后 20:00，属于时间倒流
        let mut first = Event::new("开场", 3);
        first.story_time = "2026-08-09T21:00:00+08:00".to_string();
        let mut second = Event::new("结局", 3);
        second.story_time = "2026-08-09T20:00:00+08:00".to_string();

        let result = TimelineConsistencyRule::check(&[first, second]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }

    #[test]
    fn chapter_zero_is_warning_not_silent() {
        let event = Event::new("未分配事件", 0);
        let result = TimelineConsistencyRule::check(&[event]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning));
    }
}
