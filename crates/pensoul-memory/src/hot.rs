use std::collections::HashMap;

use crate::packet::estimate_tokens;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
