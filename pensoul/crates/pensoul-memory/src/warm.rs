use std::collections::HashMap;

use crate::packet::{estimate_tokens, ChapterSummary, WarmMemoryData};

/// 温记忆 — 结构化摘要 + 角色状态 + 伏笔
pub struct WarmMemory {
    /// 章节 ID → 章节摘要
    chapters: HashMap<i64, ChapterSummary>,
    /// 活跃伏笔列表
    active_foreshadows: Vec<String>,
    /// 当前角色状态（汇总文本）
    character_states: Option<String>,
}

impl WarmMemory {
    pub fn new() -> Self {
        Self {
            chapters: HashMap::new(),
            active_foreshadows: Vec::new(),
            character_states: None,
        }
    }

    /// 插入章节摘要
    pub fn insert_chapter(&mut self, chapter_id: i64, summary: ChapterSummary) {
        self.chapters.insert(chapter_id, summary);
    }

    /// 设置活跃伏笔
    pub fn set_foreshadows(&mut self, foreshadows: Vec<String>) {
        self.active_foreshadows = foreshadows;
    }

    /// 设置角色状态
    pub fn set_character_states(&mut self, states: String) {
        self.character_states = Some(states);
    }

    /// 构建温记忆数据，受 budget (token 数) 控制
    ///
    /// 返回卷摘要 + 活跃伏笔 + 角色状态
    pub fn build(&self, _current_chapter: i64, budget: usize) -> WarmMemoryData {
        let mut tokens_used = 0usize;

        // 1. 卷摘要 — 逐章加入，受预算控制
        let volume_summary = self.build_volume_summary_within_budget(budget, &mut tokens_used);

        // 2. 活跃伏笔 — 逐条加入，受预算控制
        let mut active_foreshadows = Vec::new();
        for fs in &self.active_foreshadows {
            let tokens = estimate_tokens(fs);
            if tokens_used + tokens > budget {
                break;
            }
            tokens_used += tokens;
            active_foreshadows.push(fs.clone());
        }

        // 3. 角色状态 — 如果还有预算则加入
        let character_states = if let Some(ref states) = self.character_states {
            let tokens = estimate_tokens(states);
            if tokens_used + tokens <= budget {
                Some(states.clone())
            } else {
                None
            }
        } else {
            None
        };

        WarmMemoryData {
            volume_summary,
            active_foreshadows,
            character_states,
        }
    }

    /// 逐章构建卷摘要，受 budget 控制
    fn build_volume_summary_within_budget(&self, budget: usize, tokens_used: &mut usize) -> String {
        if self.chapters.is_empty() {
            return String::new();
        }

        let mut sorted: Vec<_> = self.chapters.iter().collect();
        sorted.sort_by_key(|(id, _)| *id);

        let mut parts = Vec::new();
        for (id, s) in sorted {
            let entry = format!("第{}章「{}」: {}", id, s.title, s.summary);
            let tokens = estimate_tokens(&entry);
            if *tokens_used + tokens > budget {
                break;
            }
            *tokens_used += tokens;
            parts.push(entry);
        }

        parts.join("\n")
    }

    pub fn chapters(&self) -> &HashMap<i64, ChapterSummary> {
        &self.chapters
    }

    /// 移除指定章节并返回摘要
    pub fn remove_chapter(&mut self, chapter_id: i64) -> Option<ChapterSummary> {
        self.chapters.remove(&chapter_id)
    }

    /// 返回章节数
    pub fn chapter_count(&self) -> usize {
        self.chapters.len()
    }
}

impl Default for WarmMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::ChapterId;

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
    fn test_warm_memory_build_volume_summary() {
        let mut warm = WarmMemory::new();
        warm.insert_chapter(1, make_summary(1, "开端", "故事开始"));
        warm.insert_chapter(2, make_summary(2, "发展", "冲突出现"));
        warm.insert_chapter(3, make_summary(3, "高潮", "矛盾激化"));

        let data = warm.build(3, 10000);
        assert!(data.volume_summary.contains("第1章"));
        assert!(data.volume_summary.contains("第2章"));
        assert!(data.volume_summary.contains("第3章"));
        assert!(data.volume_summary.contains("开端"));
        assert!(data.volume_summary.contains("冲突出现"));
    }

    #[test]
    fn test_warm_memory_foreshadows() {
        let mut warm = WarmMemory::new();
        warm.set_foreshadows(vec!["伏笔A".into(), "伏笔B".into()]);

        let data = warm.build(1, 10000);
        assert_eq!(data.active_foreshadows.len(), 2);
        assert!(data.active_foreshadows.contains(&"伏笔A".to_string()));
    }

    #[test]
    fn test_warm_memory_character_states() {
        let mut warm = WarmMemory::new();
        warm.set_character_states("角色状态汇总".into());

        let data = warm.build(1, 10000);
        assert_eq!(data.character_states.as_deref(), Some("角色状态汇总"));
    }

    #[test]
    fn test_warm_memory_budget_limit_foreshadows() {
        let mut warm = WarmMemory::new();
        // 每个伏笔 6 字 / 2 = 3 tokens
        warm.set_foreshadows(vec![
            "伏笔Alpha".into(),
            "伏笔Beta".into(),
            "伏笔Gamma".into(),
        ]);

        // budget 只够放 1 个伏笔 (约 3 tokens)
        let data = warm.build(1, 4);
        assert!(data.active_foreshadows.len() <= 1);
    }

    #[test]
    fn test_warm_memory_empty() {
        let warm = WarmMemory::new();
        let data = warm.build(1, 10000);
        assert!(data.volume_summary.is_empty());
        assert!(data.active_foreshadows.is_empty());
        assert!(data.character_states.is_none());
    }
}
