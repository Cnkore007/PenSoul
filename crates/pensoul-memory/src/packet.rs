use pensoul_core::ChapterId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 编辑模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditingMode {
    Drafting,
    Revising,
    Reviewing,
}

/// Token 预算分配比例
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BudgetRatio {
    pub hot: f32,
    pub warm: f32,
    pub cold: f32,
    pub narrative: f32,
}

/// 根据编辑模式返回预算分配比例
pub fn get_budget_ratio(mode: EditingMode) -> BudgetRatio {
    match mode {
        EditingMode::Drafting => BudgetRatio {
            hot: 0.50,
            warm: 0.25,
            cold: 0.20,
            narrative: 0.05,
        },
        EditingMode::Revising => BudgetRatio {
            hot: 0.60,
            warm: 0.20,
            cold: 0.15,
            narrative: 0.05,
        },
        EditingMode::Reviewing => BudgetRatio {
            hot: 0.30,
            warm: 0.20,
            cold: 0.40,
            narrative: 0.10,
        },
    }
}

/// 叙事类别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeCategory {
    Habit,
    Promise,
    Prop,
    Sensory,
    Subplot,
}

/// 章节摘要 — 温记忆和冷记忆共用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub chapter_id: ChapterId,
    pub title: String,
    pub summary: String,
    pub key_events: Vec<String>,
    pub character_states: HashMap<String, String>,
    pub word_count: u32,
    pub consistency_score: f32,
    pub semantic_embedding: Option<Vec<f32>>,
}

/// 叙事细节 — 用于叙事记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeDetail {
    pub detail_id: String,
    pub chapter_id: ChapterId,
    pub category: NarrativeCategory,
    pub content: String,
    pub importance: f32,
    pub last_referenced: Option<ChapterId>,
}

/// 温记忆数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WarmMemoryData {
    pub volume_summary: String,
    pub active_foreshadows: Vec<String>,
    pub character_states: Option<String>,
}

/// 记忆包 — 组装好的最终产物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPacket {
    pub hot: Vec<String>,
    pub warm: WarmMemoryData,
    pub cold: Vec<String>,
    pub narrative: Vec<NarrativeDetail>,
    pub total_tokens: usize,
    pub budget_used: BudgetRatio,
}

/// 估算文本的 token 数（原型用 len/2）
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 2
}

/// 估算字符串列表的总 token 数
pub fn estimate_tokens_batch(texts: &[String]) -> usize {
    texts.iter().map(|t| estimate_tokens(t)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_ratios_drafting() {
        let r = get_budget_ratio(EditingMode::Drafting);
        assert!((r.hot - 0.50).abs() < f32::EPSILON);
        assert!((r.warm - 0.25).abs() < f32::EPSILON);
        assert!((r.cold - 0.20).abs() < f32::EPSILON);
        assert!((r.narrative - 0.05).abs() < f32::EPSILON);
        // 比例之和应为 1.0
        let sum = r.hot + r.warm + r.cold + r.narrative;
        assert!((sum - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_budget_ratios_revising() {
        let r = get_budget_ratio(EditingMode::Revising);
        assert!((r.hot - 0.60).abs() < f32::EPSILON);
        assert!((r.warm - 0.20).abs() < f32::EPSILON);
        assert!((r.cold - 0.15).abs() < f32::EPSILON);
        assert!((r.narrative - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn test_budget_ratios_reviewing() {
        let r = get_budget_ratio(EditingMode::Reviewing);
        assert!((r.hot - 0.30).abs() < f32::EPSILON);
        assert!((r.warm - 0.20).abs() < f32::EPSILON);
        assert!((r.cold - 0.40).abs() < f32::EPSILON);
        assert!((r.narrative - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn test_estimate_tokens() {
        // "你好世界" = 4 chars / 2 = 2 tokens
        assert_eq!(estimate_tokens("你好世界"), 2);
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("ab"), 1);
    }
}
