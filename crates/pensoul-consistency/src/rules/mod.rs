//! 一致性检查规则模块
//!
//! 每条规则独立成文件（单文件 500 行上限约束）。
mod character;
mod event;
mod foreshadow;
mod setting;
mod timeline;

use crate::entity_state::{EntityState, EntityType};
use crate::report::ConsistencyViolation;

/// 一致性检查规则 trait
pub trait ConsistencyRule: Send + Sync {
    /// 规则名称
    fn name(&self) -> &str;

    /// 检查状态列表
    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation>;

    /// 是否适用于指定实体类型
    fn applies_to(&self, entity_type: &EntityType) -> bool;
}

pub use character::CharacterStateConsistencyRule;
pub use event::EventContinuityRule;
pub use foreshadow::ForeshadowTrackingRule;
pub use setting::SettingConsistencyRule;
pub use timeline::TimelineConsistencyRule;

/// 获取所有预置规则
pub fn get_all_rules() -> Vec<Box<dyn ConsistencyRule>> {
    vec![
        Box::new(CharacterStateConsistencyRule::new()),
        Box::new(SettingConsistencyRule::new()),
        Box::new(ForeshadowTrackingRule::new()),
        Box::new(TimelineConsistencyRule::new()),
        Box::new(EventContinuityRule::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ViolationSeverity;
    use pensoul_core::id::ChapterId;
    use serde_json::json;

    fn make_state(
        entity_id: &str,
        entity_type: EntityType,
        chapter_id: i64,
        data: serde_json::Value,
    ) -> EntityState {
        EntityState {
            entity_id: entity_id.to_string(),
            entity_type,
            chapter_id: ChapterId::new(chapter_id.to_string()),
            state_data: data,
            version: 1,
        }
    }

    #[test]
    fn test_character_state_consistency_rule() {
        let rule = CharacterStateConsistencyRule::new();
        assert_eq!(rule.name(), "CharacterStateConsistency");
        assert!(rule.applies_to(&EntityType::Character));
        assert!(!rule.applies_to(&EntityType::Setting));

        let states = vec![
            make_state("char_1", EntityType::Character, 1, json!({"name": "Alice"})),
            make_state("char_1", EntityType::Character, 2, json!({"name": "Bob"})),
        ];

        let violations = rule.check(&states);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, ViolationSeverity::Warning);
    }

    #[test]
    fn test_setting_consistency_rule() {
        let rule = SettingConsistencyRule::new();
        assert_eq!(rule.name(), "SettingConsistency");
        assert!(rule.applies_to(&EntityType::Setting));

        let states = vec![
            make_state(
                "world_1",
                EntityType::Setting,
                1,
                json!({"name": "Middle Earth", "description": "A fantasy world"}),
            ),
            make_state(
                "world_1",
                EntityType::Setting,
                2,
                json!({"name": "Middle Earth", "description": "A magical world"}),
            ),
        ];

        let violations = rule.check(&states);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, ViolationSeverity::Warning);
    }

    #[test]
    fn test_foreshadow_tracking_rule() {
        let rule = ForeshadowTrackingRule::new();
        assert_eq!(rule.name(), "ForeshadowTracking");
        assert!(rule.applies_to(&EntityType::Foreshadow));

        let states = vec![
            make_state(
                "fs_1",
                EntityType::Foreshadow,
                1,
                json!({
                    "name": "The Prophecy",
                    "status": "planted",
                    "expected_resolve_chapter": 5
                }),
            ),
            make_state(
                "fs_1",
                EntityType::Foreshadow,
                10,
                json!({
                    "name": "The Prophecy",
                    "status": "planted",
                    "expected_resolve_chapter": 5
                }),
            ),
        ];

        let violations = rule.check(&states);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, ViolationSeverity::Warning);
    }

    #[test]
    fn test_timeline_consistency_rule() {
        let rule = TimelineConsistencyRule::new();
        assert_eq!(rule.name(), "TimelineConsistency");
        assert!(rule.applies_to(&EntityType::Timeline));

        let states = vec![
            make_state(
                "tl_1",
                EntityType::Timeline,
                1,
                json!({"time_marker": "Dawn"}),
            ),
            make_state(
                "tl_1",
                EntityType::Timeline,
                2,
                json!({"time_marker": "Dawn"}),
            ),
        ];

        let violations = rule.check(&states);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, ViolationSeverity::Warning);
    }

    #[test]
    fn test_event_continuity_rule() {
        let rule = EventContinuityRule::new();
        assert_eq!(rule.name(), "EventContinuity");
        assert!(rule.applies_to(&EntityType::Event));

        let states = vec![
            make_state(
                "evt_1",
                EntityType::Event,
                10,
                json!({
                    "event_id": "evt_1",
                    "name": "Effect",
                    "caused_by": ["evt_2"]
                }),
            ),
            make_state(
                "evt_2",
                EntityType::Event,
                5,
                json!({
                    "event_id": "evt_2",
                    "name": "Cause"
                }),
            ),
        ];

        let violations = rule.check(&states);
        // evt_1 is at chapter 10, caused by evt_2 at chapter 5 - this is valid
        assert!(violations.is_empty());
    }

    #[test]
    fn test_event_continuity_violation() {
        let rule = EventContinuityRule::new();

        let states = vec![
            make_state(
                "evt_1",
                EntityType::Event,
                5,
                json!({
                    "event_id": "evt_1",
                    "name": "Effect",
                    "caused_by": ["evt_2"]
                }),
            ),
            make_state(
                "evt_2",
                EntityType::Event,
                10,
                json!({
                    "event_id": "evt_2",
                    "name": "Future Cause"
                }),
            ),
        ];

        let violations = rule.check(&states);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity, ViolationSeverity::Error);
    }

    #[test]
    fn test_get_all_rules() {
        let rules = get_all_rules();
        assert_eq!(rules.len(), 5);
    }
}
