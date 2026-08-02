//! 反 AI 味检测器 —— 按公开的「去 AI 味」方法论（AI 高频词/弱化副词/书面连接词/
//! 意义膨胀/情绪直说五类模式）对章节正文做规则统计，输出 0-100 的 AI 痕迹分与违例清单。
//!
//! 标准（可执行）：
//! - AI 套话：每次 6 分，上限 30
//! - 弱化副词：每千字超过 3 个后每处 4 分，上限 20
//! - 书面/论文连接词：每处 5 分，上限 15
//! - 意义膨胀/空洞结论：每处 5 分，上限 15
//! - 情绪直说：每处 5 分，上限 20
//! 总分 = 各类合计（0-100）；0-15 低（自然）、15-35 中、>35 高（AI 味重）。
use serde::Serialize;

/// 单类检测结果
#[derive(Debug, Clone, Serialize)]
pub struct FlavorCategory {
    pub key: String,
    pub label: String,
    pub hits: usize,
    pub score: f64,
    pub max_score: f64,
    /// 违例原文样例（去重，最多 5 条）
    pub examples: Vec<String>,
}

/// 反 AI 味检测报告
#[derive(Debug, Clone, Serialize)]
pub struct AiFlavorReport {
    /// 0-100，越高 AI 味越重
    pub score: f64,
    /// 低 / 中 / 高
    pub level: String,
    pub total_hits: usize,
    pub categories: Vec<FlavorCategory>,
    /// 一句话结论建议
    pub suggestion: String,
}

/// 五类 AI 痕迹模式定义：(slug, 中文名, 命中词表, 单次计分, 上限, 每千字豁免数, 建议)
const CATEGORIES: [(&str, &str, &[&str], f64, f64, usize, &str); 5] = [
    (
        "cliche",
        "AI 套话",
        &[
            "不禁",
            "仿佛",
            "宛如",
            "犹如",
            "映入眼帘",
            "心中暗道",
            "暗自思忖",
            "嘴角微扬",
            "勾起一抹",
            "脸色一变",
            "身形一顿",
            "不由自主",
            "情不自禁",
            "目光如炬",
            "目光深邃",
            "只见",
            "此时此刻",
            "沉声道",
            "淡淡地说",
            "心头一紧",
            "倒吸一口凉气",
            "心中一惊",
            "暗暗发誓",
            "眼神一凝",
            "空气仿佛凝固",
        ],
        6.0,
        30.0,
        0,
        "删除套话，改为具体动作或直接删掉",
    ),
    (
        "weak_adverb",
        "弱化副词",
        &[
            "微微", "淡淡", "缓缓", "轻轻", "悄然", "默默", "隐隐", "稍稍", "略显",
        ],
        4.0,
        20.0,
        3,
        "每千字不超过 3 个，多余的删除或改为具体动作",
    ),
    (
        "paper_connector",
        "书面连接词",
        &[
            "与此同时",
            "从而",
            "于是乎",
            "诚然",
            "由此可见",
            "不难看出",
            "事实上",
            "值得注意的是",
            "综上所述",
            "总而言之",
        ],
        5.0,
        15.0,
        0,
        "删除或改为口语化/行动化表达",
    ),
    (
        "inflation",
        "意义膨胀",
        &[
            "意义深远",
            "前所未有",
            "可谓",
            "未来可期",
            "前途无量",
            "充满希望",
            "不可小觑",
            "不容小觑",
            "石破天惊",
            "荡气回肠",
        ],
        5.0,
        15.0,
        0,
        "删标签，用具体的后续影响替代",
    ),
    (
        "emotion_telling",
        "情绪直说",
        &[
            "他感到",
            "她感到",
            "心中涌起",
            "心中充满",
            "心中泛起",
            "心中升起",
            "顿时觉得",
            "顿时感到",
            "一股寒意",
            "一股暖流",
            "一股怒火",
            "莫名的恐惧",
            "莫名的悲伤",
            "莫名的紧张",
        ],
        5.0,
        20.0,
        0,
        "用动作和感知代替情绪直说，如「后背出了一层冷汗」",
    ),
];

/// 检测章节正文，返回 0-100 的 AI 痕迹报告。
#[tauri::command]
pub async fn analyze_ai_flavor(content: String) -> Result<AiFlavorReport, String> {
    Ok(detect_ai_flavor(&content))
}

