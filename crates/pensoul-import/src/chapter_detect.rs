use crate::cn_number::extract_chapter_number;
/// 章节检测模块
use regex::Regex;

/// 检测到的章节信息
#[derive(Debug, Clone)]
pub struct DetectedChapter {
    pub chapter_number: Option<u64>,
    pub title: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub word_count: usize,
    pub confidence: f64,
}

/// 章节检测器
pub struct ChapterDetector {
    patterns: Vec<Regex>,
}

impl ChapterDetector {
    /// 创建新的章节检测器
    pub fn new() -> Self {
        let patterns = vec![
            Regex::new(r"第(\d+)章").unwrap(),
            Regex::new(r"第([零一二三四五六七八九十百千万]+)章").unwrap(),
            Regex::new(r"Chapter\s*(\d+)").unwrap(),
            Regex::new(r"^(\d+)\.\s").unwrap(),
            Regex::new(r"[【\[\(]第(\d+)章[】\]\)]").unwrap(),
        ];
        Self { patterns }
    }

    /// 检测文本中的章节
    pub fn detect(&self, text: &str, min_words: usize) -> Vec<DetectedChapter> {
        let lines: Vec<&str> = text.lines().collect();
        let mut chapters = Vec::new();
        let mut current_chapter_start = 0;
        let mut current_chapter_number: Option<u64> = None;
        let mut current_chapter_title = String::new();

        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // 检查是否匹配任何章节模式
            let mut matched = false;
            let mut chapter_number = None;
            let mut title = String::new();

            for pattern in &self.patterns {
                if let Some(caps) = pattern.captures(line) {
                    matched = true;
                    // 提取章节号
                    if let Some(num_str) = caps.get(1) {
                        chapter_number = extract_chapter_number(num_str.as_str());
                    }
                    title = line.to_string();
                    break;
                }
            }

            if matched {
                // 如果之前有章节，保存它
                if current_chapter_start < i {
                    let content_lines = &lines[current_chapter_start..i];
                    let content = content_lines.join("\n");
                    let word_count = Self::word_count(&content);

                    if word_count >= min_words {
                        chapters.push(DetectedChapter {
                            chapter_number: current_chapter_number,
                            title: current_chapter_title,
                            content,
                            start_line: current_chapter_start,
                            end_line: i,
                            word_count,
                            confidence: 0.5,
                        });
                    }
                }

                // 开始新章节
                current_chapter_start = i + 1;
                current_chapter_number = chapter_number;
                current_chapter_title = title;
            }
        }

        // 处理最后一个章节
        if current_chapter_start < lines.len() {
            let content_lines = &lines[current_chapter_start..];
            let content = content_lines.join("\n");
            let word_count = Self::word_count(&content);

            if word_count >= min_words {
                chapters.push(DetectedChapter {
                    chapter_number: current_chapter_number,
                    title: current_chapter_title,
                    content,
                    start_line: current_chapter_start,
                    end_line: lines.len(),
                    word_count,
                    confidence: 0.5,
                });
            }
        }

        // 如果没有检测到章节，返回整篇文本作为一章
        if chapters.is_empty() {
            let word_count = Self::word_count(text);
            if word_count >= min_words {
                chapters.push(DetectedChapter {
                    chapter_number: None,
                    title: String::new(),
                    content: text.to_string(),
                    start_line: 0,
                    end_line: lines.len(),
                    word_count,
                    confidence: 0.5,
                });
            }
        }

        // 估算置信度
        self.estimate_confidence(&mut chapters);

        chapters
    }

    /// 估算置信度
    /// 连续章节号 confidence+0.2
    pub fn estimate_confidence(&self, chapters: &mut [DetectedChapter]) {
        if chapters.is_empty() {
            return;
        }

        // 按章节号排序（如果有的话）
        let mut indexed_chapters: Vec<(usize, Option<u64>)> = chapters
            .iter()
            .enumerate()
            .map(|(i, ch)| (i, ch.chapter_number))
            .collect();

        indexed_chapters.sort_by_key(|a| a.1.unwrap_or(0));

        // 检查连续性
        for window in indexed_chapters.windows(2) {
            if let (Some(prev_idx), Some(curr_idx)) = (window[0].1, window[1].1)
                && curr_idx == prev_idx + 1
            {
                // 连续章节号，提升置信度
                if let Some(ch) = chapters.get_mut(window[1].0) {
                    ch.confidence += 0.2;
                }
            }
        }
    }

    /// 统计字数（去除换行和空格后的字符数）
    pub fn word_count(content: &str) -> usize {
        content.chars().filter(|c| !c.is_whitespace()).count()
    }
}

impl Default for ChapterDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chapter_detector_new() {
        let detector = ChapterDetector::new();
        assert_eq!(detector.patterns.len(), 5);
    }

    #[test]
    fn test_detect_arabic_chapters() {
        let detector = ChapterDetector::new();
        let text = "第1章 标题一\n内容一\n第2章 标题二\n内容二";
        let chapters = detector.detect(text, 1);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].chapter_number, Some(1));
        assert_eq!(chapters[1].chapter_number, Some(2));
    }

    #[test]
    fn test_detect_cn_chapters() {
        let detector = ChapterDetector::new();
        let text = "第一章 标题一\n内容一\n第二章 标题二\n内容二";
        let chapters = detector.detect(text, 1);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].chapter_number, Some(1));
        assert_eq!(chapters[1].chapter_number, Some(2));
    }

    #[test]
    fn test_detect_mixed_chapters() {
        let detector = ChapterDetector::new();
        let text = "第1章 标题一\n内容一\n第十一章 标题二\n内容二";
        let chapters = detector.detect(text, 1);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].chapter_number, Some(1));
        assert_eq!(chapters[1].chapter_number, Some(11));
    }

    #[test]
    fn test_detect_no_chapters() {
        let detector = ChapterDetector::new();
        let text = "没有章节标记的文本";
        let chapters = detector.detect(text, 1);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].chapter_number, None);
    }

    #[test]
    fn test_confidence_boost() {
        let detector = ChapterDetector::new();
        let text = "第1章 标题一\n内容一\n第2章 标题二\n内容二\n第3章 标题三\n内容三";
        let mut chapters = detector.detect(text, 1);
        detector.estimate_confidence(&mut chapters);

        // 第2章和第3章应该有置信度提升
        assert!(chapters[1].confidence > 0.5);
        assert!(chapters[2].confidence > 0.5);
    }

    #[test]
    fn test_word_count() {
        assert_eq!(ChapterDetector::word_count("你好世界"), 4);
        assert_eq!(ChapterDetector::word_count("hello world"), 10);
        assert_eq!(ChapterDetector::word_count("  hello  world  "), 10);
        assert_eq!(ChapterDetector::word_count("\nhello\nworld\n"), 10);
    }

    #[test]
    fn test_min_words_filter() {
        let detector = ChapterDetector::new();
        let text = "第1章 标题\n短内容";
        let chapters = detector.detect(text, 10);
        assert_eq!(chapters.len(), 0); // 内容太短，被过滤

        let text = "第1章 标题\n这是一个足够长的内容来通过最小字数过滤器的测试文本";
        let chapters = detector.detect(text, 10);
        assert_eq!(chapters.len(), 1);
    }
}
