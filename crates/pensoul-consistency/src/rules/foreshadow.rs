use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};
use pensoul_core::id::ChapterId;

use super::ConsistencyRule;

/// 伏笔跟踪规则
/// 检查伏笔跟踪完整性
#[derive(Debug, Default)]
pub struct ForeshadowTrackingRule;

impl ForeshadowTrackingRule {
    pub fn new() -> Self {
        Self
    }
}

impl ConsistencyRule for ForeshadowTrackingRule {
    fn name(&self) -> &str {
        "ForeshadowTracking"
    }

    fn applies_to(&self, entity_type: &EntityType) -> bool {
        *entity_type == EntityType::Foreshadow
    }

    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation> {
        let mut violations = vec![];

        for state in states {
            // 检查伏笔状态
            if let Some(status) = state.state_data.get("status").and_then(|v| v.as_str()) {
                // 检查是否过期
                if let Some(expected_resolve) = state
                    .state_data
                    .get("expected_resolve_chapter")
                    .and_then(|v| v.as_i64())
                    && status == "planted"
                {
                    let expected_ch = ChapterId::new(expected_resolve.to_string());
                    if state.chapter_id > expected_ch {
                        violations.push(
                            ConsistencyViolation::new(
                                state.entity_id.clone(),
                                EntityType::Foreshadow,
                                state.chapter_id.clone(),
                                expected_ch.clone(),
                                format!(
                                    "Foreshadow '{}' is overdue: planted at chapter {}, expected to resolve by chapter {}",
                                    state.state_data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown"),
                                    state.chapter_id,
                                    expected_ch
                                ),
                                ViolationSeverity::Warning,
                                self.name().to_string(),
                            )
                            .with_suggested_fix("Resolve the foreshadow or update its expected resolve chapter".to_string()),
                        );
                    }
                }

                // 检查伏笔是否已解决但没有解决章节
                if status == "resolved"
                    && state
                        .state_data
                        .get("actual_resolve_chapter")
                        .and_then(|v| v.as_i64())
                        .is_none()
                {
                    violations.push(
                        ConsistencyViolation::new(
                            state.entity_id.clone(),
                            EntityType::Foreshadow,
                            state.chapter_id.clone(),
                            state.chapter_id.clone(),
                            format!(
                                "Foreshadow '{}' marked as resolved but no resolve chapter recorded",
                                state.state_data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown")
                            ),
                            ViolationSeverity::Error,
                            self.name().to_string(),
                        )
                        .with_suggested_fix("Record the chapter where the foreshadow was resolved".to_string()),
                    );
                }
            }

            // 检查相关角色
            if let Some(characters) = state
                .state_data
                .get("related_characters")
                .and_then(|v| v.as_array())
                && characters.is_empty()
            {
                violations.push(ConsistencyViolation::new(
                    state.entity_id.clone(),
                    EntityType::Foreshadow,
                    state.chapter_id.clone(),
                    state.chapter_id.clone(),
                    format!(
                        "Foreshadow '{}' has no related characters",
                        state
                            .state_data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    ),
                    ViolationSeverity::Info,
                    self.name().to_string(),
                ));
            }
        }

        violations
    }
}
