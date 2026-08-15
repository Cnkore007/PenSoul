// types.rs — 记忆检索类型定义

use pensoul_domain::entity::EntityRef;
use serde::{Deserialize, Serialize};

/// 编辑模式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditingMode {
    Drafting,
    Revising,
    Reviewing,
}

/// 创作意图
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WritingIntent {
    NewContent,
    ModifyContent,
    ReviewConsistency,
}

/// 记忆检索上下文
#[derive(Debug, Clone)]
pub struct RetrievalContext {
    pub current_chapter: i64,
    pub cursor_position: Option<usize>,
    pub editing_mode: EditingMode,
    pub involved_entities: Vec<EntityRef>,
    pub intent: WritingIntent,
}

impl RetrievalContext {
    pub fn new(current_chapter: i64) -> Self {
        Self {
            current_chapter,
            cursor_position: None,
            editing_mode: EditingMode::Drafting,
            involved_entities: Vec::new(),
            intent: WritingIntent::NewContent,
        }
    }
}

/// 实体记忆信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMemory {
    pub entity: EntityRef,
    pub relevance_score: f32,
    pub summary: String,
    pub details: String,
}

/// 预算分配
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAllocation {
    pub total_tokens: usize,
    pub entity_tokens: usize,
    pub temporal_tokens: usize,
    pub emotional_tokens: usize,
}

/// 记忆检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPacket {
    pub entities: Vec<EntityMemory>,
    pub temporal_context: String,
    pub emotional_context: String,
    pub total_tokens: usize,
    pub budget_used: BudgetAllocation,
}

impl Default for MemoryPacket {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            temporal_context: String::new(),
            emotional_context: String::new(),
            total_tokens: 0,
            budget_used: BudgetAllocation {
                total_tokens: 0,
                entity_tokens: 0,
                temporal_tokens: 0,
                emotional_tokens: 0,
            },
        }
    }
}
