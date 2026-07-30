/// 创作设定 — 项目级创作目标
///
/// 在前后端同步存储，Agent 工作流可读取作为执行目标。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSettings {
    /// 目标总章数
    pub target_chapters: u32,
    /// 目标总字数（由 target_chapters × chapter_target_words 自动计算）
    pub target_words: u64,
    /// 每章目标字数
    pub chapter_target_words: u32,
    /// 预计卷数
    pub target_volumes: u32,
    /// 故事类型
    pub genre: String,
}

impl ProjectSettings {
    /// 创建默认设定
    pub fn new() -> Self {
        Self {
            target_chapters: 0,
            target_words: 0,
            chapter_target_words: 0,
            target_volumes: 0,
            genre: String::new(),
        }
    }

    /// 自动计算目标总字数 = 目标章数 × 每章字数
    pub fn recalc_target_words(&mut self) {
        self.target_words = self.target_chapters as u64 * self.chapter_target_words as u64;
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_zero() {
        let s = ProjectSettings::new();
        assert_eq!(s.target_chapters, 0);
        assert_eq!(s.target_words, 0);
        assert_eq!(s.chapter_target_words, 0);
        assert_eq!(s.target_volumes, 0);
        assert!(s.genre.is_empty());
    }

    #[test]
    fn test_recalc_target_words_multiplies() {
        let mut s = ProjectSettings::new();
        s.target_chapters = 100;
        s.chapter_target_words = 3000;
        s.recalc_target_words();
        assert_eq!(s.target_words, 300_000);
    }

    #[test]
    fn test_recalc_target_words_zero_chapters() {
        let mut s = ProjectSettings::new();
        s.chapter_target_words = 3000;
        s.recalc_target_words();
        assert_eq!(s.target_words, 0);
    }

    #[test]
    fn test_recalc_target_words_overwrites_stale_value() {
        let mut s = ProjectSettings::new();
        s.target_words = 999;
        s.target_chapters = 10;
        s.chapter_target_words = 2000;
        s.recalc_target_words();
        assert_eq!(s.target_words, 20_000);
    }

    #[test]
    fn test_settings_serde_round_trip() {
        let mut s = ProjectSettings::new();
        s.target_chapters = 50;
        s.chapter_target_words = 4000;
        s.recalc_target_words();
        s.genre = "玄幻".to_string();
        let json = serde_json::to_string(&s).unwrap();
        let back: ProjectSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.target_words, 200_000);
        assert_eq!(back.genre, "玄幻");
    }
}
