// budget.rs — 预算分配
// 根据编辑模式分配 token 预算

use crate::types::{BudgetAllocation, EditingMode, WritingIntent};

/// 默认总预算
const DEFAULT_TOTAL_BUDGET: usize = 8000;

/// 预算分配器
pub struct BudgetAllocator;

impl BudgetAllocator {
    /// 根据编辑模式分配预算
    pub fn allocate(mode: &EditingMode, intent: &WritingIntent) -> BudgetAllocation {
        let total = DEFAULT_TOTAL_BUDGET;

        let (entity_pct, temporal_pct, emotional_pct) = match mode {
            EditingMode::Drafting => (0.6, 0.25, 0.15),
            EditingMode::Revising => (0.5, 0.3, 0.2),
            EditingMode::Reviewing => (0.4, 0.35, 0.25),
        };

        // 根据识别出的意图微调预算：新增内容更偏实体，审查更偏时间线
        let (entity_pct, temporal_pct): (f32, f32) = match intent {
            WritingIntent::NewContent => (entity_pct + 0.05, temporal_pct - 0.05),
            WritingIntent::ReviewConsistency => (entity_pct - 0.05, temporal_pct + 0.05),
            WritingIntent::ModifyContent => (entity_pct, temporal_pct),
        };

        BudgetAllocation {
            total_tokens: total,
            entity_tokens: (total as f32 * entity_pct.clamp(0.0, 1.0)) as usize,
            temporal_tokens: (total as f32 * temporal_pct) as usize,
            emotional_tokens: (total as f32 * emotional_pct) as usize,
        }
    }
}
