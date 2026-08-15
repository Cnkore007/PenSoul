// evaluator.rs — 约束评估器
// 聚合全部内置规则，对图谱中的实体执行真实检查

use pensoul_domain::constraint::*;
use pensoul_domain::entity::{Entity, ForeshadowStatus};
use pensoul_graph::EntityGraph;
use std::collections::HashSet;

use crate::rules::{
    CharacterConsistencyRule, EventContinuityRule, ForeshadowTrackingRule,
    SettingConsistencyRule, TimelineConsistencyRule,
};

/// 约束评估器
pub struct ConstraintEvaluator;

impl ConstraintEvaluator {
    /// 对图谱执行全部内置规则，返回合并的违规列表
    pub fn evaluate_all(graph: &EntityGraph) -> Vec<ConstraintViolation> {
        let mut characters = Vec::new();
        let mut events = Vec::new();
        let mut settings = Vec::new();
        let mut foreshadows = Vec::new();
        let mut known_ids = HashSet::new();

        for entity in graph.all_entities() {
            known_ids.insert(entity.entity_id().to_string());
            match entity {
                Entity::Character(c) => characters.push(c.clone()),
                Entity::Event(e) => events.push(e.clone()),
                Entity::Setting(s) => settings.push(s.clone()),
                Entity::Foreshadow(f) => foreshadows.push(f.clone()),
                // 组织档案暂不参与内置五规则（角色/时间线/设定/伏笔/事件），留待组织一致性规则
                Entity::Organization(_) => {}
            }
        }

        let mut violations = Vec::new();
        violations.extend(CharacterConsistencyRule::check(&characters).violations);
        violations.extend(TimelineConsistencyRule::check(&events).violations);
        violations.extend(SettingConsistencyRule::check(&settings).violations);
        violations.extend(ForeshadowTrackingRule::check(&foreshadows).violations);
        violations.extend(EventContinuityRule::check(&events, &known_ids).violations);
        violations
    }

    /// 对单个实体执行其类型对应的规则（用于修改后验证）
    pub fn evaluate_entity(entity: &Entity) -> ConstraintCheckResult {
        let mut characters = Vec::new();
        let mut events = Vec::new();
        let mut settings = Vec::new();
        let mut foreshadows = Vec::new();
        let mut known_ids = HashSet::new();

        match entity {
            Entity::Character(c) => characters.push(c.clone()),
            Entity::Event(e) => events.push(e.clone()),
            Entity::Setting(s) => settings.push(s.clone()),
            Entity::Foreshadow(f) => foreshadows.push(f.clone()),
            Entity::Organization(_) => {}
        }
        known_ids.insert(entity.entity_id().to_string());

        let mut violations = Vec::new();
        violations.extend(CharacterConsistencyRule::check(&characters).violations);
        violations.extend(TimelineConsistencyRule::check(&events).violations);
        violations.extend(SettingConsistencyRule::check(&settings).violations);
        violations.extend(ForeshadowTrackingRule::check(&foreshadows).violations);
        violations.extend(EventContinuityRule::check(&events, &known_ids).violations);

        if violations.is_empty() {
            ConstraintCheckResult::pass()
        } else {
            ConstraintCheckResult::fail(violations)
        }
    }

    /// 解析状态字符串，用于修改前的状态机门控
    pub fn parse_foreshadow_status(input: &str) -> Option<ForeshadowStatus> {
        match input {
            "Planned" => Some(ForeshadowStatus::Planned),
            "Planted" => Some(ForeshadowStatus::Planted),
            "Progressing" => Some(ForeshadowStatus::Progressing),
            "Resolved" => Some(ForeshadowStatus::Resolved),
            "Abandoned" => Some(ForeshadowStatus::Abandoned),
            "Overdue" => Some(ForeshadowStatus::Overdue),
            _ => None,
        }
    }
}
