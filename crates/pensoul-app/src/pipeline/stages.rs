//! 管线三阶段模板与输出解析器。
//!
//! 阶段模板 P0 用 Rust 硬编码（写作 → 审查 → 回灌），
//! P1 由声明式工作流模板（模板库）覆盖阶段手册/门控/重试。
//! 解析器负责把 LLM 文本产出拆成「信号（给引擎判门控）」
//! 与「报告（给用户看）」双通道。
use pensoul_core::StageName;
use pensoul_core::workflow::WorkflowTemplate;
use pensoul_harness::{GateType, RunnerType, Stage};

/// 写作阶段
pub const STAGE_WRITING: &str = "chapter_writing";
/// 审查阶段
pub const STAGE_REVIEW: &str = "chapter_review";
/// 回灌阶段
pub const STAGE_INJECTION: &str = "state_injection";
/// 章前策划阶段（模板声明该阶段时插入到写作之前）
pub const STAGE_PLANNING: &str = "chapter_planning";

/// 阶段模板：写作(auto) → 审查(conditional, 异模型) → 回灌(auto)；
/// 若模板声明了「章前策划」环节（默认三阶段之外的第四阶段），
/// 则编排为 策划 → 写作 → 审查 → 回灌。
///
/// 传入全局工作流模板时可覆盖阶段显示名/手册/门控/重试/审查阈值；
/// 模板缺省（None）时使用默认值。
pub fn pipeline_stages(template: Option<&WorkflowTemplate>) -> Vec<Stage> {
    let mut stages: Vec<Stage> = Vec::new();

    // 章前策划：仅在模板显式声明该环节时启用（webnovel v2 内置）
    let with_planning = template
        .and_then(|t| t.find_stage(STAGE_PLANNING))
        .map(|def| def.enabled)
        .unwrap_or(false);
    if with_planning {
        stages.push(Stage {
            name: StageName::new(STAGE_PLANNING),
            display_name: "章前策划".to_string(),
            manual: "写前策划：结合本章梗概、前章纪要、世界观与人物状态，产出一张可执行的节拍表。"
                .to_string(),
            tools_allowed: vec![
                "read_memo".into(),
                "read_chapter_outline".into(),
                "read_world_settings".into(),
                "read_character_state".into(),
                "read_memory_packet".into(),
                "read_creation_settings".into(),
            ],
            tools_denied: vec!["write_settings".into(), "write_outline".into()],
            gate_type: GateType::Auto,
            next_stage: Some(StageName::new(STAGE_WRITING)),
            runner: RunnerType::Local,
            ..Default::default()
        });
    }

    stages.extend(vec![
        Stage {
            name: StageName::new(STAGE_WRITING),
            display_name: "章节写作".to_string(),
            manual: "根据备忘录、本章梗概、节拍表、世界观、人物与记忆包撰写正文，只输出正文。"
                .to_string(),
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
            manual: "用不同模型按七维加权审查本章：卖点兑现/开场钩子/情绪曲线/场景节奏/断章钩子/人物与设定一致性/文笔反 AI 味，输出分数与问题清单。"
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
    ]);

    if let Some(tpl) = template {
        for st in &mut stages {
            let Some(def) = tpl.find_stage(st.name.as_str()) else {
                continue;
            };
            if !def.display_name.trim().is_empty() {
                st.display_name = def.display_name.clone();
            }
            if !def.prompt_hint.trim().is_empty() {
                st.manual = def.prompt_hint.clone();
            }
            st.gate_type = match def.gate.as_str() {
                "manual" => GateType::Manual,
                "conditional" => GateType::Conditional,
                _ => GateType::Auto,
            };
            st.max_retries = def.max_retries;
            if let Some(on_fail) = def.on_fail.as_deref() {
                if !on_fail.is_empty() {
                    st.on_fail = Some(StageName::new(on_fail));
                }
            }
            // 条件门控：用模板阈值生成门控表达式（引擎优先按表达式判定）
            if def.gate == "conditional" {
                st.gate_condition = Some(format!("score >= {}", tpl.review_pass_score));
            }
        }
    }
    stages
}

// ── 输出解析 ────────────────────────────────────────────────────────────

const SIGNAL_BEGIN: &str = "===SIGNAL_BEGIN===";
const SIGNAL_END: &str = "===SIGNAL_END===";
const REPORT_BEGIN: &str = "===REPORT_BEGIN===";
const REPORT_END: &str = "===REPORT_END===";
/// 写作阶段双通道标记：模型必须把正文包裹在这两个标记之间，
/// 标记外不得输出任何内容（防英文规划/思考/场景说明混入正文）。
pub const CHAPTER_BEGIN: &str = "===CHAPTER_BEGIN===";
pub const CHAPTER_END: &str = "===CHAPTER_END===";

/// 审查信号（SIGNAL 通道）：门控分数 + 问题清单
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewSignal {
    pub score: f64,
    pub issues: Vec<String>,
    /// 诊断报告（四字段：问题族/触发点/建议动作/是否建议改写），
    /// 供前端展示与写手阶段按诊断修正
    pub diagnosis: Vec<DiagnosisItem>,
    /// 黄金三章门控子分数：开场钩子（0-10，仅前 3 章审查输出）
    pub hook_score: Option<f64>,
    /// 黄金三章门控子分数：爽点/情绪释放（0-10，仅前 3 章审查输出）
    pub payoff_score: Option<f64>,
}

/// 审查诊断条目 —— 对应「去 AI 味」评审的 annotation mode 四字段
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DiagnosisItem {
    /// 问题族：如 结构骨架 / 翻译腔 / 人物一致性 / 断章钩子
    pub family: String,
    /// 触发点：命中的词、结构或局部句子
    pub trigger: String,
    /// 建议动作：删掉 / 换成具体表达 / 补充信息 / 保持不动
    pub action: String,
    /// 是否建议改写
    pub rewrite: bool,
}

/// 截取两个标记之间的文本
fn between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let b = text.find(begin)? + begin.len();
    let e = text[b..].find(end)? + b;
    Some(text[b..e].trim())
}

