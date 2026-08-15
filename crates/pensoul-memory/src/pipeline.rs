// pipeline.rs — 动态记忆检索管线
// 意图识别 → 相关性评分 → 预算分配 → 上下文组装

use crate::assembly::ContextAssembler;
use crate::budget::BudgetAllocator;
use crate::intent::IntentRecognizer;
use crate::scoring::RelevanceScorer;
use crate::types::{EntityMemory, MemoryPacket, RetrievalContext};
use pensoul_domain::entity::{Entity, EntityRef};
use pensoul_graph::EntityGraph;

/// 动态记忆检索管线
pub struct MemoryRetrievalPipeline {
    graph: EntityGraph,
}

impl MemoryRetrievalPipeline {
    pub fn new(graph: EntityGraph) -> Self {
        Self { graph }
    }

    /// 根据当前上下文检索最相关的信息
    pub fn retrieve(&self, context: &RetrievalContext) -> MemoryPacket {
        // 1. 意图识别
        let intent = IntentRecognizer::recognize(context);

        // 2. 分配预算（意图参与分配）
        let budget = BudgetAllocator::allocate(&context.editing_mode, &intent);

        // 3. 对所有实体评分
        let mut scored_entities: Vec<EntityMemory> = self
            .graph
            .all_entities()
            .map(|e| {
                let score = RelevanceScorer::score(e, context);
                EntityMemory {
                    entity: EntityRef::new(e.entity_type(), e.entity_id().to_string())
                        .with_label(e.name().to_string()),
                    summary: summarize(e),
                    details: describe(e),
                    relevance_score: score,
                }
            })
            .filter(|m| m.relevance_score > 0.0)
            .collect();

        // 4. 按相关性排序
        scored_entities.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 5. 按预算截断
        let max_entities = (budget.entity_tokens / 200).max(1); // 假设每个实体约 200 token
        let entities: Vec<EntityMemory> = scored_entities
            .into_iter()
            .take(max_entities)
            .collect();

        let total_tokens = entities
            .iter()
            .map(|e| e.summary.chars().count() + e.details.chars().count() + 50)
            .sum::<usize>();

        // 时间上下文：当前章节 + 最近的 N 个事件
        let recent_events: Vec<String> = self
            .graph
            .all_entities()
            .filter_map(|e| match e {
                Entity::Event(ev) if ev.chapter_id <= context.current_chapter => {
                    Some(format!("第{}章 {}", ev.chapter_id, ev.name))
                }
                _ => None,
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        let mut temporal_context =
            format!("当前章节: {}", context.current_chapter);
        if !recent_events.is_empty() {
            temporal_context.push_str(&format!("\n近期事件: {}", recent_events.join("；")));
        }

        // 情感上下文：当前章节的角色状态快照
        let emotional_bits: Vec<String> = self
            .graph
            .all_entities()
            .filter_map(|e| match e {
                Entity::Character(c) => c
                    .state_at(context.current_chapter)
                    .map(|state| format!("{}: {}", c.name, state.data)),
                _ => None,
            })
            .collect();
        let emotional_context = if emotional_bits.is_empty() {
            String::new()
        } else {
            emotional_bits.join("\n")
        };

        MemoryPacket {
            entities,
            temporal_context,
            emotional_context,
            total_tokens,
            budget_used: budget,
        }
    }

    /// 格式化为 LLM 可读上下文
    pub fn assemble_context(packet: &MemoryPacket) -> String {
        ContextAssembler::assemble(packet)
    }
}

/// 一句话摘要
fn summarize(entity: &Entity) -> String {
    match entity {
        Entity::Character(c) => {
            let age = c
                .properties
                .age
                .map(|a| a.to_string())
                .unwrap_or_else(|| "未知".to_string());
            let occupation = c
                .properties
                .occupation
                .as_deref()
                .unwrap_or("未知职业");
            format!("角色 {}（{}岁，{}）", c.name, age, occupation)
        }
        Entity::Event(e) => format!("事件 {}（第{}章）", e.name, e.chapter_id),
        Entity::Setting(s) => format!("设定 {}（{}）", s.name, s.category),
        Entity::Foreshadow(f) => format!("伏笔 {}（{:?}）", f.name, f.status),
        Entity::Organization(o) => format!("组织 {}（{}）", o.name, o.category),
    }
}

/// 详细描述
fn describe(entity: &Entity) -> String {
    match entity {
        Entity::Character(c) => {
            let mut parts = Vec::new();
            if let Some(wants) = &c.properties.wants {
                parts.push(format!("渴望: {}", wants));
            }
            if let Some(fears) = &c.properties.fears {
                parts.push(format!("恐惧: {}", fears));
            }
            if let Some(secret) = &c.properties.secret {
                parts.push(format!("秘密: {}", secret));
            }
            if let Some(backstory) = &c.properties.backstory {
                parts.push(format!("背景: {}", backstory));
            }
            parts.join("；")
        }
        Entity::Event(e) => {
            let mut parts = Vec::new();
            if !e.participants.is_empty() {
                parts.push(format!(
                    "参与者: {}",
                    e.participants
                        .iter()
                        .map(|r| r.label.as_deref().unwrap_or(&r.entity_id))
                        .collect::<Vec<_>>()
                        .join("、")
                ));
            }
            if !e.description.is_empty() {
                parts.push(e.description.clone());
            }
            parts.join("；")
        }
        Entity::Setting(s) => {
            let mut parts = Vec::new();
            if !s.rules.is_empty() {
                parts.push(format!("规则: {}", s.rules.join("；")));
            }
            if !s.description.is_empty() {
                parts.push(s.description.clone());
            }
            parts.join("；")
        }
        Entity::Foreshadow(f) => {
            let mut parts = vec![format!("埋设于第{}章", f.planted_chapter)];
            if let Some(p) = f.expected_payoff {
                parts.push(format!("计划回收于第{}章", p));
            }
            if let Some(p) = f.actual_payoff {
                parts.push(format!("实际回收于第{}章", p));
            }
            if !f.description.is_empty() {
                parts.push(f.description.clone());
            }
            parts.join("；")
        }
        Entity::Organization(o) => {
            let mut parts = Vec::new();
            if !o.structure.is_empty() {
                parts.push(format!("结构: {}", o.structure));
            }
            if !o.goals.is_empty() {
                parts.push(format!("目标: {}", o.goals));
            }
            if !o.members.is_empty() {
                parts.push(format!(
                    "成员: {}",
                    o.members
                        .iter()
                        .map(|r| r.label.as_deref().unwrap_or(&r.entity_id))
                        .collect::<Vec<_>>()
                        .join("、")
                ));
            }
            if !o.rules.is_empty() {
                parts.push(format!("规则: {}", o.rules.join("；")));
            }
            if !o.description.is_empty() {
                parts.push(o.description.clone());
            }
            parts.join("；")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RetrievalContext;
    use pensoul_domain::entity::{Character, Entity};

    #[test]
    fn retrieve_fills_summary_and_details() {
        let mut graph = EntityGraph::new();
        let mut character = Character::new("林默");
        character.properties.occupation = Some("医师".to_string());
        character.properties.wants = Some("查明真相".to_string());
        graph.add_entity(Entity::Character(character));

        let pipeline = MemoryRetrievalPipeline::new(graph);
        let context = RetrievalContext::new(3);
        let packet = pipeline.retrieve(&context);

        assert!(!packet.entities.is_empty());
        let memory = &packet.entities[0];
        assert!(memory.summary.contains("林默"));
        assert!(memory.summary.contains("医师"));
        assert!(memory.details.contains("查明真相"));
        assert!(packet.temporal_context.contains("当前章节: 3"));
        assert!(packet.total_tokens > 0);
    }

    #[test]
    fn review_intent_allocates_more_temporal_budget() {
        let graph = EntityGraph::new();
        let pipeline = MemoryRetrievalPipeline::new(graph);

        let draft_context = RetrievalContext {
            editing_mode: crate::types::EditingMode::Reviewing,
            ..RetrievalContext::new(1)
        };
        let packet = pipeline.retrieve(&draft_context);
        assert_eq!(
            packet.budget_used.temporal_tokens,
            (8000.0 * 0.40) as usize,
            "Reviewing 模式应分配 40% 时间预算"
        );
    }
}
