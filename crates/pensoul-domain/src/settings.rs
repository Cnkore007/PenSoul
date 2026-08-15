// settings.rs — 项目设置类型定义

use serde::{Deserialize, Serialize};

/// 项目设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// 目标章数
    pub target_chapters: u32,
    /// 目标总字数
    pub target_words: u64,
    /// 每章目标字数
    pub chapter_target_words: u32,
    /// 目标卷数
    pub target_volumes: u32,
    /// 类型/题材
    pub genre: String,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            target_chapters: 100,
            target_words: 2_000_000,
            // 每章 2 万字，100 章 = 200 万字，与 target_words 自洽
            chapter_target_words: 20_000,
            target_volumes: 5,
            genre: String::new(),
        }
    }
}

impl ProjectSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// 根据章数和每章字数重新计算目标总字数
    pub fn recalc_target_words(&mut self) {
        self.target_words = self.target_chapters as u64 * self.chapter_target_words as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_target_words_is_consistent() {
        let settings = ProjectSettings::default();
        assert_eq!(
            settings.target_words,
            settings.target_chapters as u64 * settings.chapter_target_words as u64
        );
    }
}
