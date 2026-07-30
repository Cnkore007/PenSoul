use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};

use super::ConsistencyRule;

/// 世界观设定一致性规则
/// 检查世界观设定跨章一致性
#[derive(Debug, Default)]
pub struct SettingConsistencyRule;

impl SettingConsistencyRule {
    pub fn new() -> Self {
        Self
    }
}

impl ConsistencyRule for SettingConsistencyRule {
    fn name(&self) -> &str {
        "SettingConsistency"
    }

    fn applies_to(&self, entity_type: &EntityType) -> bool {
        *entity_type == EntityType::Setting
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

                // 比较设定名称
                if let (Some(name_a), Some(name_b)) = (
                    state_a.state_data.get("name").and_then(|v| v.as_str()),
                    state_b.state_data.get("name").and_then(|v| v.as_str()),
                ) && name_a != name_b
                {
                    violations.push(
                        ConsistencyViolation::new(
                            state_a.entity_id.clone(),
                            EntityType::Setting,
                            state_a.chapter_id.clone(),
                            state_b.chapter_id.clone(),
                            format!("Setting name changed from '{}' to '{}'", name_a, name_b),
                            ViolationSeverity::Error,
                            self.name().to_string(),
                        )
                        .with_suggested_fix(
                            "Setting names should be consistent throughout the book".to_string(),
                        ),
                    );
                }

                // 比较描述
                if let (Some(desc_a), Some(desc_b)) = (
                    state_a
                        .state_data
                        .get("description")
                        .and_then(|v| v.as_str()),
                    state_b
                        .state_data
                        .get("description")
                        .and_then(|v| v.as_str()),
                ) && desc_a != desc_b
                {
                    violations.push(
                        ConsistencyViolation::new(
                            state_a.entity_id.clone(),
                            EntityType::Setting,
                            state_a.chapter_id.clone(),
                            state_b.chapter_id.clone(),
                            "Setting description differs between chapters".to_string(),
                            ViolationSeverity::Warning,
                            self.name().to_string(),
                        )
                        .with_suggested_fix(
                            "Ensure setting descriptions are consistent".to_string(),
                        ),
                    );
                }

                // 检查规则约束
                if let (Some(rules_a), Some(rules_b)) = (
                    state_a.state_data.get("rules").and_then(|v| v.as_array()),
                    state_b.state_data.get("rules").and_then(|v| v.as_array()),
                ) {
                    let mut missing_rules = vec![];
                    for rule in rules_a {
                        if !rules_b.contains(rule)
                            && let Some(rule_name) = rule.as_str()
                        {
                            missing_rules.push(rule_name.to_string());
                        }
                    }
                    if !missing_rules.is_empty() {
                        violations.push(
                            ConsistencyViolation::new(
                                state_a.entity_id.clone(),
                                EntityType::Setting,
                                state_a.chapter_id.clone(),
                                state_b.chapter_id.clone(),
                                format!(
                                    "Setting rules missing in later chapter: {:?}",
                                    missing_rules
                                ),
                                ViolationSeverity::Error,
                                self.name().to_string(),
                            )
                            .with_suggested_fix(
                                "Ensure all setting rules are applied consistently".to_string(),
                            ),
                        );
                    }
                }
            }
        }

        violations
    }
}