/// 解析写作阶段输出：优先取 CHAPTER 标记内的正文；无标记时剥掉
/// Markdown 代码围栏与前导规划文本（模型常把英文/中文规划混在正文前）。
pub fn parse_writing_output(text: &str) -> String {
    let trimmed = text.trim();

    // 1) 标记协议优先：===CHAPTER_BEGIN=== … ===CHAPTER_END===
    if let Some(b) = trimmed.find(CHAPTER_BEGIN) {
        let rest = &trimmed[b + CHAPTER_BEGIN.len()..];
        let body = match rest.find(CHAPTER_END) {
            Some(e) => &rest[..e],
            None => rest,
        };
        let body = strip_code_fence(body.trim());
        if !body.is_empty() {
            return body;
        }
    }

    // 2) 剥代码围栏
    let stripped = strip_code_fence(trimmed);
    // 3) 剥离前导规划文本（模型把 planning 输出在正文前面时）
    strip_planning_prefix(&stripped)
}

/// 剥掉整体包裹的 Markdown 代码围栏（如 ```text … ```）
fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let body = trimmed
        .trim_start_matches('`')
        .trim_start_matches(|c: char| c.is_alphabetic()) // 剥掉语言标记如 ```text
        .trim_start();
    if let Some(end) = body.rfind("```") {
        body[..end].trim().to_string()
    } else {
        body.trim().to_string()
    }
}

