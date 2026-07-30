use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};

use super::ConsistencyRule;

/// 角色状态一致性规则
/// 检查角色属性跨章一致性
#[derive(Debug, Default)]
pub struct CharacterStateConsistencyRule;

impl CharacterStateConsistencyRule {
    pub fn new() -> Self {
        Self
    }
}

impl ConsistencyRule for CharacterStateConsistencyRule {
    fn name(&self) -> &str {
        "CharacterStateConsistency"
    }

    fn applies_to(&self, entity_type: &EntityType) -> bool {
        *entity_type == EntityType::Character
    }

    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation> {
        if states.len() < 2 {
            return vec![];
        }

        let mut violations = vec![];
        let mut sorted_states = states.to_vec();
        sorted_states.sort_by_key(|s| s.chapter_id.clone());

        for i in 0..sorted_states.len() {
            for j in (i + 1)..sorted_states.len() {
                let state_a = &sorted_states[i];
                let state_b = &sorted_states[j];

                // 比较 name 字段
                if let (Some(name_a), Some(name_b)) = (
                    state_a.state_data.get("name").and_then(|v| v.as_str()),
                    state_b.state_data.get("name").and_then(|v| v.as_str()),
                ) && name_a != name_b
                {
                    violations.push(
                        ConsistencyViolation::new(
                            state_a.entity_id.clone(),
                            EntityType::Character,
                            state_a.chapter_id.clone(),
                            state_b.chapter_id.clone(),
                            format!("Character name changed from '{}' to '{}'", name_a, name_b),
                            ViolationSeverity::Warning,
                            self.name().to_string(),
                        )
                        .with_suggested_fix("Verify if the name change is intentional".to_string()),
                    );
                }

                // 比较 location 字段
                if let (Some(loc_a), Some(loc_b)) = (
                    state_a.state_data.get("location").and_then(|v| v.as_str()),
                    state_b.state_data.get("location").and_then(|v| v.as_str()),
                ) && loc_a != loc_b
                {
                    // 检查是否有合理的空间转换
                    let has_valid_transition = state_b
                        .state_data
                        .get("transition_reason")
                        .and_then(|v| v.as_str())
                        .is_some();

                    if !has_valid_transition {
                        violations.push(
                                ConsistencyViolation::new(
                                    state_a.entity_id.clone(),
                                    EntityType::Character,
                                    state_a.chapter_id.clone(),
                                    state_b.chapter_id.clone(),
                                    format!("Character location changed from '{}' to '{}' without explanation", loc_a, loc_b),
                                    ViolationSeverity::Info,
                                    self.name().to_string(),
                                )
                                .with_suggested_fix("Add a transition explanation for the location change".to_string()),
                            );
                    }
                }

                // 比较状态版本连续性
                if state_b.version - state_a.version > 1 {
                    violations.push(ConsistencyViolation::new(
                        state_a.entity_id.clone(),
                        EntityType::Character,
                        state_a.chapter_id.clone(),
                        state_b.chapter_id.clone(),
                        format!(
                            "Version gap detected: v{} -> v{}",
                            state_a.version, state_b.version
                        ),
                        ViolationSeverity::Info,
                        self.name().to_string(),
                    ));
                }
            }
        }

        violations
    }
}
