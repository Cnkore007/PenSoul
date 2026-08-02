//! 热记忆与冷记忆 — 窗口内全文与窗口外摘要检索
use std::collections::HashMap;

use crate::packet::{ChapterSummary, estimate_tokens};

/// 热记忆 — 保留当前章 ± window_size 的完整文本
pub struct HotMemory {
    /// 章节 ID → 完整文本
    full_texts: HashMap<i64, String>,
    /// 窗口大小（前后各取多少章）
    window_size: usize,
}

impl HotMemory {
    pub fn new(window_size: usize) -> Self {
        Self {
            full_texts: HashMap::new(),
            window_size,
        }
    }

    /// 插入章节文本
    pub fn insert(&mut self, chapter_id: i64, text: String) {
        self.full_texts.insert(chapter_id, text);
    }

    /// 获取当前窗口范围内的所有章节 ID（有序）
    pub fn window_chapters(&self, current_chapter: i64) -> Vec<i64> {
        let mut ids: Vec<i64> = self
            .full_texts
            .keys()
            .filter(|&&id| {
                let diff = (id - current_chapter).unsigned_abs() as usize;
                diff <= self.window_size
            })
            .copied()
            .collect();
        ids.sort();
        ids
    }

    /// 构建热记忆文本，受 budget (token 数) 控制
    ///
    /// 从当前章往前取 window_size 章，标注 [当前章]/[前一章]/[前前章]
    pub fn build(&self, current_chapter: i64, budget: usize) -> Vec<String> {
        let mut ids = self.window_chapters(current_chapter);
        ids.sort(); // 确保有序

        let mut result = Vec::new();
        let mut tokens_used = 0usize;

        for id in &ids {
            // 跳过未来章节（id > current_chapter）
            if *id > current_chapter {
                continue;
            }

            if let Some(text) = self.full_texts.get(id) {
                let label: String = match current_chapter - id {
                    0 => "[当前章]".to_string(),
                    1 => "[前一章]".to_string(),
                    2 => "[前前章]".to_string(),
                    _ => continue,
                };

                let entry = format!("{} 章节{}:\n{}", label, id, text);
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

    /// 返回窗口内章节数
    pub fn len(&self, current_chapter: i64) -> usize {
        self.window_chapters(current_chapter).len()
    }

    /// 返回窗口大小配置
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// 热记忆是否为空
    pub fn is_empty(&self) -> bool {
        self.full_texts.is_empty()
    }
}

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
    fn test_hot_memory_insert_and_build() {
        let mut hot = HotMemory::new(2);
        hot.insert(1, "第一章内容".into());
        hot.insert(2, "第二章内容".into());
        hot.insert(3, "第三章内容".into());
        hot.insert(4, "第四章内容".into());

        // 在第3章，窗口 ±2，排除未来章节，应包含 1,2,3
        let result = hot.build(3, 10000);
        assert_eq!(result.len(), 3);
        assert!(result[0].contains("[前前章]"));
        assert!(result[1].contains("[前一章]"));
        assert!(result[2].contains("[当前章]"));
    }

    #[test]
    fn test_hot_memory_window_size() {
        let mut hot = HotMemory::new(1);
        hot.insert(1, "第一章".into());
        hot.insert(2, "第二章".into());
        hot.insert(3, "第三章".into());

        // 窗口 ±1，当前在第2章，排除未来章节，应包含 1,2
        let result = hot.build(2, 10000);
        assert_eq!(result.len(), 2);

        // 窗口 ±1，当前在第1章，应包含 1
        let result = hot.build(1, 10000);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_hot_memory_budget_limit() {
        let mut hot = HotMemory::new(2);
        // 每章约 4 字 / 2 = 2 tokens
        hot.insert(1, "第一章内容是很多字".into());
        hot.insert(2, "第二章内容是很多字".into());
        hot.insert(3, "第三章内容是很多字".into());

        // budget 只够放 1 章
        let result = hot.build(2, 5);
        assert!(result.len() <= 1);
    }

    #[test]
    fn test_hot_memory_empty() {
        let hot = HotMemory::new(2);
        let result = hot.build(1, 10000);
        assert!(result.is_empty());
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
