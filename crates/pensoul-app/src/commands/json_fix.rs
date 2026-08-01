//! LLM 输出 JSON 的容错修复
//!
//! 大模型输出的 JSON 常见瑕疵：漏逗号、尾逗号、代码围栏、
//! 字符串内裸换行、输出被 max_tokens 截断、注释等。
//! 本模块先做一次预处理，再依据 serde_json 报错位置驱动式修复，
//! 直到解析成功或无法继续修复为止。

use serde_json::Value;

/// 最大修复尝试次数，防止修复循环不收敛
const MAX_FIX_ROUNDS: usize = 64;

/// 把可能带瑕疵的 LLM JSON 文本修复为 `serde_json::Value`
///
/// 失败时返回最后一次 serde 报错信息。
pub(crate) fn repair_to_value(raw: &str) -> Result<Value, String> {
    let mut text = preprocess(raw);
    for _ in 0..MAX_FIX_ROUNDS {
        match serde_json::from_str::<Value>(&text) {
            Ok(v) => return Ok(v),
            Err(e) => {
                let offset = line_col_to_offset(&text, e.line(), e.column());
                if !apply_fix(&mut text, offset, &e.to_string()) {
                    return Err(e.to_string());
                }
            }
        }
    }
    Err("JSON 修复尝试次数超限".to_string())
}

/// 预处理：剥代码围栏、截取最外层 JSON 结构、去注释、转义字符串内控制字符
fn preprocess(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 截取最外层 { ... } 或 [ ... ]，去掉模型多余的前后赘言
    let start = trimmed
        .char_indices()
        .find(|(_, c)| *c == '{' || *c == '[')
        .map(|(i, _)| i);
    let end = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| *c == '}' || *c == ']')
        .map(|(i, c)| i + c.len_utf8());
    let body = match (start, end) {
        (Some(s), Some(e)) if e > s => &trimmed[s..e],
        _ => trimmed,
    };

    escape_controls_and_strip_comments(body)
}

/// 单次扫描：去掉字符串外的 // 与 /* */ 注释，
/// 并把字符串内的裸控制字符（换行、制表等）转义为合法形式
fn escape_controls_and_strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
                out.push(c);
                continue;
            }
            match c {
                '\\' => {
                    escaped = true;
                    out.push(c);
                }
                '"' => {
                    in_string = false;
                    out.push(c);
                }
                '\n' => out.push_str("\\n"),
                '\r' => {}
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push(' '),
                c => out.push(c),
            }
        } else {
            match c {
                '"' => {
                    in_string = true;
                    out.push(c);
                }
                '/' if chars.peek() == Some(&'/') => {
                    // 行注释：吞到行尾
                    for c2 in chars.by_ref() {
                        if c2 == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    // 块注释：吞到 */
                    let _ = chars.next();
                    let mut prev = '\0';
                    for c2 in chars.by_ref() {
                        if prev == '*' && c2 == '/' {
                            break;
                        }
                        prev = c2;
                    }
                }
                c => out.push(c),
            }
        }
    }
    out
}

/// 把 serde_json 的 1-based 行/列换算为字节偏移（列按字节计）
fn line_col_to_offset(text: &str, line: usize, col: usize) -> usize {
    let mut offset = 0;
    for (n, l) in text.split('\n').enumerate() {
        if n + 1 == line {
            return offset + col.saturating_sub(1).min(l.len());
        }
        offset += l.len() + 1;
    }
    text.len()
}

/// 依据 serde 报错信息在指定位置做一次修复，返回是否做了修改
fn apply_fix(text: &mut String, offset: usize, msg: &str) -> bool {
    let offset = offset.min(text.len());

    if msg.contains("expected `,` or `}`") || msg.contains("expected `,` or `]`") {
        // 值之间漏了逗号：在报错 token 前补一个逗号
        text.insert(offset, ',');
        return true;
    }
    if msg.contains("trailing comma") || msg.contains("expected value") {
        // 尾逗号或多余逗号：向前找最近的逗号删掉
        if let Some(pos) = text[..offset].rfind(',') {
            text.remove(pos);
            return true;
        }
        return false;
    }
    if msg.contains("EOF while parsing") {
        // 输出被截断：补全未闭合的字符串与括号
        close_truncated(text);
        return true;
    }
    if msg.contains("trailing characters") {
        // JSON 后有多余内容：截断
        text.truncate(offset);
        return true;
    }
    false
}

/// 扫描括号栈（忽略字符串），补全截断的 JSON：
/// 字符串未闭合先补引号，再按栈逆序补 ] 和 }
fn close_truncated(text: &mut String) {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' | '[' => stack.push(c),
            '}' => {
                if stack.last() == Some(&'{') {
                    stack.pop();
                }
            }
            ']' if stack.last() == Some(&'[') => {
                stack.pop();
            }
            _ => {}
        }
    }
    if escaped {
        text.pop(); // 末尾孤立的转义反斜杠
    }
    if in_string {
        text.push('"');
    }
    while let Some(open) = stack.pop() {
        text.push(if open == '{' { '}' } else { ']' });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repair_str(raw: &str) -> String {
        repair_to_value(raw)
            .map(|v| serde_json::to_string(&v).unwrap())
            .unwrap_or_else(|e| panic!("修复失败: {e}"))
    }

    #[test]
    fn test_missing_comma_between_fields() {
        // 用户实际遇到的场景：summary 后漏逗号（expected `,` or `}` at line 3 column 3）
        let raw = "{\n  \"summary\": \"讨论结论\"\n  \"locations\": []\n}";
        let v = repair_to_value(raw).expect("应能修复漏逗号");
        assert_eq!(v["summary"], "讨论结论");
        assert!(v["locations"].is_array());
    }

    #[test]
    fn test_trailing_comma_removed() {
        let raw = "{\"a\": 1, \"b\": [1, 2,],}";
        let v = repair_to_value(raw).expect("应能去掉尾逗号");
        assert_eq!(v["a"], 1);
        assert_eq!(v["b"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_code_fence_and_prose_stripped() {
        let raw = "好的，以下是成果：\n```json\n{\"summary\": \"x\"}\n```\n希望对你有帮助。";
        assert_eq!(repair_str(raw), "{\"summary\":\"x\"}");
    }

    #[test]
    fn test_raw_newline_in_string_escaped() {
        let raw = "{\"summary\": \"第一行\n第二行\"}";
        let v = repair_to_value(raw).expect("应能转义字符串内裸换行");
        assert_eq!(v["summary"], "第一行\n第二行");
    }

    #[test]
    fn test_comments_stripped() {
        let raw = "{\n // 总结\n \"summary\": \"x\", /* 地点 */ \"locations\": [] }";
        let v = repair_to_value(raw).expect("应能去掉注释");
        assert_eq!(v["summary"], "x");
    }

    #[test]
    fn test_truncated_output_closed() {
        let raw = "{\"summary\": \"x\", \"locations\": [{\"name\": \"城堡\"";
        let v = repair_to_value(raw).expect("应能补全截断的括号");
        assert_eq!(v["locations"][0]["name"], "城堡");
    }

    #[test]
    fn test_truncated_inside_string() {
        let raw = "{\"summary\": \"被截断的结";
        let v = repair_to_value(raw).expect("应能补全未闭合字符串");
        assert_eq!(v["summary"], "被截断的结");
    }

    #[test]
    fn test_unrepairable_returns_error() {
        assert!(repair_to_value("这不是 JSON").is_err());
    }
}
