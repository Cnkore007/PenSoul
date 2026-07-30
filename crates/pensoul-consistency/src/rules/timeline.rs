use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};
use pensoul_core::id::ChapterId;

use super::ConsistencyRule;

/// 时间线一致性规则
/// 检查时间线一致性
#[derive(Debug, Default)]
pub struct TimelineConsistencyRule;

impl TimelineConsistencyRule {
    pub fn new() -> Self {
        Self
    }
}

impl ConsistencyRule for TimelineConsistencyRule {
    fn name(&self) -> &str {
        "TimelineConsistency"
    }

    fn applies_to(&self, entity_type: &EntityType) -> bool {
        *entity_type == EntityType::Timeline
    }

    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation> {
        if states.is_empty() {
            return vec![];
        }

        let mut violations = vec![];
        let mut sorted_states = states.to_vec();
        sorted_states.sort_by_key(|s| s.chapter_id.clone());

        for i in 0..sorted_states.len() {
            for j in (i + 1)..sorted_states.len() {
                let state_a = &sorted_states[i];
                let state_b = &sorted_states[j];

                // 比较时间标记
                if let (Some(time_a), Some(time_b)) = (
                    state_a
                        .state_data
                        .get("time_marker")
                        .and_then(|v| v.as_str()),
                    state_b
                        .state_data
                        .get("time_marker")
                        .and_then(|v| v.as_str()),
                ) && time_a == time_b
                    && state_a.chapter_id != state_b.chapter_id
                {
                    violations.push(
                        ConsistencyViolation::new(
                            state_a.entity_id.clone(),
                            EntityType::Timeline,
                            state_a.chapter_id.clone(),
                            state_b.chapter_id.clone(),
                            format!("Same time marker '{}' in different chapters", time_a),
                            ViolationSeverity::Warning,
                            self.name().to_string(),
                        )
                        .with_suggested_fix("Ensure time progresses between chapters".to_string()),
                    );
                }

                // 检查时间线因果关系
                if let Some(cause_chapter) = state_a
                    .state_data
                    .get("caused_by_chapter")
                    .and_then(|v| v.as_i64())
                {
                    let cause_ch = ChapterId::new(cause_chapter.to_string());
                    if cause_ch >= state_a.chapter_id {
                        violations.push(
                            ConsistencyViolation::new(
                                state_a.entity_id.clone(),
                                EntityType::Timeline,
                                state_a.chapter_id.clone(),
                                cause_ch,
                                "Timeline event caused by a future chapter".to_string(),
                                ViolationSeverity::Error,
                                self.name().to_string(),
                            )
                            .with_suggested_fix(
                                "Events cannot be caused by future chapters".to_string(),
                            ),
                        );
                    }
                }
            }
        }

        violations
    }
}
