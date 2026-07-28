/// 写作视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingViewState {
    /// 当前编辑的章节 ID
    pub current_chapter_id: Option<String>,
    /// 编辑器内容
    pub content: String,
    /// 光标位置
    pub cursor_position: usize,
    /// 是否有未保存的更改
    pub is_dirty: bool,
    /// 版本号（用于乐观锁）
    pub version: i32,
}

impl WritingViewState {
    pub fn new() -> Self {
        Self {
            current_chapter_id: None,
            content: String::new(),
            cursor_position: 0,
            is_dirty: false,
            version: 1,
        }
    }

    pub fn reset(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
        self.is_dirty = false;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for WritingViewState {
    fn default() -> Self {
        Self::new()
    }
}
