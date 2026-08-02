//! 反 AI 味检测器 —— 按「去 AI 味」方法论（AI 高频词/弱化副词/书面连接词/
//! 意义膨胀/情绪直说/结构骨架/翻译腔）对章节正文做规则统计，
//! 输出 0-100 的 AI 痕迹分、违例清单与节奏信号。
//! 词表与计分来自全局配置（anti_ai.rs），可在「墨韵」页编辑。
use crate::anti_ai::AntiAiCategory;
use crate::state::AppState;
use regex::Regex;
use serde::Serialize;

/// 单类检测结果
#[derive(Debug, Clone, Serialize)]
pub struct FlavorCategory {
    pub key: String,
    pub label: String,
    /// 严重度分级（1 命中即扣 / 2 同段聚集 / 3 全文密度）
    pub tier: u8,
    pub hits: usize,
    pub score: f64,
    pub max_score: f64,
    /// 违例原文样例（去重，最多 5 条）
    pub examples: Vec<String>,
}

/// 节奏信号（确定性指标，信息性展示，不参与总分）
#[derive(Debug, Clone, Serialize)]
pub struct RhythmSignal {
    /// 平均句长（字）
    pub avg_sentence_length: f64,
    /// 句长方差（越小句子越整齐，AI 味风险越高）
    pub sentence_var: f64,
    /// 段落长度变异系数（值越低段落越均匀）
    pub paragraph_uniformity: f64,
    pub flagged: bool,
    pub note: String,
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
    pub rhythm: RhythmSignal,
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
                    tier: c.tier,
                    hits: 0,
                    score: 0.0,
                    max_score: c.max_score,
                    examples: Vec::new(),
                })
                .collect(),
            rhythm: RhythmSignal {
                avg_sentence_length: 0.0,
                sentence_var: 0.0,
                paragraph_uniformity: 0.0,
                flagged: false,
                note: "文本为空，无可检测内容".to_string(),
            },
            suggestion: "文本为空，无可检测内容".to_string(),
        };
    }

    // 千字数（按字符粗略估算，中文 1 字符 ≈ 1 字）
    let char_count = trimmed.chars().count();
    let per_thousand = char_count as f64 / 1000.0;

    // 段落拆分：按换行拆；无换行时整段作为一个段落
    let paragraphs: Vec<&str> = {
        let raw: Vec<&str> = trimmed
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if raw.is_empty() { vec![trimmed] } else { raw }
    };
    let para_lens = paragraph_lens(&paragraphs);

    let mut cats = Vec::new();
    let mut total_score = 0.0_f64;
    let mut total_hits = 0usize;

    for c in categories {
        let (hits, per_para, per_word) = count_category_hits(&paragraphs, c);
        let effective = effective_hits(
            c,
            &hits,
            &per_para,
            &per_word,
            char_count,
            per_thousand,
            &para_lens,
        );
        let score = (effective as f64 * c.score_per_hit).min(c.max_score);
        total_score += score;
        total_hits += hits;
        cats.push(FlavorCategory {
            key: c.key.clone(),
            label: c.label.clone(),
            tier: c.tier,
            hits,
            score,
            max_score: c.max_score,
            examples: collect_examples(trimmed, c, &paragraphs, &per_para),
        });
    }

    let rhythm = compute_rhythm(trimmed, &paragraphs);
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
        "存在可感知的 AI 痕迹，建议按命中清单逐条替换（优先处理套话、结构骨架与翻译腔）".to_string()
    } else {
        "AI 痕迹明显，建议重写涉及命中片段：删套话与二元对比空转、控弱化副词、拆翻译腔、用具体动作代替情绪直说".to_string()
    };

    AiFlavorReport {
        score,
        level,
        total_hits,
        categories: cats,
        rhythm,
        suggestion,
    }
}

/// 各段字符数（Tier 2 阈值判断用）
fn paragraph_lens(paragraphs: &[&str]) -> Vec<usize> {
    paragraphs.iter().map(|p| p.chars().count()).collect()
}

