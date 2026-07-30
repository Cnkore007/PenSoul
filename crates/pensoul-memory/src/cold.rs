use std::collections::HashMap;

use crate::packet::{ChapterSummary, estimate_tokens};

/// 冷记忆 — 向量检索（原型用简单关键词匹配）
///
/// 存储所有章节摘要，排除热记忆窗口内的章节后提供检索。
/// 窗口大小与 `HotMemory` 保持一致，由 `ColdMemory::new` 注入。
pub struct ColdMemory {
    /// 章节 ID → 章节摘要
    chapters: HashMap<i64, ChapterSummary>,
    /// 排除窗口大小（与热记忆窗口一致）
    window_size: i64,
}

impl ColdMemory {
    /// 创建冷记忆，`window_size` 必须与热记忆窗口一致
    pub fn new(window_size: i64) -> Self {
        Self {
            chapters: HashMap::new(),
            window_size,
        }
    }

    /// 插入章节摘要
    pub fn insert_chapter(&mut self, chapter_id: i64, summary: ChapterSummary) {
        self.chapters.insert(chapter_id, summary);
    }

    /// 检索冷记忆，排除 current_chapter ± window_size 章范围（这些已在热记忆中）
    ///
    /// 按章节 ID 排序，受 budget (token 数) 控制
    pub fn retrieve(&self, current_chapter: i64, budget: usize) -> Vec<String> {
        let window_size = self.window_size;

        let mut candidates: Vec<_> = self
            .chapters
            .iter()
            .filter(|(id, _)| (*id - current_chapter).abs() > window_size)
            .collect();

        // 按章节 ID 排序
        candidates.sort_by_key(|(id, _)| *id);

        let mut result = Vec::new();
        let mut tokens_used = 0usize;

        for (id, summary) in candidates {
            let entry = format!(
                "第{}章「{}」: {} (关键词: {})",
                id,
                summary.title,
                summary.summary,
                summary.key_events.join(", ")
            );

            let tokens = estimate_tokens(&entry);
            if tokens_used + tokens > budget {
                break;
            }

            tokens_used += tokens;
            result.push(entry);
        }

        result
    }

    /// 返回章节数
    pub fn chapter_count(&self) -> usize {
        self.chapters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::ChapterId;

    fn make_summary(
        chapter_id: i64,
        title: &str,
        summary: &str,
        events: Vec<String>,
    ) -> ChapterSummary {
        ChapterSummary {
            chapter_id: ChapterId::new(chapter_id.to_string()),
            title: title.to_string(),
            summary: summary.to_string(),
            key_events: events,
            character_states: HashMap::new(),
            word_count: 0,
            consistency_score: 1.0,
            semantic_embedding: None,
        }
    }

    #[test]
    fn test_cold_memory_excludes_window() {
        let mut cold = ColdMemory::new(2);
        for i in 1..=10 {
            cold.insert_chapter(
                i,
                make_summary(i, &format!("第{}章", i), &format!("摘要{}", i), vec![]),
            );
        }

        // 当前第5章，±2 范围 = 3,4,5,6,7 不应出现
        let result = cold.retrieve(5, 100000);
        for entry in &result {
            assert!(!entry.contains("第3章"));
            assert!(!entry.contains("第4章"));
            assert!(!entry.contains("第5章"));
            assert!(!entry.contains("第6章"));
            assert!(!entry.contains("第7章"));
        }
        // 应包含 1,2,8,9,10
        assert!(result.iter().any(|e| e.contains("第1章")));
        assert!(result.iter().any(|e| e.contains("第10章")));
    }

    #[test]
    fn test_cold_memory_sorted() {
        let mut cold = ColdMemory::new(2);
        cold.insert_chapter(3, make_summary(3, "C", "s3", vec![]));
        cold.insert_chapter(1, make_summary(1, "A", "s1", vec![]));
        cold.insert_chapter(2, make_summary(2, "B", "s2", vec![]));

        // 当前第10章，所有章节都在冷记忆中
        let result = cold.retrieve(10, 100000);
        assert_eq!(result.len(), 3);
        // 应按 ID 排序
        assert!(result[0].contains("第1章"));
        assert!(result[1].contains("第2章"));
        assert!(result[2].contains("第3章"));
    }

    #[test]
    fn test_cold_memory_budget_limit() {
        let mut cold = ColdMemory::new(2);
        for i in 1..=5 {
            cold.insert_chapter(
                i,
                make_summary(
                    i,
                    &format!("标题{}", i),
                    &format!("这是一段很长的摘要内容{}", i),
                    vec![],
                ),
            );
        }

        // 当前第10章，budget 很小
        let result = cold.retrieve(10, 5);
        assert!(result.len() < 5);
    }

    #[test]
    fn test_cold_memory_boundary() {
        let mut cold = ColdMemory::new(2);
        cold.insert_chapter(3, make_summary(3, "C", "s3", vec![]));
        cold.insert_chapter(4, make_summary(4, "D", "s4", vec![]));
        cold.insert_chapter(5, make_summary(5, "E", "s5", vec![]));
        cold.insert_chapter(6, make_summary(6, "F", "s6", vec![]));
        cold.insert_chapter(7, make_summary(7, "G", "s7", vec![]));

        // 当前第5章，±2 = 3,4,5,6,7 全部排除
        let result = cold.retrieve(5, 100000);
        assert!(result.is_empty());
    }
}
