// intent.rs — 意图识别
// 识别用户当前的创作意图

use crate::types::{RetrievalContext, WritingIntent};

/// 意图识别器
pub struct IntentRecognizer;

impl IntentRecognizer {
    /// 根据上下文识别创作意图
    pub fn recognize(context: &RetrievalContext) -> WritingIntent {
        // 基础实现：根据编辑模式推断
        match context.editing_mode {
            crate::types::EditingMode::Drafting => WritingIntent::NewContent,
            crate::types::EditingMode::Revising => WritingIntent::ModifyContent,
            crate::types::EditingMode::Reviewing => WritingIntent::ReviewConsistency,
        }
    }
}
