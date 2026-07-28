/// Layer 4 审美层类型定义
use crate::id::AntiAiRuleId;

/// 审美层
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AestheticLayer {
    /// 风格指纹
    pub style_fingerprint: StyleFingerprint,
    /// 节奏模型
    pub pacing_model: PacingModel,
    /// 反 AI 规则
    pub anti_ai_rules: Vec<AntiAiRule>,
}

/// 风格指纹
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StyleFingerprint {
    /// 平均句子长度
    pub sentence_length_avg: f32,
    /// 词汇丰富度
    pub vocabulary_richness: f32,
    /// 修辞频率
    pub rhetorical_frequency: f32,
    /// 对话比例
    pub dialogue_ratio: f32,
    /// 平均段落长度
    pub paragraph_length_avg: f32,
    /// 样本文本
    pub sample_texts: Vec<String>,
}

/// 节奏模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacingModel {
    /// 张力曲线 (章节, 张力值)
    pub tension_curve: Vec<(i64, f32)>,
    /// 平均场景长度
    pub scene_length_avg: f32,
    /// 动作比例
    pub action_ratio: f32,
}

/// 反 AI 规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntiAiRule {
    /// 规则 ID
    pub rule_id: AntiAiRuleId,
    /// 模式
    pub pattern: String,
    /// 处理动作
    pub action: AntiAiAction,
    /// 原因
    pub reason: String,
}

/// 反 AI 处理动作
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AntiAiAction {
    /// 重写
    Rewrite,
    /// 标记
    Flag,
    /// 移除
    Remove,
}
