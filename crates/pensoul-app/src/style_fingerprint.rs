//! 文风指纹 —— 从本书已有章节确定性统计句长/段落/连接词/标点/引号/对话习惯，
//! 形成「本书文风基线」注入写作与审查 prompt，防止模型把文字改得比基线更整齐的模板腔。
use crate::state::AppState;
use pensoul_core::NovelOntology;
use serde::Serialize;
use std::collections::HashSet;

/// 采样章数上限：最近 40 章 + 开头 5 章
const MAX_SAMPLED_CHAPTERS: usize = 45;
/// 采样字符上限：约 30 万字封顶（长书避免全量扫描拖慢 prompt 组装）
const MAX_SAMPLED_CHARS: usize = 300_000;

/// 书面连接词（文风指纹的密度口径，与检测词表口径一致）
const CONNECTORS: &[&str] = &[
    "与此同时",
    "然而",
    "此外",
    "事实上",
    "实际上",
    "值得注意的是",
    "总而言之",
    "综上所述",
    "由此可见",
    "不难看出",
    "诚然",
    "一方面",
    "另一方面",
    "不仅如此",
    "归根结底",
    "本质上",
    "首先",
    "其次",
    "最后",
];

/// 文风指纹（确定性统计，无 LLM 成本）
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StyleFingerprint {
    pub sampled_chapters: usize,
    pub sampled_chars: usize,
    /// 平均句长（字）
    pub avg_sentence_length: f64,
    /// 句长方差（越小句子越整齐，AI 味风险越高）
    pub sentence_var: f64,
    /// 平均段落长度（字）
    pub avg_paragraph_length: f64,
    /// 段落长度变异系数
    pub paragraph_uniformity: f64,
    /// 每千字书面连接词数
    pub connector_per_1k: f64,
    /// 每千字破折号（——）处数
    pub dash_per_1k: f64,
    /// 每千字冒号处数
    pub colon_per_1k: f64,
    /// 引号习惯：`「」` / `“”`
    pub quote_style: String,
    /// 对话占比（引号内字符占比）
    pub dialogue_ratio: f64,
    /// 词汇丰富度（字符 TTR：去重字符数 / 总字符数）
    pub vocabulary_richness: f64,
}

/// 单段文本的统计中间量
#[derive(Default)]
struct TextStats {
    sentence_lens: Vec<usize>,
    para_lens: Vec<usize>,
    connectors: usize,
    dashes: usize,
    colons: usize,
    quote_open_jp: usize,
    quote_open_cn: usize,
    quoted_chars: usize,
    total_chars: usize,
    unique_chars: HashSet<char>,
}

/// 从本书本体计算文风指纹（采样最近章节 + 开篇，30 万字封顶）
pub fn compute_fingerprint(onto: &NovelOntology) -> StyleFingerprint {
    let mut sampled: Vec<String> = Vec::new();
    let mut total_chars = 0usize;
    let mut chapters: Vec<&pensoul_core::Chapter> = onto
        .chapters
        .iter()
        .filter(|c| c.chapter_no > 0 && !c.content.trim().is_empty())
        .collect();
    chapters.sort_by_key(|c| c.chapter_no);

    // 开头 5 章 + 最近 40 章，去重后按章号排序
    let mut pick: Vec<usize> = Vec::new();
    for i in 0..5.min(chapters.len()) {
        pick.push(i);
    }
    let tail_start = chapters.len().saturating_sub(40);
    for i in tail_start..chapters.len() {
        pick.push(i);
    }
    pick.sort_unstable();
    pick.dedup();

    for i in pick {
        let content = chapters[i].content.trim();
        if content.is_empty() {
            continue;
        }
        if total_chars >= MAX_SAMPLED_CHARS {
            break;
        }
        let remain = MAX_SAMPLED_CHARS - total_chars;
        if content.chars().count() > remain {
            // 截断到上限字符
            let cut: String = content.chars().take(remain).collect();
            sampled.push(cut);
            break;
        }
        sampled.push(content.to_string());
        total_chars += content.chars().count();
    }
    sampled.truncate(MAX_SAMPLED_CHAPTERS.min(sampled.len()));

    let mut stats = TextStats::default();
    for text in &sampled {
        accumulate(text, &mut stats);
    }
    stats_to_fingerprint(sampled.len(), &stats)
}

/// 累加单章统计
fn accumulate(content: &str, s: &mut TextStats) {
    let chars: Vec<char> = content.chars().collect();
    s.total_chars += chars.len();
    for ch in &chars {
        s.unique_chars.insert(*ch);
    }

    // 句长：按中文句末标点与换行切分
    for piece in content.split(|ch: char| "。！？…；\n".contains(ch)) {
        let len = piece.trim().chars().count();
        if len > 0 {
            s.sentence_lens.push(len);
        }
    }
    // 段落长：按换行切分
    for para in content.lines() {
        let len = para.trim().chars().count();
        if len > 0 {
            s.para_lens.push(len);
        }
    }
    // 连接词 / 标点计数
    for w in CONNECTORS {
        s.connectors += content.matches(w).count();
    }
    s.dashes += content.matches("——").count();
    s.colons += content.matches('：').count();
    s.quote_open_jp += content.matches('「').count();
    s.quote_open_cn += content.matches('“').count();

    // 对话占比：估算引号对内的字符数（「」或 “” 配对的累计长度）
    let mut in_quote = false;
    let mut seg_len = 0usize;
    for ch in &chars {
        match ch {
            '「' | '“' => {
                if !in_quote {
                    in_quote = true;
                    seg_len = 0;
                }
            }
            '」' | '”' => {
                if in_quote {
                    s.quoted_chars += seg_len;
                    in_quote = false;
                }
            }
            _ => {
                if in_quote {
                    seg_len += 1;
                }
            }
        }
    }
}