/// 统计某类别的命中：总次数 / 每段次数 / 每个词条次数（Tier 3 密度用）
fn count_category_hits(
    paragraphs: &[&str],
    c: &AntiAiCategory,
) -> (usize, Vec<usize>, Vec<(String, usize)>) {
    let mut total = 0usize;
    let mut per_para = vec![0usize; paragraphs.len()];
    let mut per_word: Vec<(String, usize)> = Vec::new();

    for w in &c.words {
        let mut wcount = 0usize;
        for (pi, para) in paragraphs.iter().enumerate() {
            let mut start = 0;
            while let Some(rel) = para[start..].find(w.as_str()) {
                let idx = start + rel;
                wcount += 1;
                per_para[pi] += 1;
                start = idx + w.len();
            }
        }
        total += wcount;
        if wcount > 0 {
            per_word.push((w.clone(), wcount));
        }
    }

    for p in &c.patterns {
        let Ok(re) = Regex::new(p) else {
            continue; // 非法正则按配置错误跳过，不让单条坏规则拖垮检测
        };
        let mut pcount = 0usize;
        for (pi, para) in paragraphs.iter().enumerate() {
            let n = re.find_iter(para).count();
            if n > 0 {
                pcount += n;
                per_para[pi] += n;
            }
        }
        total += pcount;
        if pcount > 0 {
            per_word.push((format!("【句式】{p}"), pcount));
        }
    }
    (total, per_para, per_word)
}

/// 按 Tier 语义折算有效命中数（用于计分，展示仍用原始 hits）
fn effective_hits(
    c: &AntiAiCategory,
    hits: &usize,
    per_para: &[usize],
    per_word: &[(String, usize)],
    char_count: usize,
    per_thousand: f64,
    para_lens: &[usize],
) -> usize {
    match c.tier {
        // Tier 2：同段聚集才计分（短段 <100 字 2+ 命中；长段 ≥100 字 3+ 命中）
        2 => {
            let mut eff = 0usize;
            for (pi, n) in per_para.iter().enumerate() {
                if *n == 0 {
                    continue;
                }
                let len = para_lens.get(pi).copied().unwrap_or(0);
                let threshold = if len < 100 { 2 } else { 3 };
                if *n >= threshold {
                    eff += *n;
                }
            }
            eff
        }
        3 => {
            // Tier 3：按全文密度豁免（短 <200 字同词 3+；中 ≤1000 字 5+；长按每千字 5 次）
            let allowed = if char_count < 200 {
                2usize
            } else if char_count <= 1000 {
                4usize
            } else {
                (per_thousand * 5.0) as usize
            };
            per_word
                .iter()
                .map(|(_, n)| n.saturating_sub(allowed))
                .sum()
        }
        // Tier 1：命中即计分；配置了每千字豁免的类别（如弱化副词）先按密度豁免
        _ => {
            if c.exempt_per_1k > 0 {
                let allowed = (per_thousand * c.exempt_per_1k as f64) as usize;
                hits.saturating_sub(allowed)
            } else {
                *hits
            }
        }
    }
}

/// 收集违例样例：Tier 2 优先取命中段落内的片段，其余取全文前 5 条去重片段
fn collect_examples(
    content: &str,
    c: &AntiAiCategory,
    paragraphs: &[&str],
    _per_para: &[usize],
) -> Vec<String> {
    let mut examples: Vec<String> = Vec::new();
    for w in &c.words {
        let mut start = 0;
        while let Some(rel) = content[start..].find(w.as_str()) {
            let idx = start + rel;
            let prefix: String = content[..idx]
                .chars()
                .rev()
                .take(12)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let suffix: String = content[idx + w.len()..].chars().take(12).collect();
            let snippet = format!("{prefix}「{w}」{suffix}");
            if !examples.contains(&snippet) {
                examples.push(snippet);
                if examples.len() >= 5 {
                    return examples;
                }
            }
            start = idx + w.len();
        }
    }
    // 句式样例：取第一个命中的段落截取
    for p in &c.patterns {
        let Ok(re) = Regex::new(p) else {
            continue;
        };
        for para in paragraphs {
            if let Some(m) = re.find(para) {
                let snippet = para[..m.end()]
                    .chars()
                    .rev()
                    .take(24)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>();
                let snippet = format!("「{snippet}」");
                if !examples.contains(&snippet) {
                    examples.push(snippet);
                    if examples.len() >= 5 {
                        return examples;
                    }
                }
                break;
            }
        }
    }
    examples
}

