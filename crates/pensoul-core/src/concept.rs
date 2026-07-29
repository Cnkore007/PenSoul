/// 核心概念 / 高概念 — 整部小说的"种子"
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreConcept {
    /// 高概念 / 核心想法（一句话概括）
    pub high_concept: String,
    /// 故事前提 / 冲突前提
    pub premise: String,
    /// 主角雏形
    pub protagonist_hint: String,
    /// 故事基调 / 风格
    pub tone: String,
    /// 核心冲突
    pub central_conflict: String,
    /// 灵感来源 / 创作缘由
    pub inspiration: String,
}

impl CoreConcept {
    pub fn new() -> Self {
        Self {
            high_concept: String::new(),
            premise: String::new(),
            protagonist_hint: String::new(),
            tone: String::new(),
            central_conflict: String::new(),
            inspiration: String::new(),
        }
    }
}

impl Default for CoreConcept {
    fn default() -> Self {
        Self::new()
    }
}
