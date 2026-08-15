// character.rs — 角色状态一致性规则

use pensoul_domain::constraint::*;
use pensoul_domain::entity::Character;

/// 角色状态一致性规则
pub struct CharacterConsistencyRule;

impl CharacterConsistencyRule {
    pub fn check(characters: &[Character]) -> ConstraintCheckResult {
        let mut violations = Vec::new();

        for character in characters {
            // 状态快照必须按章节递增（时间倒流 = 硬错误）
            let mut last_chapter = i64::MIN;
            for state in &character.states {
                if state.chapter_id < last_chapter {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("character-consistency"),
                        severity: ViolationSeverity::Error,
                        message: format!("角色 {} 的状态快照章节顺序倒流（第 {} 章出现在第 {} 章之后）",
                            character.name, state.chapter_id, last_chapter),
                        entity_id: Some(character.id.to_string()),
                        chapter_range: Some((state.chapter_id, last_chapter)),
                        suggestion: Some("按时间顺序重新排列状态快照".to_string()),
                    });
                }
                last_chapter = state.chapter_id;
            }

            // 同一章节多个状态快照 = 数据冗余警告
            let mut seen = std::collections::HashSet::new();
            for state in &character.states {
                if !seen.insert(state.chapter_id) {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("character-consistency"),
                        severity: ViolationSeverity::Warning,
                        message: format!("角色 {} 在第 {} 章存在多个状态快照", character.name, state.chapter_id),
                        entity_id: Some(character.id.to_string()),
                        chapter_range: Some((state.chapter_id, state.chapter_id)),
                        suggestion: Some("合并同一章节的重复快照".to_string()),
                    });
                }
            }

            // 性格强度必须落在 0~1
            for (trait_name, strength) in &character.properties.personality {
                if !(0.0..=1.0).contains(strength) {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("character-consistency"),
                        severity: ViolationSeverity::Error,
                        message: format!("角色 {} 的性格维度 {} 强度 {} 超出 0~1", character.name, trait_name, strength),
                        entity_id: Some(character.id.to_string()),
                        chapter_range: None,
                        suggestion: Some("将强度归一化到 0~1".to_string()),
                    });
                }
            }

            // 状态转换缺少触发原因 = 追踪断裂警告
            for transition in &character.history {
                if transition.trigger.trim().is_empty() {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("character-consistency"),
                        severity: ViolationSeverity::Warning,
                        message: format!("角色 {} 的状态转换（{} → {}）缺少触发原因", character.name, transition.from, transition.to),
                        entity_id: Some(character.id.to_string()),
                        chapter_range: Some((transition.chapter_id, transition.chapter_id)),
                        suggestion: Some("补充触发原因".to_string()),
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
    use pensoul_domain::entity::{Character, EntityState};

    #[test]
    fn state_chapter_reversal_is_error() {
        let mut character = Character::new("林默");
        character.states.push(EntityState {
            chapter_id: 5,
            story_time: String::new(),
            data: serde_json::json!({}),
        });
        character.states.push(EntityState {
            chapter_id: 3,
            story_time: String::new(),
            data: serde_json::json!({}),
        });

        let result = CharacterConsistencyRule::check(&[character]);
        assert!(!result.passed);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }

    #[test]
    fn personality_strength_out_of_range_is_error() {
        let mut character = Character::new("苏晚");
        character.properties.personality.push(("胆识".to_string(), 1.5));
        let result = CharacterConsistencyRule::check(&[character]);
        assert!(result
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Error));
    }
}
