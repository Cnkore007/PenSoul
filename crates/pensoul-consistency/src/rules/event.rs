use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};

use super::ConsistencyRule;

/// 事件连续性规则
/// 检查事件连续性
#[derive(Debug, Default)]
pub struct EventContinuityRule;

impl EventContinuityRule {
    pub fn new() -> Self {
        Self
    }
}

impl ConsistencyRule for EventContinuityRule {
    fn name(&self) -> &str {
        "EventContinuity"
    }

    fn applies_to(&self, entity_type: &EntityType) -> bool {
        *entity_type == EntityType::Event
    }

    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation> {
        let mut violations = vec![];

        // 收集所有事件
        let mut events: Vec<&EntityState> = states.iter().collect();
        events.sort_by_key(|s| s.chapter_id.clone());

        // 检查事件因果关系
        for event in &events {
            // 检查因果来源
            if let Some(causes) = event.state_data.get("caused_by").and_then(|v| v.as_array()) {
                for cause in causes {
                    if let Some(cause_id) = cause.as_str() {
                        // 查找原因事件
                        let cause_event = events.iter().find(|e| {
                            e.state_data
                                .get("event_id")
                                .and_then(|v| v.as_str())
                                .map(|id| id == cause_id)
                                .unwrap_or(false)
                        });

                        if let Some(cause_event) = cause_event {
                            // 检查时间顺序
                            if cause_event.chapter_id > event.chapter_id {
                                violations.push(
                                    ConsistencyViolation::new(
                                        event.entity_id.clone(),
                                        EntityType::Event,
                                        cause_event.chapter_id.clone(),
                                        event.chapter_id.clone(),
                                        format!(
                                            "Event '{}' caused by future event '{}'",
                                            event
                                                .state_data
                                                .get("name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown"),
                                            cause_id
                                        ),
                                        ViolationSeverity::Error,
                                        self.name().to_string(),
                                    )
                                    .with_suggested_fix(
                                        "Events must be caused by earlier events".to_string(),
                                    ),
                                );
                            }
                        }
                    }
                }
            }

            // 检查参与者
            if let Some(participants) = event
                .state_data
                .get("participants")
                .and_then(|v| v.as_array())
                && participants.is_empty()
            {
                violations.push(ConsistencyViolation::new(
                    event.entity_id.clone(),
                    EntityType::Event,
                    event.chapter_id.clone(),
                    event.chapter_id.clone(),
                    format!(
                        "Event '{}' has no participants",
                        event
                            .state_data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    ),
                    ViolationSeverity::Warning,
                    self.name().to_string(),
                ));
            }
        }

        violations
    }
}
