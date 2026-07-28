/// 中文数字解析模块
use regex::Regex;

/// 解析中文字符串为数字
/// "零一二三四五六七八九十百千万" -> u64
/// 支持"二十三" = 23, "十一" = 11, "一百" = 100
pub fn parse_cn_number(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    
    // 先尝试解析阿拉伯数字
    if let Ok(num) = s.parse::<u64>() {
        return Some(num);
    }
    
    // 解析中文数字
    let mut result = 0u64;
    let mut current = 0u64;
    let mut has_unit = false;
    
    for ch in s.chars() {
        match ch {
            '零' => {
                // 零不影响位值，但需要标记
                if current == 0 && !has_unit {
                    // 零在开头或中间
                    continue;
                }
            }
            '一' => current = 1,
            '二' | '两' => current = 2,
            '三' => current = 3,
            '四' => current = 4,
            '五' => current = 5,
            '六' => current = 6,
            '七' => current = 7,
            '八' => current = 8,
            '九' => current = 9,
            '十' => {
                if current == 0 {
                    current = 1;
                }
                result += current * 10;
                current = 0;
                has_unit = true;
            }
            '百' => {
                result += current * 100;
                current = 0;
                has_unit = true;
            }
            '千' => {
                result += current * 1000;
                current = 0;
                has_unit = true;
            }
            '万' => {
                result += current * 10000;
                current = 0;
                has_unit = true;
            }
            _ => return None, // 非法字符
        }
    }
    
    // 加上最后的余数
    result += current;
    
    if result == 0 && s.contains('零') {
        Some(0)
    } else if result == 0 {
        None
    } else {
        Some(result)
    }
}

/// 从章节标题中提取章节号
/// 先尝试数字匹配，再尝试中文数字匹配
pub fn extract_chapter_number(title: &str) -> Option<u64> {
    // 先尝试阿拉伯数字匹配
    let arabic_regex = Regex::new(r"(\d+)").ok()?;
    if let Some(caps) = arabic_regex.captures(title)
        && let Some(num_str) = caps.get(1)
            && let Ok(num) = num_str.as_str().parse::<u64>() {
                return Some(num);
            }
    
    // 尝试中文数字匹配
    let cn_regex = Regex::new(r"([零一二三四五六七八九十百千万]+)").ok()?;
    if let Some(caps) = cn_regex.captures(title)
        && let Some(cn_str) = caps.get(1) {
            return parse_cn_number(cn_str.as_str());
        }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_cn_number_basic() {
        assert_eq!(parse_cn_number("零"), Some(0));
        assert_eq!(parse_cn_number("一"), Some(1));
        assert_eq!(parse_cn_number("二"), Some(2));
        assert_eq!(parse_cn_number("三"), Some(3));
        assert_eq!(parse_cn_number("四"), Some(4));
        assert_eq!(parse_cn_number("五"), Some(5));
        assert_eq!(parse_cn_number("六"), Some(6));
        assert_eq!(parse_cn_number("七"), Some(7));
        assert_eq!(parse_cn_number("八"), Some(8));
        assert_eq!(parse_cn_number("九"), Some(9));
        assert_eq!(parse_cn_number("十"), Some(10));
    }
    
    #[test]
    fn test_parse_cn_number_compound() {
        assert_eq!(parse_cn_number("十一"), Some(11));
        assert_eq!(parse_cn_number("十二"), Some(12));
        assert_eq!(parse_cn_number("二十"), Some(20));
        assert_eq!(parse_cn_number("二十三"), Some(23));
        assert_eq!(parse_cn_number("一百"), Some(100));
        assert_eq!(parse_cn_number("一百二十三"), Some(123));
        assert_eq!(parse_cn_number("一千"), Some(1000));
        assert_eq!(parse_cn_number("一万"), Some(10000));
    }
    
    #[test]
    fn test_parse_cn_number_with_zero() {
        assert_eq!(parse_cn_number("一百零一"), Some(101));
        assert_eq!(parse_cn_number("一千零一"), Some(1001));
        assert_eq!(parse_cn_number("一万零一"), Some(10001));
    }
    
    #[test]
    fn test_parse_cn_number_arabic() {
        assert_eq!(parse_cn_number("0"), Some(0));
        assert_eq!(parse_cn_number("1"), Some(1));
        assert_eq!(parse_cn_number("23"), Some(23));
        assert_eq!(parse_cn_number("123"), Some(123));
    }
    
    #[test]
    fn test_parse_cn_number_invalid() {
        assert_eq!(parse_cn_number(""), None);
        assert_eq!(parse_cn_number("abc"), None);
        assert_eq!(parse_cn_number("一二abc"), None);
    }
    
    #[test]
    fn test_extract_chapter_number_arabic() {
        assert_eq!(extract_chapter_number("第1章"), Some(1));
        assert_eq!(extract_chapter_number("第10章"), Some(10));
        assert_eq!(extract_chapter_number("Chapter 5"), Some(5));
        assert_eq!(extract_chapter_number("1. 标题"), Some(1));
    }
    
    #[test]
    fn test_extract_chapter_number_cn() {
        assert_eq!(extract_chapter_number("第一章"), Some(1));
        assert_eq!(extract_chapter_number("第十一章"), Some(11));
        assert_eq!(extract_chapter_number("第二十三章"), Some(23));
        assert_eq!(extract_chapter_number("第一百章"), Some(100));
    }
    
    #[test]
    fn test_extract_chapter_number_mixed() {
        assert_eq!(extract_chapter_number("第1章 标题"), Some(1));
        assert_eq!(extract_chapter_number("【第10章】"), Some(10));
        assert_eq!(extract_chapter_number("[第一章]"), Some(1));
    }
    
    #[test]
    fn test_extract_chapter_number_none() {
        assert_eq!(extract_chapter_number("没有数字的标题"), None);
        assert_eq!(extract_chapter_number(""), None);
    }
}