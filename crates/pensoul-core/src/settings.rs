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
