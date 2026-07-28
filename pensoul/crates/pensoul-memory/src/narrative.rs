use crate::packet::{estimate_tokens, NarrativeDetail};

/// 叙事记忆 — 重要性排序的叙事细节
pub struct NarrativeMemory {
    details: Vec<NarrativeDetail>,
}

impl NarrativeMemory {
    pub fn new() -> Self {
        Self {
            details: Vec::new(),
        }
    }

    /// 添加叙事细节
    pub fn add_detail(&mut self, detail: NarrativeDetail) {
        self.details.push(detail);
    }

    /// 检索叙事细节 — importance > 0.5，按重要性降序排列
    ///
    /// 受 budget (token 数) 控制
    pub fn retrieve(&self, _current_chapter: i64, budget: usize) -> Vec<NarrativeDetail> {
        let mut candidates: Vec<&NarrativeDetail> = self
            .details
            .iter()
            .filter(|d| d.importance > 0.5)
            .collect();

        // 按重要性降序
        candidates.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut result = Vec::new();
        let mut tokens_used = 0usize;

        for detail in candidates {
            let tokens = estimate_tokens(&detail.content);
            if tokens_used + tokens > budget {
                break;
            }
            tokens_used += tokens;
            result.push(detail.clone());
        }

        result
    }

    /// 返回所有细节数量
    pub fn total_details(&self) -> usize {
        self.details.len()
    }
}

impl Default for NarrativeMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::NarrativeCategory;
    use pensoul_core::ChapterId;

    fn make_detail(detail_id: &str, importance: f32, content: &str) -> NarrativeDetail {
        NarrativeDetail {
            detail_id: detail_id.to_string(),
            chapter_id: ChapterId::new("1"),
            category: NarrativeCategory::Habit,
            content: content.to_string(),
            importance,
            last_referenced: None,
        }
    }

    #[test]
    fn test_narrative_importance_filter() {
        let mut mem = NarrativeMemory::new();
        mem.add_detail(make_detail("d1", 0.3, "不重要的细节"));
        mem.add_detail(make_detail("d2", 0.8, "重要的细节"));
        mem.add_detail(make_detail("d3", 0.6, "中等重要"));

        let result = mem.retrieve(1, 10000);
        // 只返回 importance > 0.5 的
        assert_eq!(result.len(), 2);
        // 应按重要性降序
        assert_eq!(result[0].detail_id, "d2");
        assert_eq!(result[1].detail_id, "d3");
    }

    #[test]
    fn test_narrative_budget_limit() {
        let mut mem = NarrativeMemory::new();
        mem.add_detail(make_detail("d1", 0.9, "一段很长的叙事内容用于测试预算限制"));
        mem.add_detail(make_detail("d2", 0.8, "另一段很长的叙事内容用于测试预算限制"));
        mem.add_detail(make_detail("d3", 0.7, "第三段很长的叙事内容用于测试预算限制"));

        let result = mem.retrieve(1, 5);
        // budget 很小，可能只返回 0 或 1 条
        assert!(result.len() <= 1);
    }

    #[test]
    fn test_narrative_empty() {
        let mem = NarrativeMemory::new();
        let result = mem.retrieve(1, 10000);
        assert!(result.is_empty());
        assert_eq!(mem.total_details(), 0);
    }

    #[test]
    fn test_narrative_all_below_threshold() {
        let mut mem = NarrativeMemory::new();
        mem.add_detail(make_detail("d1", 0.1, "低重要性"));
        mem.add_detail(make_detail("d2", 0.5, "边界值（不包含）"));

        let result = mem.retrieve(1, 10000);
        assert!(result.is_empty());
    }
}
