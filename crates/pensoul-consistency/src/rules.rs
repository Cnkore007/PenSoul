/// 一致性检查规则模块
use crate::entity_state::{EntityState, EntityType};
use crate::report::{ConsistencyViolation, ViolationSeverity};
use pensoul_core::id::ChapterId;

/// 一致性检查规则 trait
pub trait ConsistencyRule: Send + Sync {
    /// 规则名称
    fn name(&self) -> &str;

    /// 检查状态列表
    fn check(&self, states: &[EntityState]) -> Vec<ConsistencyViolation>;

    /// 是否适用于指定实体类型
    fn applies_to(&self, entity_type: &EntityType) -> bool;
}

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
