// engine.rs — 约束边界引擎核心
// 管理约束定义、修改前检查、修改后验证、定期审计

use pensoul_domain::constraint::*;
use pensoul_domain::entity::{Entity, EntityRef};
use pensoul_graph::EntityGraph;
use crate::evaluator::ConstraintEvaluator;

/// 弹性区域
#[derive(Debug, Clone)]
pub struct FlexibilityZone {
    pub name: String,
    pub description: String,
    pub entity_ids: Vec<String>,
}

/// 约束边界引擎
pub struct ConstraintEngine {
    constraints: Vec<Constraint>,
    flexibility_zones: Vec<FlexibilityZone>,
    graph: EntityGraph,
}

impl ConstraintEngine {
    pub fn new(graph: EntityGraph) -> Self {
        let mut engine = Self {
            constraints: Vec::new(),
            flexibility_zones: Vec::new(),
            graph,
        };
        engine.register_builtin_rules();
        engine
    }

    /// 注册内置约束规则
    fn register_builtin_rules(&mut self) {
        // 硬约束
        self.add_constraint(
            Constraint::new("角色状态一致性", ConstraintKind::Hard, "角色属性跨章不能矛盾")
                .with_id("character-consistency")
                .with_priority(100),
        );
        self.add_constraint(
            Constraint::new("时间线顺序", ConstraintKind::Hard, "时间顺序不能倒流")
                .with_id("timeline-order")
                .with_priority(100),
        );
        self.add_constraint(
            Constraint::new("设定规则", ConstraintKind::Hard, "世界观设定不能自相矛盾")
                .with_id("setting-rule")
                .with_priority(90),
        );
        self.add_constraint(
            Constraint::new("事件连续性", ConstraintKind::Hard, "事件参与者与后果引用必须存在")
                .with_id("event-continuity")
                .with_priority(90),
        );
        self.add_constraint(
            Constraint::new("伏笔跟踪", ConstraintKind::Hard, "伏笔不能未埋先收")
                .with_id("foreshadow-tracking")
                .with_priority(90),
        );

        // 软约束
        self.add_constraint(
            Constraint::new("风格一致性", ConstraintKind::Soft, "写作风格应保持一致")
                .with_id("style-consistency")
                .with_priority(50),
        );
        self.add_constraint(
            Constraint::new("伏笔平衡", ConstraintKind::Soft, "伏笔应有始有终")
                .with_id("foreshadow-balance")
                .with_priority(40),
        );
    }

    // ---- 约束管理 ----

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn remove_constraint(&mut self, id: &pensoul_domain::id::RuleId) {
        self.constraints.retain(|c| &c.id != id);
    }

    pub fn list_constraints(&self, kind: Option<ConstraintKind>) -> Vec<&Constraint> {
        match kind {
            Some(k) => self.constraints.iter().filter(|c| c.kind == k).collect(),
            None => self.constraints.iter().collect(),
        }
    }

    // ---- 修改前检查 ----

    pub fn pre_edit_check(
        &self,
        entity_id: &EntityRef,
        proposed_change: &serde_json::Value,
    ) -> ConstraintCheckResult {
        let mut violations = Vec::new();

        // 检查是否在弹性区域内
        if self.in_flexibility_zone(&entity_id.entity_id) {
            return ConstraintCheckResult::pass();
        }

        // 实体必须存在
        let Some(entity) = self.graph.get_entity(&entity_id.entity_id) else {
            return ConstraintCheckResult::fail(vec![ConstraintViolation {
                constraint_id: pensoul_domain::id::RuleId::new("entity-exists"),
                severity: ViolationSeverity::Error,
                message: format!("实体 {} 不存在", entity_id.entity_id),
                entity_id: Some(entity_id.entity_id.clone()),
                chapter_range: None,
                suggestion: Some("请先创建该实体".to_string()),
            }]);
        };

        // 状态机门控：伏笔状态只能按流程转换
        if let Entity::Foreshadow(foreshadow) = entity {
            if let Some(next_status) = proposed_change
                .get("status")
                .and_then(|v| v.as_str())
                .and_then(ConstraintEvaluator::parse_foreshadow_status)
            {
                if !foreshadow.status.can_transition_to(&next_status) {
                    violations.push(ConstraintViolation {
                        constraint_id: pensoul_domain::id::RuleId::new("foreshadow-tracking"),
                        severity: ViolationSeverity::Error,
                        message: format!(
                            "伏笔 {} 的状态不能从 {:?} 直接变为 {:?}",
                            foreshadow.name, foreshadow.status, next_status
                        ),
                        entity_id: Some(entity_id.entity_id.clone()),
                        chapter_range: None,
                        suggestion: Some(
                            "按 Planned → Planted → Progressing → Resolved 顺序推进，或先回退到 Planned"
                                .to_string(),
                        ),
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

    // ---- 修改后验证 ----

    pub fn post_edit_validate(
        &self,
        entity_id: &EntityRef,
    ) -> ConstraintCheckResult {
        // 实体必须存在
        let Some(entity) = self.graph.get_entity(&entity_id.entity_id) else {
            return ConstraintCheckResult::fail(vec![ConstraintViolation {
                constraint_id: pensoul_domain::id::RuleId::default(),
                severity: ViolationSeverity::Error,
                message: format!("实体 {} 不存在", entity_id.entity_id),
                entity_id: Some(entity_id.entity_id.clone()),
                chapter_range: None,
                suggestion: None,
            }]);
        };

        // 对该实体执行真实规则验证
        ConstraintEvaluator::evaluate_entity(entity)
    }

    // ---- 定期审计 ----

    pub fn full_audit(&self) -> crate::report::AuditReport {
        let violations = ConstraintEvaluator::evaluate_all(&self.graph);
        let checked = self.graph.all_entities().count();

        crate::report::AuditReport {
            checked_entities: checked,
            violations,
        }
    }

    // ---- 弹性区域 ----

    pub fn mark_flexibility_zone(&mut self, zone: FlexibilityZone) {
        self.flexibility_zones.push(zone);
    }

    fn in_flexibility_zone(&self, entity_id: &str) -> bool {
        self.flexibility_zones
            .iter()
            .any(|z| z.entity_ids.iter().any(|id| id == entity_id))
    }
}