/// 汇总为指纹
fn stats_to_fingerprint(sampled_chapters: usize, s: &TextStats) -> StyleFingerprint {
    let (avg_sentence_length, sentence_var) = mean_var(&s.sentence_lens);
    let (avg_paragraph_length, para_var) = mean_var(&s.para_lens);
    let paragraph_uniformity = if avg_paragraph_length > 0.0 {
        para_var.sqrt() / avg_paragraph_length
    } else {
        0.0
    };
    let per_k = 1000.0 / s.total_chars.max(1) as f64;
    let quote_style = if s.quote_open_jp >= s.quote_open_cn && s.quote_open_jp > 0 {
        "「」".to_string()
    } else if s.quote_open_cn > 0 {
        "“”".to_string()
    } else {
        "无固定引号习惯".to_string()
    };
    StyleFingerprint {
        sampled_chapters,
        sampled_chars: s.total_chars,
        avg_sentence_length,
        sentence_var,
        avg_paragraph_length,
        paragraph_uniformity,
        connector_per_1k: s.connectors as f64 * per_k,
        dash_per_1k: s.dashes as f64 * per_k,
        colon_per_1k: s.colons as f64 * per_k,
        quote_style,
        dialogue_ratio: if s.total_chars > 0 {
            s.quoted_chars as f64 / s.total_chars as f64
        } else {
            0.0
        },
        vocabulary_richness: if s.total_chars > 0 {
            s.unique_chars.len() as f64 / s.total_chars as f64
        } else {
            0.0
        },
    }
}

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

/// 把指纹转成注入写作/审查 prompt 的文风基线块
pub fn fingerprint_block(fp: &StyleFingerprint) -> String {
    if fp.sampled_chapters == 0 || fp.sampled_chars == 0 {
        return String::new();
    }
    let rhythm_note = if fp.sentence_var < 5.0 && fp.avg_sentence_length > 0.0 {
        "（当前句长偏整齐，请主动长短交错）"
    } else {
        ""
    };
    format!(
        "【本书文风基线（统计自已有 {} 章）】\n\
         平均句长 {} 字，句长方差 {} {}；平均段落长 {} 字；\n\
         每千字连接词 {} 个；破折号每千字 {} 处；冒号每千字 {} 处；\n\
         引号习惯「{}」；对话占比 {}%。\n\
         写作与审查时保持与本书一致的句长、段落与标点习惯：\n\
         1. 不把句子改得比基线更整齐、更均匀（模板腔的最大来源）；\n\
         2. 不引入基线中不存在的标点习惯（如基线无破折号就尽量不用）；\n\
         3. 连接词密度不超过基线，能用短句断开就不用连接词粘合。",
        fp.sampled_chapters,
        format_1(fp.avg_sentence_length),
        format_1(fp.sentence_var),
        rhythm_note,
        format_1(fp.avg_paragraph_length),
        format_1(fp.connector_per_1k),
        format_1(fp.dash_per_1k),
        format_1(fp.colon_per_1k),
        fp.quote_style,
        format_1(fp.dialogue_ratio * 100.0),
    )
}

fn format_1(v: f64) -> String {
    format!("{v:.1}")
}

/// 读取缓存或重算（章节变更后缓存被置 None）
pub fn cached_or_compute(state: &AppState) -> StyleFingerprint {
    if let Some(fp) = &*state.style_fp.read() {
        return fp.clone();
    }
    let fp = compute_fingerprint(&state.ontology.read());
    *state.style_fp.write() = Some(fp.clone());
    fp
}

/// 查询本书文风指纹（墨韵页展示 / 手动刷新缓存）
#[tauri::command]
pub async fn get_style_fingerprint(
    state: tauri::State<'_, AppState>,
) -> Result<StyleFingerprint, String> {
    Ok(cached_or_compute(&state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate_stats() {
        let mut s = TextStats::default();
        accumulate("他往前走了一步。\n「你来了？」她问。", &mut s);
        assert!(s.sentence_lens.len() >= 3);
        assert_eq!(s.para_lens.len(), 2);
        assert!(s.quoted_chars >= 3); // 你来了？
        assert!(s.quote_open_jp >= 1);
    }

    #[test]
    fn test_fingerprint_block_empty_when_no_sample() {
        let fp = StyleFingerprint {
            sampled_chapters: 0,
            sampled_chars: 0,
            avg_sentence_length: 0.0,
            sentence_var: 0.0,
            avg_paragraph_length: 0.0,
            paragraph_uniformity: 0.0,
            connector_per_1k: 0.0,
            dash_per_1k: 0.0,
            colon_per_1k: 0.0,
            quote_style: String::new(),
            dialogue_ratio: 0.0,
            vocabulary_richness: 0.0,
        };
        assert!(fingerprint_block(&fp).is_empty());
    }

    #[test]
    fn test_fingerprint_block_non_empty() {
        let fp = compute_fingerprint_texts(vec![
            "他往前走了一步，没有说话。\n「你来了？」她问，声音很轻。",
            "夜色深了，街灯一盏一盏亮起来。",
        ]);
        assert!(!fingerprint_block(&fp).is_empty());
        assert!(fp.sampled_chapters >= 2);
        assert!(fp.avg_sentence_length > 0.0);
        assert!(fp.quote_style == "「」");
    }

    fn compute_fingerprint_texts(texts: Vec<&str>) -> StyleFingerprint {
        let mut s = TextStats::default();
        for t in &texts {
            accumulate(t, &mut s);
        }
        stats_to_fingerprint(texts.len(), &s)
    }
}