/// 计算节奏信号：句长方差 + 段落长度变异系数
fn compute_rhythm(content: &str, paragraphs: &[&str]) -> RhythmSignal {
    let sentences: Vec<usize> = content
        .split(|ch: char| "。！？…；\n".contains(ch))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().count())
        .collect();

    let (avg_sentence_length, sentence_var) = mean_var(&sentences);
    let para_lens: Vec<usize> = paragraph_lens(paragraphs);
    let (para_avg, para_var) = mean_var(&para_lens);
    let paragraph_uniformity = if para_avg > 0.0 {
        para_var.sqrt() / para_avg
    } else {
        0.0
    };

    let uniform_sentence = sentences.len() >= 4 && sentence_var < 5.0;
    let uniform_paragraph = paragraphs.len() >= 4 && paragraph_uniformity < 0.35;
    let flagged = uniform_sentence || uniform_paragraph;
    let mut notes = Vec::new();
    if uniform_sentence {
        notes.push("句子长度过于整齐（句长方差偏低），建议长短句交错".to_string());
    }
    if uniform_paragraph {
        notes.push("段落长度过于均匀，建议打破对称的段落节奏".to_string());
    }
    let note = if notes.is_empty() {
        "句长与段落节奏正常".to_string()
    } else {
        notes.join("；")
    };

    RhythmSignal {
        avg_sentence_length,
        sentence_var,
        paragraph_uniformity,
        flagged,
        note,
    }
}

/// 均值与总体方差
fn mean_var(values: &[usize]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let n = values.len() as f64;
    let avg = values.iter().sum::<usize>() as f64 / n;
    let var = values
        .iter()
        .map(|v| {
            let d = *v as f64 - avg;
            d * d
        })
        .sum::<f64>()
        / n;
    (avg, var)
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
    fn test_detects_structure_skeleton() {
        let text = "这不是技术问题，而是管理问题。这不仅是一场危机，还是一次机会。";
        let report = detect_ai_flavor(text, &test_rules());
        let structure = report
            .categories
            .iter()
            .find(|c| c.key == "structure")
            .unwrap();
        assert!(structure.hits >= 2);
    }

    #[test]
    fn test_detects_translationese() {
        let text = "对于这个方案而言，基于现有数据的判断使得优化得以实现。";
        let report = detect_ai_flavor(text, &test_rules());
        let trans = report
            .categories
            .iter()
            .find(|c| c.key == "translationese")
            .unwrap();
        assert!(trans.hits >= 2);
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
    fn test_tier2_paragraph_cluster() {
        // 短段内 2+ 个结构模式 → 计分；散落单段 → 不计分
        let clustered = "这不是巧合，而是宿命。不仅如此，他还是一切的根源。";
        let report = detect_ai_flavor(clustered, &test_rules());
        let structure = report
            .categories
            .iter()
            .find(|c| c.key == "structure")
            .unwrap();
        assert!(structure.score > 0.0);
    }

    #[test]
    fn test_rhythm_signal() {
        // 全篇相同长度短句 → 句长方差低，标记节奏风险
        let mut text = String::new();
        for _ in 0..20 {
            text.push_str("他往前走了一步。");
        }
        let report = detect_ai_flavor(&text, &test_rules());
        assert!(report.rhythm.flagged);
        assert!(report.rhythm.sentence_var < 1.0);
    }

    #[test]
    fn test_empty_text() {
        let report = detect_ai_flavor("   ", &test_rules());
        assert_eq!(report.score, 0.0);
        assert_eq!(report.total_hits, 0);
    }
}
