/// 增量一致性检查器模块
use std::time::Instant;

use crate::entity_state::{EntityType, EntityState, EntityStateManager};
use crate::report::{ConsistencyReport, ConsistencyViolation};
use crate::rules::{ConsistencyRule, get_all_rules};
use crate::scope::{ConsistencyCheckScope, determine_scope};
use pensoul_core::id::ChapterId;

/// 将 ChapterId 转为 i64，便于做加减运算。非数字 ID 返回 0。
fn chapter_to_i64(ch: &ChapterId) -> i64 {
    ch.as_str().parse::<i64>().unwrap_or(0)
}

/// 将 i64 转回 ChapterId。
fn i64_to_chapter(n: i64) -> ChapterId {
    ChapterId::new(n.to_string())
}

/// 增量一致性检查器
pub struct IncrementalChecker {
    /// 状态管理器
    state_manager: EntityStateManager,
    /// 检查规则
    rules: Vec<Box<dyn ConsistencyRule>>,
}

impl IncrementalChecker {
    /// 创建新的检查器
    pub fn new() -> Self {
        Self {
            state_manager: EntityStateManager::new(),
            rules: get_all_rules(),
        }
    }

    /// 创建带有自定义规则的检查器
    pub fn with_rules(rules: Vec<Box<dyn ConsistencyRule>>) -> Self {
        Self {
            state_manager: EntityStateManager::new(),
            rules,
        }
    }

    /// 注册状态
    pub fn register_state(&mut self, state: EntityState) {
        self.state_manager.register_state(state);
    }

