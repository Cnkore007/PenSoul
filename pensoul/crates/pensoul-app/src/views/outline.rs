/// 大纲视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineViewState {
    /// 大纲内容
    pub outline: String,
    /// 章节列表摘要
    pub chapter_summaries: Vec<ChapterSummaryItem>,
    /// 当前选中的章节
    pub selected_chapter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummaryItem {
    pub chapter_id: String,
    pub title: String,
    pub summary: String,
}

impl OutlineViewState {
    pub fn new() -> Self {
        Self {
            outline: String::new(),
            chapter_summaries: Vec::new(),
            selected_chapter: None,
        }
    }

    pub fn reset(&mut self) {
        self.outline.clear();
        self.chapter_summaries.clear();
        self.selected_chapter = None;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for OutlineViewState {
    fn default() -> Self {
        Self::new()
    }
}
