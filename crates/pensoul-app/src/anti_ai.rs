//! 反 AI 味规则配置 —— 词表/计分与「语言铁律」提示词，全局可编辑，
//! 保存后注入写作、审查、批注重写工作流。
use std::path::Path;

use serde::{Deserialize, Serialize};

/// 单类 AI 痕迹规则（词表 + 计分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiAiCategory {
    pub key: String,
    pub label: String,
    pub words: Vec<String>,
    /// 单次命中扣分
    pub score_per_hit: f64,
    /// 该类扣分上限
    pub max_score: f64,
    /// 每千字豁免数（弱化副词等密度类规则）
    pub exempt_per_1k: usize,
    pub suggestion: String,
}

/// 反 AI 味规则配置（全局，跨项目）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiAiRuleConfig {
    pub categories: Vec<AntiAiCategory>,
    /// 语言铁律提示词（注入写作/审查/重写 prompt）
    pub prompt: String,
}

/// 默认语言铁律（原 context.rs ANTI_AI_RULES）
pub const DEFAULT_PROMPT: &str = "\
语言铁律（反 AI 味）：
1. 删除套话：「不禁」「仿佛」「映入眼帘」「心中暗道」「嘴角微扬」「脸色一变」等一律不写；
2. 弱化副词（微微/淡淡/缓缓/轻轻/悄然/默默）每千字不超过 3 个；
3. 不用排比三连（三个词一组堆砌「全面感」）；
4. 不用「与此同时」「从而」「于是乎」「诚然」「一方面…另一方面…」等书面连接词；
5. 情绪不许直说：不写「他很担忧」，写「他的后背出了一层冷汗」；
6. 用具体细节代替判断：不写「她很聪明」，写她做了什么具体的聪明事；
7. 长短句交替，段落不超过五行，对话优先于叙述、行动优先于形容。";

/// 默认五类词表（原 ai_flavor.rs CATEGORIES）
fn default_categories() -> Vec<AntiAiCategory> {
    vec![
        AntiAiCategory {
            key: "cliche".to_string(),
            label: "AI 套话".to_string(),
            words: vec![
                "不禁", "仿佛", "宛如", "犹如", "映入眼帘", "心中暗道", "暗自思忖", "嘴角微扬",
                "勾起一抹", "脸色一变", "身形一顿", "不由自主", "情不自禁", "目光如炬",
                "目光深邃", "只见", "此时此刻", "沉声道", "淡淡地说", "心头一紧",
                "倒吸一口凉气", "心中一惊", "暗暗发誓", "眼神一凝", "空气仿佛凝固",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            score_per_hit: 6.0,
            max_score: 30.0,
            exempt_per_1k: 0,
            suggestion: "删除套话，改为具体动作或直接删掉".to_string(),
        },
        AntiAiCategory {
            key: "weak_adverb".to_string(),
            label: "弱化副词".to_string(),
            words: vec!["微微", "淡淡", "缓缓", "轻轻", "悄然", "默默", "隐隐", "稍稍", "略显"]
                .into_iter()
                .map(String::from)
                .collect(),
            score_per_hit: 4.0,
            max_score: 20.0,
            exempt_per_1k: 3,
            suggestion: "每千字不超过 3 个，多余的删除或改为具体动作".to_string(),
        },
        AntiAiCategory {
            key: "paper_connector".to_string(),
            label: "书面连接词".to_string(),
            words: vec![
                "与此同时", "从而", "于是乎", "诚然", "由此可见", "不难看出", "事实上",
                "值得注意的是", "综上所述", "总而言之",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            score_per_hit: 5.0,
            max_score: 15.0,
            exempt_per_1k: 0,
            suggestion: "删除或改为口语化/行动化表达".to_string(),
        },
        AntiAiCategory {
            key: "inflation".to_string(),
            label: "意义膨胀".to_string(),
            words: vec![
                "意义深远", "前所未有", "可谓", "未来可期", "前途无量", "充满希望", "不可小觑",
                "不容小觑", "石破天惊", "荡气回肠",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            score_per_hit: 5.0,
            max_score: 15.0,
            exempt_per_1k: 0,
            suggestion: "删标签，用具体的后续影响替代".to_string(),
        },
        AntiAiCategory {
            key: "emotion_telling".to_string(),
            label: "情绪直说".to_string(),
            words: vec![
                "他感到", "她感到", "心中涌起", "心中充满", "心中泛起", "心中升起", "顿时觉得",
                "顿时感到", "一股寒意", "一股暖流", "一股怒火", "莫名的恐惧", "莫名的悲伤",
                "莫名的紧张",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            score_per_hit: 5.0,
            max_score: 20.0,
            exempt_per_1k: 0,
            suggestion: "用动作和感知代替情绪直说，如「后背出了一层冷汗」".to_string(),
        },
    ]
}

impl Default for AntiAiRuleConfig {
    fn default() -> Self {
        Self {
            categories: default_categories(),
            prompt: DEFAULT_PROMPT.to_string(),
        }
    }
}

/// 从 `_config/anti-ai-rules.json` 加载；不存在或损坏时返回默认
pub fn load_or_default(config_dir: &Path) -> AntiAiRuleConfig {
    let path = config_dir.join("anti-ai-rules.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AntiAiRuleConfig::default(),
    }
}

/// 保存配置到 `_config/anti-ai-rules.json`
pub fn save_to_disk(config_dir: &Path, config: &AntiAiRuleConfig) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("anti-ai-rules.json"), text).map_err(|e| e.to_string())
}
