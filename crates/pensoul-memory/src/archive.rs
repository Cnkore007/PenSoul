use crate::packet::ChapterSummary;

/// 冰记忆 — 归档已完成的章节
pub struct ArchiveMemory {
    /// 已归档的 (chapter_id, summary)
    archived: Vec<(i64, ChapterSummary)>,
}

impl ArchiveMemory {
    pub fn new() -> Self {
        Self {
            archived: Vec::new(),
        }
    }

    /// 归档一个章节
    pub fn archive(&mut self, chapter_id: i64, summary: ChapterSummary) {
        self.archived.push((chapter_id, summary));
    }

    /// 检索归档内容，支持关键词匹配
    pub fn retrieve(&self, query: &str, budget: usize) -> Vec<String> {
        use crate::packet::estimate_tokens;

        let mut result = Vec::new();
        let mut tokens_used = 0usize;

        for (id, summary) in &self.archived {
            // 简单关键词匹配：query 出现在标题、摘要或关键事件中
            let matches = summary.title.contains(query)
                || summary.summary.contains(query)
                || summary.key_events.iter().any(|e| e.contains(query));

            if matches {
                let entry = format!(
                    "[归档] 第{}章「{}」: {}",
                    id, summary.title, summary.summary
                );
                let tokens = estimate_tokens(&entry);
                if tokens_used + tokens > budget {
                    break;
                }
                tokens_used += tokens;
                result.push(entry);
            }
        }

        result
    }

    /// 归档数量
    pub fn len(&self) -> usize {
        self.archived.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.archived.is_empty()
    }
}

impl Default for ArchiveMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::ChapterId;
    use std::collections::HashMap;

    fn make_summary(chapter_id: i64, title: &str, summary: &str) -> ChapterSummary {
        ChapterSummary {
            chapter_id: ChapterId::new(chapter_id.to_string()),
            title: title.to_string(),
            summary: summary.to_string(),
            key_events: vec![],
            character_states: HashMap::new(),
            word_count: 0,
            consistency_score: 1.0,
            semantic_embedding: None,
        }
    }

    #[test]
    fn test_archive_and_retrieve() {
        let mut archive = ArchiveMemory::new();
        archive.archive(1, make_summary(1, "开端", "主角踏上旅程"));
        archive.archive(2, make_summary(2, "战斗", "主角与敌人战斗"));

        let result = archive.retrieve("战斗", 10000);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("第2章"));
        assert!(result[0].contains("[归档]"));
    }

    #[test]
    fn test_archive_retrieve_all() {
        let mut archive = ArchiveMemory::new();
        archive.archive(1, make_summary(1, "A", "summary_a"));
        archive.archive(2, make_summary(2, "B", "summary_b"));

        // 空查询匹配所有
        let result = archive.retrieve("", 10000);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_archive_empty() {
        let archive = ArchiveMemory::new();
        assert!(archive.is_empty());
        let result = archive.retrieve("anything", 10000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_archive_budget_limit() {
        let mut archive = ArchiveMemory::new();
        archive.archive(1, make_summary(1, "长标题测试", "一段很长的摘要内容用于测试预算限制"));
        archive.archive(2, make_summary(2, "另一个长标题", "另一段很长的摘要内容用于测试预算限制"));

        let result = archive.retrieve("", 5);
        // budget 很小，可能只返回 0 或 1 条
        assert!(result.len() <= 1);
    }
}