/// 同步检测入口（供测试与内部复用）
pub fn detect_ai_flavor(content: &str) -> AiFlavorReport {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return AiFlavorReport {
            score: 0.0,
            level: "低".to_string(),
            total_hits: 0,
            categories: CATEGORIES
                .iter()
                .map(|(key, label, _, _, max, _, _)| FlavorCategory {
                    key: key.to_string(),
                    label: label.to_string(),
                    hits: 0,
                    score: 0.0,
                    max_score: *max,
                    examples: Vec::new(),
                })
                .collect(),
            suggestion: "文本为空，无可检测内容".to_string(),
        };
    }

    // 千字数（按字符粗略估算，中文 1 字符 ≈ 1 字）
    let char_count = trimmed.chars().count();
    let per_thousand = char_count as f64 / 1000.0;
    let mut categories = Vec::new();
    let mut total_score = 0.0_f64;
    let mut total_hits = 0usize;

    for (key, label, words, per_hit, max_score, allowance, _suggestion) in CATEGORIES {
        let mut hits = 0usize;
        let mut examples: Vec<String> = Vec::new();
        for w in words {
            let wbytes = w.len();
            let mut start = 0;
            while let Some(rel) = trimmed[start..].find(w) {
                let idx = start + rel;
                hits += 1;
                // 取命中位置前后一小段作为样例（按字符窗口，避免切坏 UTF-8）
                if examples.len() < 5 {
                    let prefix: String = trimmed[..idx]
                        .chars()
                        .rev()
                        .take(12)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    let suffix: String = trimmed[idx + wbytes..].chars().take(12).collect();
                    let snippet = format!("{prefix}「{w}」{suffix}");
                    if !examples.contains(&snippet) {
                        examples.push(snippet);
                    }
                }
                start = idx + wbytes;
            }
        }
        // 弱化副词按每千字密度豁免；其余类别 0 豁免
        let effective = if allowance > 0 {
            let allowed = (per_thousand * allowance as f64) as usize;
            hits.saturating_sub(allowed)
        } else {
            hits
        };
        let score = (effective as f64 * per_hit).min(max_score);
        total_score += score;
        total_hits += hits;
        categories.push(FlavorCategory {
            key: key.to_string(),
            label: label.to_string(),
            hits,
            score,
            max_score,
            examples,
        });
    }

    let score = total_score.min(100.0);
    let level = if score <= 15.0 {
        "低".to_string()
    } else if score <= 35.0 {
        "中".to_string()
    } else {
        "高".to_string()
    };
    let suggestion = if level == "低" {
        "文风较自然，未见明显 AI 痕迹".to_string()
    } else if level == "中" {
        "存在可感知的 AI 痕迹，建议按命中清单逐条替换（优先处理套话与情绪直说）".to_string()
    } else {
        "AI 痕迹明显，建议重写涉及命中片段：删套话、控弱化副词、用具体动作代替情绪直说".to_string()
    };

    AiFlavorReport {
        score,
        level,
        total_hits,
        categories,
        suggestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_ai_cliches() {
        let text = "他不禁感到一阵心悸，仿佛有什么大事要发生。她嘴角微扬，勾起一抹弧度。";
        let report = detect_ai_flavor(text);
        let cliche = report
            .categories
            .iter()
            .find(|c| c.key == "cliche")
            .unwrap();
        assert!(cliche.hits >= 4); // 不禁 / 仿佛 / 嘴角微扬 / 勾起一抹
        assert!(report.score > 20.0);
        assert!(!cliche.examples.is_empty());
    }

    #[test]
    fn test_weak_adverb_density_allowance() {
        // 2000 字只出现 3 个弱化副词 → 豁免（每千字 ≤3）
        let mut text = "他微微点了点头。".to_string();
        while text.chars().count() < 2000 {
            text.push_str("他继续往前走，没有说话。");
        }
        let report = detect_ai_flavor(&text);
        let weak = report
            .categories
            .iter()
            .find(|c| c.key == "weak_adverb")
            .unwrap();
        assert_eq!(weak.score, 0.0);
    }

    #[test]
    fn test_empty_text() {
        let report = detect_ai_flavor("   ");
        assert_eq!(report.score, 0.0);
        assert_eq!(report.total_hits, 0);
    }
}
