//! 反 AI 味检测器 —— 按公开的「去 AI 味」方法论（AI 高频词/弱化副词/书面连接词/
//! 意义膨胀/情绪直说五类模式）对章节正文做规则统计，输出 0-100 的 AI 痕迹分与违例清单。
//! 词表与计分来自全局配置（anti_ai.rs），可在「墨韵」页编辑。
use crate::anti_ai::AntiAiCategory;
use crate::state::AppState;
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

/// 检测章节正文，返回 0-100 的 AI 痕迹报告。
#[tauri::command]
pub async fn analyze_ai_flavor(
    state: tauri::State<'_, AppState>,
    content: String,
) -> Result<AiFlavorReport, String> {
    let categories = state.anti_ai.read().categories.clone();
    Ok(detect_ai_flavor(&content, &categories))
}

/// 读取反 AI 味规则配置（词表/计分/提示词）
#[tauri::command]
pub async fn get_anti_ai_rules(
    state: tauri::State<'_, AppState>,
) -> Result<crate::anti_ai::AntiAiRuleConfig, String> {
    Ok(state.anti_ai.read().clone())
}

/// 保存反 AI 味规则配置（写盘 + 更新内存，随后写入/审查工作流生效）
#[tauri::command]
pub async fn save_anti_ai_rules(
    state: tauri::State<'_, AppState>,
    config: crate::anti_ai::AntiAiRuleConfig,
) -> Result<(), String> {
    let config_dir = state.config_dir();
    crate::anti_ai::save_to_disk(&config_dir, &config)?;
    *state.anti_ai.write() = config;
    Ok(())
}

/// 同步检测入口（供测试与内部复用）
pub fn detect_ai_flavor(content: &str, categories: &[AntiAiCategory]) -> AiFlavorReport {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return AiFlavorReport {
            score: 0.0,
            level: "低".to_string(),
            total_hits: 0,
            categories: categories
                .iter()
                .map(|c| FlavorCategory {
                    key: c.key.clone(),
                    label: c.label.clone(),
                    hits: 0,
                    score: 0.0,
                    max_score: c.max_score,
                    examples: Vec::new(),
                })
                .collect(),
            suggestion: "文本为空，无可检测内容".to_string(),
        };
    }

    // 千字数（按字符粗略估算，中文 1 字符 ≈ 1 字）
    let char_count = trimmed.chars().count();
    let per_thousand = char_count as f64 / 1000.0;
    let mut cats = Vec::new();
    let mut total_score = 0.0_f64;
    let mut total_hits = 0usize;

    for c in categories {
        let mut hits = 0usize;
        let mut examples: Vec<String> = Vec::new();
        for w in &c.words {
            let wbytes = w.len();
            let mut start = 0;
            while let Some(rel) = trimmed[start..].find(w.as_str()) {
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
        let effective = if c.exempt_per_1k > 0 {
            let allowed = (per_thousand * c.exempt_per_1k as f64) as usize;
            hits.saturating_sub(allowed)
        } else {
            hits
        };
        let score = (effective as f64 * c.score_per_hit).min(c.max_score);
        total_score += score;
        total_hits += hits;
        cats.push(FlavorCategory {
            key: c.key.clone(),
            label: c.label.clone(),
            hits,
            score,
            max_score: c.max_score,
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
        categories: cats,
        suggestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rules() -> Vec<AntiAiCategory> {
        crate::anti_ai::AntiAiRuleConfig::default().categories
    }

    #[test]
    fn test_detects_ai_cliches() {
        let text = "他不禁感到一阵心悸，仿佛有什么大事要发生。她嘴角微扬，勾起一抹弧度。";
        let report = detect_ai_flavor(text, &test_rules());
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
        let report = detect_ai_flavor(&text, &test_rules());
        let weak = report
            .categories
            .iter()
            .find(|c| c.key == "weak_adverb")
            .unwrap();
        assert_eq!(weak.score, 0.0);
    }

    #[test]
    fn test_empty_text() {
        let report = detect_ai_flavor("   ", &test_rules());
        assert_eq!(report.score, 0.0);
        assert_eq!(report.total_hits, 0);
    }
}