    /// 增量检查特定章节的特定实体类型
    pub fn check_incremental(&self, chapter_id: ChapterId, entity_type: EntityType) -> ConsistencyReport {
        let start = Instant::now();
        let mut report = ConsistencyReport::new();

        // 确定检查范围
        let scope = determine_scope(&entity_type);

        // 获取范围内的状态
        let entity_ids = self.state_manager.get_all_entity_ids_by_type(&entity_type);
        report.total_entities_checked = entity_ids.len();

        for entity_id in &entity_ids {
            let states = self.get_states_for_scope(entity_id, &chapter_id, &scope);

            if states.len() < 2 {
                continue;
            }

            // 对每条规则运行检查
            for rule in &self.rules {
                if rule.applies_to(&entity_type) {
                    let violations = rule.check(&states);
                    for violation in violations {
                        report.add_violation(violation);
                    }
                }
            }
        }

        report.check_duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// 全书检查
    pub fn check_all(&self) -> ConsistencyReport {
        let start = Instant::now();
        let mut report = ConsistencyReport::new();

        // 获取所有实体类型
        let entity_types = vec![
            EntityType::Character,
            EntityType::Setting,
            EntityType::Timeline,
            EntityType::Event,
            EntityType::Plot,
            EntityType::Foreshadow,
        ];

        let mut total_entities = 0;

        for entity_type in &entity_types {
            let entity_ids = self.state_manager.get_all_entity_ids_by_type(entity_type);
            total_entities += entity_ids.len();

            let scope = determine_scope(entity_type);

            for entity_id in &entity_ids {
                let all_states = self.state_manager.get_state(entity_id).cloned().unwrap_or_default();
                if all_states.len() < 2 {
                    continue;
                }

                // 按章节 ID 排序
                let mut sorted = all_states;
                sorted.sort_by_key(|s| s.chapter_id.clone());

                // 根据作用域确定批处理方式
                let batches: Vec<Vec<crate::entity_state::EntityState>> = match &scope {
                    ConsistencyCheckScope::FullBook => {
                        vec![sorted]
                    }
                    ConsistencyCheckScope::ChapterOnly => {
                        // 相邻章节对
                        sorted.windows(2).map(|w| w.to_vec()).collect()
                    }
                    ConsistencyCheckScope::ChapterPlusNeighbors => {
                        // 3 章滑动窗口
                        sorted.windows(3).map(|w| w.to_vec()).collect()
                    }
                };

                for states in batches {
                    if states.len() < 2 {
                        continue;
                    }

                    for rule in &self.rules {
                        if rule.applies_to(entity_type) {
                            let violations = rule.check(&states);
                            for violation in violations {
                                report.add_violation(violation);
                            }
                        }
                    }
                }
            }
        }

        report.total_entities_checked = total_entities;
        report.check_duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// 获取所有违反记录
    pub fn get_violations(&self) -> Vec<ConsistencyViolation> {
        self.check_all().violations
    }

    /// 根据范围获取状态
    fn get_states_for_scope(
        &self,
        entity_id: &str,
        chapter_id: &ChapterId,
        scope: &ConsistencyCheckScope,
    ) -> Vec<crate::entity_state::EntityState> {
        match scope {
            ConsistencyCheckScope::ChapterOnly => {
                // 仅当前章节及之前的状态
                let zero = i64_to_chapter(0);
                self.state_manager
                    .get_states_in_chapter_range(entity_id, &zero, chapter_id)
                    .into_iter()
                    .cloned()
                    .collect()
            }
            ConsistencyCheckScope::ChapterPlusNeighbors => {
                // 当前章节及前后各一章
                let num = chapter_to_i64(chapter_id);
                let start = i64_to_chapter(std::cmp::max(0, num - 1));
                let end = i64_to_chapter(num + 1);
                self.state_manager
                    .get_states_in_chapter_range(entity_id, &start, &end)
                    .into_iter()
                    .cloned()
                    .collect()
            }
            ConsistencyCheckScope::FullBook => {
                // 全书所有状态
                self.state_manager
                    .get_state(entity_id)
                    .cloned()
                    .unwrap_or_default()
            }
        }
    }
}

impl Default for IncrementalChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_state(entity_id: &str, entity_type: EntityType, chapter_id: i64, data: serde_json::Value) -> EntityState {
        EntityState {
            entity_id: entity_id.to_string(),
            entity_type,
            chapter_id: ChapterId::new(chapter_id.to_string()),
            state_data: data,
            version: 1,
        }
    }

    #[test]
    fn test_new_checker() {
        let checker = IncrementalChecker::new();
        assert!(checker.rules.len() == 5);
    }

    #[test]
    fn test_register_and_check_character() {
        let mut checker = IncrementalChecker::new();

        // 注册角色状态
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            1,
            json!({"name": "Alice", "location": "Forest"}),
        ));
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            2,
            json!({"name": "Alice", "location": "Village"}),
        ));

        // 检查第2章
        let report = checker.check_incremental(ChapterId::new("2"), EntityType::Character);

        assert_eq!(report.total_entities_checked, 1);
        // 位置变化应该产生一个 Info 违反
        assert!(report.violations.iter().any(|v| v.severity == crate::report::ViolationSeverity::Info));
    }

    #[test]
    fn test_check_all() {
        let mut checker = IncrementalChecker::new();

        // 注册多种类型
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            1,
            json!({"name": "Alice"}),
        ));
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            2,
            json!({"name": "Bob"}),
        ));

        checker.register_state(make_state(
            "world_1",
            EntityType::Setting,
            1,
            json!({"name": "World A"}),
        ));
        checker.register_state(make_state(
            "world_1",
            EntityType::Setting,
            2,
            json!({"name": "World B"}),
        ));

        let report = checker.check_all();

        assert_eq!(report.total_entities_checked, 2);
        assert!(!report.violations.is_empty());
    }

    #[test]
    fn test_get_violations() {
        let mut checker = IncrementalChecker::new();

        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            1,
            json!({"name": "Alice"}),
        ));
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            2,
            json!({"name": "Bob"}),
        ));

        let violations = checker.get_violations();
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_scope_determination() {
        let mut checker = IncrementalChecker::new();

        // Character 只检查当前章节及之前
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            1,
            json!({"name": "Alice"}),
        ));
        checker.register_state(make_state(
            "char_1",
            EntityType::Character,
            5,
            json!({"name": "Bob"}),
        ));

        // 检查第2章 - 不应该看到第5章的状态
        let report = checker.check_incremental(ChapterId::new("2"), EntityType::Character);
        assert_eq!(report.total_entities_checked, 1);
        // 第5章的状态不在范围内，所以不会有违反
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_with_custom_rules() {
        use crate::rules::CharacterStateConsistencyRule;

        let rules: Vec<Box<dyn ConsistencyRule>> = vec![
            Box::new(CharacterStateConsistencyRule::new()),
        ];

        let checker = IncrementalChecker::with_rules(rules);
        assert!(checker.rules.len() == 1);
    }
}
