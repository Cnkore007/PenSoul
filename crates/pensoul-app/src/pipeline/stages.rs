//! 管线三阶段模板与输出解析器。
//!
//! 阶段模板 P0 用 Rust 硬编码（写作 → 审查 → 回灌），
//! P1 才外置为 YAML 插件（PluginStage → Stage 转换器）。
//! 解析器负责把 LLM 文本产出拆成「信号（给引擎判门控）」
//! 与「报告（给用户看）」双通道。
use pensoul_core::StageName;
use pensoul_harness::{GateType, RunnerType, Stage};

/// 写作阶段
pub const STAGE_WRITING: &str = "chapter_writing";
/// 审查阶段
pub const STAGE_REVIEW: &str = "chapter_review";
/// 回灌阶段
pub const STAGE_INJECTION: &str = "state_injection";

/// 三阶段模板：写作(auto) → 审查(conditional, 异模型) → 回灌(auto)
pub fn pipeline_stages() -> Vec<Stage> {
    vec![
        Stage {
            name: StageName::new(STAGE_WRITING),
            display_name: "章节写作".to_string(),
            manual: "根据备忘录、本章梗概、世界观、人物与记忆包撰写正文，只输出正文。".to_string(),
            tools_allowed: vec![
                "read_memo".into(),
                "read_chapter_outline".into(),
                "read_world_settings".into(),
                "read_character_state".into(),
                "read_memory_packet".into(),
                "read_creation_settings".into(),
                "read_prev_feedback".into(),
            ],
            tools_denied: vec!["write_settings".into(), "write_outline".into()],
            gate_type: GateType::Auto,
            next_stage: Some(StageName::new(STAGE_REVIEW)),
            runner: RunnerType::Local,
            ..Default::default()
        },
        Stage {
            name: StageName::new(STAGE_REVIEW),
            display_name: "一致性审查".to_string(),
            manual: "用不同模型审查本章与设定/人物/前文的一致性，输出 score 与 issues。"
                .to_string(),
            tools_allowed: vec![
                "read_memo".into(),
                "read_world_settings".into(),
                "read_character_state".into(),
            ],
            tools_denied: vec!["write_settings".into(), "write_chapter".into()],
            gate_type: GateType::Conditional,
            // 默认 result.score >= 80 放行，不写表达式
            gate_condition: None,
            next_stage: Some(StageName::new(STAGE_INJECTION)),
            on_fail: Some(StageName::new(STAGE_WRITING)),
            runner: RunnerType::Delegated,
            max_retries: 2,
            ..Default::default()
        },
        Stage {
            name: StageName::new(STAGE_INJECTION),
            display_name: "状态回灌".to_string(),
            manual: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
            tools_allowed: vec!["read_memo".into(), "write_memo".into()],
            tools_denied: vec!["write_settings".into()],
            gate_type: GateType::Auto,
            next_stage: None,
            runner: RunnerType::Local,
            ..Default::default()
        },
    ]
}

// ── 输出解析 ────────────────────────────────────────────────────────────

const SIGNAL_BEGIN: &str = "===SIGNAL_BEGIN===";
const SIGNAL_END: &str = "===SIGNAL_END===";
const REPORT_BEGIN: &str = "===REPORT_BEGIN===";
const REPORT_END: &str = "===REPORT_END===";

/// 审查信号（SIGNAL 通道）：门控分数 + 问题清单
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewSignal {
    pub score: f64,
    pub issues: Vec<String>,
}

/// 截取两个标记之间的文本
fn between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let b = text.find(begin)? + begin.len();
    let e = text[b..].find(end)? + b;
    Some(text[b..e].trim())
}

/// 解析写作阶段输出：剥掉 Markdown 代码围栏，返回纯正文。
///
/// 模型偶尔会用 ``` 包裹全文或加前导语，这里取最长连续文本块。
pub fn parse_writing_output(text: &str) -> String {
    let trimmed = text.trim();
    // 整体被代码围栏包裹时剥掉围栏
    if trimmed.starts_with("```") {
        let body = trimmed
            .trim_start_matches('`')
            .trim_start_matches(|c: char| c.is_alphabetic()) // 剥掉语言标记如 ```text
            .trim_start();
        if let Some(end) = body.rfind("```") {
            return body[..end].trim().to_string();
        }
        return body.trim().to_string();
    }
    trimmed.to_string()
}