/// 剥离正文前的规划/思考文本。
///
/// 模型常先输出英文或中文的写作规划（"Let me carefully write…"、
/// "Scene 1…"、"场景一：…"、大纲编号等）再开始正文。规则：逐行扫描，
/// 遇到第一行「正文特征行」（含中文且不是规划模式）时，从该行开始截取。
fn strip_planning_prefix(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut start = 0usize;
    for (i, line) in lines.iter().enumerate() {
        let l = line.trim();
        if l.is_empty() {
            continue; // 前缀空行跳过
        }
        let is_planning = {
            let lower = l.to_lowercase();
            l.starts_with("##")
                || l.starts_with("###")
                || l.starts_with("- ")
                || l.starts_with("* ")
                || l.starts_with("【")
                || lower.starts_with("scene")
                || lower.starts_with("step")
                || lower.starts_with("let me")
                || lower.starts_with("i need")
                || lower.starts_with("i'll")
                || lower.starts_with("i will")
                || lower.contains("场景 ")
                || lower.contains("章节规划")
                || lower.contains("chapter ")
                || lower.starts_with("plan")
                || lower.starts_with("draft")
                || (l.starts_with(|c: char| c.is_ascii_digit())
                    && (l.contains('.') || l.contains('、') || l.contains(')')))
                || !l.contains(|c: char| c >= '\u{4e00}' && c <= '\u{9fff}') // 纯英文/符号行
        };
        if !is_planning {
            start = i;
            break;
        }
    }
    lines[start..].join("\n").trim().to_string()
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
    let diagnosis = json
        .get("diagnosis")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        // 字符串形态兜底：只给问题族
                        Some(DiagnosisItem {
                            family: s.to_string(),
                            trigger: String::new(),
                            action: String::new(),
                            rewrite: true,
                        })
                    } else {
                        let family = item
                            .get("family")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        if family.is_empty() {
                            return None;
                        }
                        Some(DiagnosisItem {
                            family,
                            trigger: item
                                .get("trigger")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            action: item
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            rewrite: item
                                .get("rewrite")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true),
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let hook_score = json.get("hook").and_then(|v| v.as_f64());
    let payoff_score = json.get("payoff").and_then(|v| v.as_f64());

    let report = between(text, REPORT_BEGIN, REPORT_END)
        .map(str::to_string)
        .unwrap_or_else(|| format!("一致性评分 {score:.0}"));
    Ok((
        ReviewSignal {
            score,
            issues,
            diagnosis,
            hook_score,
            payoff_score,
        },
        report,
    ))
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

/// 解析章前策划输出：提取 JSON 节拍表；失败时取最长连续文本段兜底。
pub fn parse_planning_output(raw: &str) -> String {
    let cleaned = raw
        .trim()
        .strip_prefix("```json")
        .or_else(|| raw.trim().strip_prefix("```"))
        .map(|s| s.strip_suffix("```").unwrap_or(s).trim())
        .unwrap_or(raw.trim());
    if serde_json::from_str::<serde_json::Value>(cleaned).is_ok() {
        return cleaned.to_string();
    }
    let longest = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .max_by_key(|l| l.chars().count())
        .unwrap_or(raw)
        .to_string();
    if longest.chars().count() > 200 {
        longest
    } else {
        raw.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stages_topology() {
        let stages = pipeline_stages(None);
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
    fn test_pipeline_stages_with_planning() {
        use pensoul_core::workflow::builtin_workflow_templates;
        let tpl = builtin_workflow_templates()
            .into_iter()
            .find(|t| t.template_id == "webnovel")
            .expect("内置网文模板必须存在");
        let stages = pipeline_stages(Some(&tpl));
        assert_eq!(stages.len(), 4);
        assert_eq!(stages[0].name.as_str(), STAGE_PLANNING);
        assert_eq!(stages[0].display_name, "章前策划");
        assert_eq!(
            stages[0].next_stage.as_ref().map(|s| s.as_str()),
            Some(STAGE_WRITING)
        );
        assert_eq!(stages[1].name.as_str(), STAGE_WRITING);
        assert_eq!(stages[3].name.as_str(), STAGE_INJECTION);
    }

    #[test]
    fn test_parse_planning_output() {
        let fenced = "```json\n{\"chapter_goal\": \"主角首次用金手指脱困\", \"beats\": []}\n```";
        let plan = parse_planning_output(fenced);
        assert!(plan.contains("chapter_goal"));
        // 兜底：纯文本取最长段
        let plain = "第一段很短。\n第二段是完整的节拍说明：开篇冲突、三个场景、结尾钩子，内容足够长可以成为兜底。";
        let fallback = parse_planning_output(plain);
        assert!(fallback.contains("节拍说明"));
    }

    #[test]
    fn test_parse_writing_strips_code_fence() {
        let fenced = "```\n第一章的正文内容。\n第二段。\n```";
        assert_eq!(parse_writing_output(fenced), "第一章的正文内容。\n第二段。");
        let plain = "直接的正文";
        assert_eq!(parse_writing_output(plain), "直接的正文");
    }

    #[test]
    fn test_parse_writing_chapter_markers() {
        let raw = "Let me carefully write this chapter.\n===CHAPTER_BEGIN===\n他最后记得的，是天台栏杆上一枚反射灯光的金属铆钉。\n第二段正文。\n===CHAPTER_END===\n以上是本章正文。";
        let out = parse_writing_output(raw);
        assert!(out.starts_with("他最后记得的"));
        assert!(!out.contains("CHAPTER"));
        assert!(!out.contains("Let me"));
    }

    #[test]
    fn test_parse_writing_strips_planning_prefix() {
        // 模型把英文规划 + 中文场景规划混在正文前（无标记协议时的兜底）
        let raw = "Let me carefully write Chapter 1 of this novel.\nI need to: 1. write 3000 chars 2. build the opening\nScene 1 (550 words): Falling and landing\n- Opening hook\n- The falling sensation\n\n他最后记得的，是天台栏杆上一枚反射灯光的金属铆钉。\n金属铆钉很小，藏在栏杆底部。";
        let out = parse_writing_output(raw);
        assert!(out.starts_with("他最后记得的"));
        assert!(!out.contains("Let me"));
        assert!(!out.contains("Scene 1"));
        assert!(out.contains("金属铆钉很小"));
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
        assert!(signal.diagnosis.is_empty()); // 旧格式向后兼容
    }

    #[test]
    fn test_parse_review_diagnosis() {
        let text = r#"===SIGNAL_BEGIN===
{"score": 72, "issues": ["断章钩子不足"], "diagnosis": [
  {"family": "断章钩子", "trigger": "结尾写成「他转身离开」", "action": "改成停在疑问或危机上", "rewrite": true},
  {"family": "翻译腔", "trigger": "基于此，使得故事得以推进", "action": "拆成主动短句", "rewrite": true}
]}
===SIGNAL_END===
===REPORT_BEGIN===
报告
===REPORT_END==="#;
        let (signal, _) = parse_review_output(text).unwrap();
        assert_eq!(signal.diagnosis.len(), 2);
        assert_eq!(signal.diagnosis[0].family, "断章钩子");
        assert_eq!(signal.diagnosis[1].trigger, "基于此，使得故事得以推进");
        assert!(signal.diagnosis[0].rewrite);
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
