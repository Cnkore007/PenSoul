// llm/mod.rs — LLM 适配层

pub mod client;
pub mod config;
pub mod remote;

pub use client::{LlmClient, LlmMessage, LlmRequest, LlmResponse, TokenUsage};
pub use config::{LlmConfig, LlmConfigStore, Provider, ProviderConfig, ThinkingMode};

/// 解析 LLM 结构化输出为 JSON。
///
/// 防御式设计：LLM 常用 Markdown 代码块（```json ... ```）或前后说明文字包裹输出，
/// 直接 `serde_json::from_str` 会失败（2026-08-13 实测 GLM 中转返回 ```json 包裹导致
/// 蒸馏/事实提取等所有结构化功能从未成功过）。依次尝试：
/// 1. 原文直接解析；
/// 2. 剥离 ``` 代码块围栏后解析；
/// 3. 截取首个 `{` 到末个 `}` 之间的主体（容忍前后说明文字）。
/// 全部失败返回带原文前缀的错误，供诊断。
pub fn parse_llm_json<T: serde::de::DeserializeOwned>(content: &str) -> Result<T, String> {
    let trimmed = content.trim();

    let mut candidates: Vec<&str> = vec![trimmed];
    // 候选 2：剥离 Markdown 代码块围栏（```json ... ```）。
    // 宽容处理：先按字符剥掉首尾反引号与语言标识，再定位 `{`..`}` 主体，
    // 容忍围栏后残留换行/空白导致的 trim_end_matches 剥不净问题。
    if let Some(inner) = trimmed.strip_prefix("```") {
        let after_lang = inner.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-');
        if let (Some(start), Some(end)) = (after_lang.find('{'), after_lang.rfind('}')) {
            if end > start {
                candidates.push(&after_lang[start..=end]);
            }
        }
    }
    // 候选 3：直接取首个 { 到末个 }（容忍前后说明文字；嵌套对象不受影响）
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            candidates.push(&trimmed[start..=end]);
        }
    }

    for candidate in candidates {
        if let Ok(value) = serde_json::from_str::<T>(candidate) {
            return Ok(value);
        }
    }

    Err(format!(
        "LLM 输出不是合法 JSON；原始内容前 300 字: {}",
        trimmed.chars().take(300).collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_llm_json;

    #[test]
    fn parses_plain_json() {
        let v: serde_json::Value = parse_llm_json(r#"{"a": 1}"#).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn parses_markdown_fenced_json() {
        // 2026-08-13 实测：GLM 中转以 ```json 代码块包裹输出
        let raw = "```json\n{\"dimensions\": [{\"dimension\": \"词汇\", \"features\": [\"a\"]}]}\n```";
        let v: serde_json::Value = parse_llm_json(raw).unwrap();
        assert_eq!(v["dimensions"][0]["dimension"], "词汇");
    }

    #[test]
    fn parses_fenced_with_leading_text() {
        let raw = "好的，分析如下：\n```json\n{\"genes\": [\"短句收尾\"]}\n```\n希望对你有帮助";
        let v: serde_json::Value = parse_llm_json(raw).unwrap();
        assert_eq!(v["genes"][0], "短句收尾");
    }

    #[test]
    fn rejects_invalid_output_with_preview() {
        let err = parse_llm_json::<serde_json::Value>("完全不是 JSON").unwrap_err();
        assert!(err.contains("不是合法 JSON"), "错误应说明原因: {err}");
        assert!(err.contains("完全不是 JSON"), "错误应带原文预览: {err}");
    }
}
