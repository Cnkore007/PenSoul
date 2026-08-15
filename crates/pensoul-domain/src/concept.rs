// concept.rs — 核心概念定义
// 高概念、前提、主角、基调、核心冲突

use serde::{Deserialize, Serialize};

/// 核心概念（高概念层）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreConcept {
    /// 一句话高概念
    pub high_concept: String,
    /// 故事前提
    pub premise: String,
    /// 主角提示
    pub protagonist_hint: String,
    /// 基调/调性
    pub tone: String,
    /// 核心冲突
    pub central_conflict: String,
    /// 灵感来源
    pub inspiration: String,
}

impl CoreConcept {
    pub fn new() -> Self {
        Self::default()
    }
}