/// 解析审查阶段输出：SIGNAL JSON + REPORT 文本。
///
/// SIGNAL 缺失或 JSON 无法解析 / 缺 score 时返回 Err（视为执行失败，触发重试）。
pub fn parse_review_output(text: &str) -> Result<(ReviewSignal, String), String> {
    let signal_text = between(text, SIGNAL_BEGIN, SIGNAL_END)
        .ok_or_else(|| "审查输出缺少 SIGNAL 标记块".to_string())?;
    let json: serde_json::Value =
        serde_json::from_str(signal_text).map_err(|e| format!("审查 SIGNAL JSON 解析失败: {e}"))?;
    let score = json
        .get("score")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "审查 SIGNAL 缺少 score 数值".to_string())?;
    let issues = json
        .get("issues")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| {
                    if let Some(s) = item.as_str() {
                        s.to_string()
                    } else {
                        // 对象形态：{severity, description, suggestion} 拍平成一行
                        let desc = item
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        let sug = item
                            .get("suggestion")
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        if sug.is_empty() {
                            desc.to_string()
                        } else {
                            format!("{desc}（建议：{sug}）")
                        }
                    }
                })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let report = between(text, REPORT_BEGIN, REPORT_END)
        .map(str::to_string)
        .unwrap_or_else(|| format!("一致性评分 {score:.0}"));
    Ok((ReviewSignal { score, issues }, report))
}

/// 解析回灌阶段输出：提取 JSON 中的 chapter_brief；失败用正文前 150 字兜底。
pub fn parse_injection_output(text: &str, fallback_content: &str) -> String {
    // 优先找 JSON 块（可能被 ```json 包裹）
    let candidate = if let Some(b) = text.find('{') {
        let e = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
        &text[b..e]
    } else {
        text
    };
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(candidate)
        && let Some(brief) = json.get("chapter_brief").and_then(|v| v.as_str())
        && !brief.trim().is_empty()
    {
        return brief.trim().to_string();
    }
    // 兜底：截正文前 150 字
    let fallback: String = fallback_content.chars().take(150).collect();
    format!("（纪要提炼失败，正文节选）{fallback}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stages_topology() {
        let stages = pipeline_stages();
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0].name.as_str(), STAGE_WRITING);
        assert_eq!(stages[0].gate_type, GateType::Auto);
        assert_eq!(
            stages[0].next_stage.as_ref().map(|s| s.as_str()),
            Some(STAGE_REVIEW)
        );
        assert_eq!(stages[1].gate_type, GateType::Conditional);
        assert_eq!(
            stages[1].on_fail.as_ref().map(|s| s.as_str()),
            Some(STAGE_WRITING)
        );
        assert_eq!(stages[1].max_retries, 2);
        assert!(stages[2].next_stage.is_none());
    }

    #[test]
    fn test_parse_writing_strips_code_fence() {
        let fenced = "```\n第一章的正文内容。\n第二段。\n```";
        assert_eq!(parse_writing_output(fenced), "第一章的正文内容。\n第二段。");
        let plain = "直接的正文";
        assert_eq!(parse_writing_output(plain), "直接的正文");
    }

    #[test]
    fn test_parse_review_output_ok() {
        let text = r#"前导说明
===SIGNAL_BEGIN===
{"score": 85, "issues": [{"description": "第三章时间线矛盾", "suggestion": "改为次日"}]}
===SIGNAL_END===
===REPORT_BEGIN===
本章评分 85，发现 1 处时间线问题。
===REPORT_END==="#;
        let (signal, report) = parse_review_output(text).unwrap();
        assert_eq!(signal.score, 85.0);
        assert_eq!(signal.issues, vec!["第三章时间线矛盾（建议：改为次日）"]);
        assert!(report.contains("85"));
    }

    #[test]
    fn test_parse_review_output_string_issues() {
        let text = "===SIGNAL_BEGIN===\n{\"score\": 60, \"issues\": [\"节奏拖沓\"]}\n===SIGNAL_END===\n===REPORT_BEGIN===\n报告\n===REPORT_END===";
        let (signal, _) = parse_review_output(text).unwrap();
        assert_eq!(signal.score, 60.0);
        assert_eq!(signal.issues, vec!["节奏拖沓"]);
    }

    #[test]
    fn test_parse_review_output_missing_signal_is_error() {
        assert!(parse_review_output("没有任何标记的输出").is_err());
        let no_score = "===SIGNAL_BEGIN===\n{\"issues\": []}\n===SIGNAL_END===\n===REPORT_BEGIN===\nr\n===REPORT_END===";
        assert!(parse_review_output(no_score).is_err());
    }

    #[test]
    fn test_parse_injection_output_json_and_fallback() {
        let json_text = "提炼结果：\n```json\n{\"chapter_brief\": \"林晚与沈舟决裂\"}\n```";
        assert_eq!(parse_injection_output(json_text, "正文"), "林晚与沈舟决裂");
        // 非 JSON 输出 → 正文前 150 字兜底
        let fallback = parse_injection_output("这不是 JSON", "正文内容 abc");
        assert!(fallback.contains("正文内容"));
    }
}
