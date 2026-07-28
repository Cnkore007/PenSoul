/// 文风工坊视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleWorkshopState {
    /// 风格指纹
    pub style_fingerprint: StyleFingerprintView,
    /// 节奏模型
    pub pacing_model: PacingModelView,
    /// 反 AI 规则列表
    pub anti_ai_rules: Vec<String>,
    /// 当前选中的规则
    pub selected_rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleFingerprintView {
    pub sentence_length_avg: f32,
    pub vocabulary_richness: f32,
    pub dialogue_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingModelView {
    pub tension_curve: Vec<f32>,
    pub action_ratio: f32,
}

impl StyleWorkshopState {
    pub fn new() -> Self {
        Self {
            style_fingerprint: StyleFingerprintView {
                sentence_length_avg: 0.0,
                vocabulary_richness: 0.0,
                dialogue_ratio: 0.0,
            },
            pacing_model: PacingModelView {
                tension_curve: Vec::new(),
                action_ratio: 0.0,
            },
            anti_ai_rules: Vec::new(),
            selected_rule: None,
        }
    }

    pub fn reset(&mut self) {
        self.style_fingerprint = StyleFingerprintView {
            sentence_length_avg: 0.0,
            vocabulary_richness: 0.0,
            dialogue_ratio: 0.0,
        };
        self.pacing_model = PacingModelView {
            tension_curve: Vec::new(),
            action_ratio: 0.0,
        };
        self.anti_ai_rules.clear();
        self.selected_rule = None;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for StyleWorkshopState {
    fn default() -> Self {
        Self::new()
    }
}
