/// 门控三模式评估器。
///
/// 决定一个阶段完成后是否放行进入下一阶段。
/// 三种模式对应不同的决策方式：
/// - `Auto`: 无条件放行
/// - `Manual`: 等待人工确认
/// - `Conditional`: 根据评分或条件表达式判定
use crate::stage::{GateResult, GateType, Stage};
use pensoul_core::{PensoulError, Result};

/// 门控评估器。
///
/// 负责根据阶段的门控配置和产出结果，判定是否放行。
#[derive(Debug, Clone)]
pub struct GateEvaluator;

impl GateEvaluator {
    /// 评估门控是否通过。
    ///
    /// # 逻辑
    /// - **Auto**: 直接返回 `passed = true`。
    /// - **Manual**: 仅当引擎收到带外人工批准（`manual_approved = true`）时放行。
    ///   注意：绝不读取 `result` 中的字段来判断人工意图——阶段产出由 AI 生成，
    ///   AI 可以通过写入 `human_approved: true` 自我批准，使人工门控失效。
    /// - **Conditional**: 若配置了 `stage.gate_condition` 表达式（如 `score >= 85`），
    ///   优先按表达式判定（工作流模板可用它自定义审查阈值）；
    ///   否则读取 `result.score`，>= 80 则通过；两者皆无则拦截。
    ///
    /// # 错误
    /// 当条件表达式无法解析时返回 `GateConditionFailed`。
    pub fn evaluate(
        stage: &Stage,
        result: &serde_json::Value,
        manual_approved: bool,
    ) -> Result<GateResult> {
        match &stage.gate_type {
            GateType::Auto => Ok(GateResult {
                passed: true,
                score: None,
                reason: "自动放行".to_string(),
            }),

            GateType::Manual => Ok(GateResult {
                passed: manual_approved,
                score: None,
                reason: if manual_approved {
                    "人工确认通过".to_string()
                } else {
                    "等待人工确认".to_string()
                },
            }),

            GateType::Conditional => {
                // 优先尝试 gate_condition 表达式（模板自定义阈值走这里）
                if let Some(ref condition) = stage.gate_condition {
                    let passed = Self::evaluate_condition(condition, result)?;
                    let score = result.get("score").and_then(|v| v.as_f64());
                    return Ok(GateResult {
                        passed,
                        score,
                        reason: if passed {
                            format!("条件表达式通过: {condition}")
                        } else {
                            format!("条件表达式未满足: {condition}")
                        },
                    });
                }

                // 无表达式时回退到默认阈值 80
                if let Some(score) = result.get("score").and_then(|v| v.as_f64()) {
                    let passed = score >= 80.0;
                    return Ok(GateResult {
                        passed,
                        score: Some(score),
                        reason: if passed {
                            format!("条件放行：分数 {score} >= 80")
                        } else {
                            format!("条件拦截：分数 {score} < 80")
                        },
                    });
                }

                // 既无 score 也无 condition，默认拦截
                Ok(GateResult {
                    passed: false,
                    score: None,
                    reason: "条件放行但无分数或条件表达式".to_string(),
                })
            }
        }
    }

    /// 解析简单的条件表达式。
    ///
    /// 支持格式：`field_name >= number`、`field_name > number`、
    /// `field_name <= number`、`field_name < number`、`field_name == value`，
    /// 以及用 `&&` 连接的多条件（如 `score >= 80 && hook >= 8 && payoff >= 8`，全部满足才放行）。
    fn evaluate_condition(condition: &str, result: &serde_json::Value) -> Result<bool> {
        let condition = condition.trim();

        // 多条件：先按 && 拆开，逐条判定，全部通过才放行
        if condition.contains("&&") {
            let clauses: Vec<&str> = condition.split("&&").map(|c| c.trim()).collect();
            if clauses.len() > 1 {
                let mut passed = true;
                for clause in clauses {
                    passed = Self::evaluate_single_condition(clause, result)? && passed;
                }
                return Ok(passed);
            }
        }

        Self::evaluate_single_condition(condition, result)
    }

    /// 单条 `field op value` 判定。
    fn evaluate_single_condition(
        condition: &str,
        result: &serde_json::Value,
    ) -> Result<bool> {
        let condition = condition.trim();
        // 解析 "field op value" 模式
        for op in &["==", ">=", "<=", ">", "<"] {
            if let Some((field, value_str)) = condition.split_once(op) {
                let field = field.trim();
                let value_str = value_str.trim();

                let field_value =
                    result
                        .get(field)
                        .ok_or_else(|| PensoulError::GateConditionFailed {
                            reason: format!("字段 '{field}' 不存在于结果中"),
                        })?;

                // 尝试数值比较
                if let (Some(num_val), Some(threshold)) =
                    (field_value.as_f64(), value_str.parse::<f64>().ok())
                {
                    let passed = match *op {
                        ">=" => num_val >= threshold,
                        "<=" => num_val <= threshold,
                        ">" => num_val > threshold,
                        "<" => num_val < threshold,
                        "==" => (num_val - threshold).abs() < f64::EPSILON,
                        _ => false,
                    };
                    return Ok(passed);
                }

                // 字符串比较（仅 ==）
                if *op == "==" {
                    let passed = field_value.as_str() == Some(value_str);
                    return Ok(passed);
                }

                return Err(PensoulError::GateConditionFailed {
                    reason: format!(
                        "无法比较字段 '{field}' (值: {field_value}) 与阈值 '{value_str}'"
                    ),
                });
            }
        }

        Err(PensoulError::GateConditionFailed {
            reason: format!("无法解析条件表达式: '{condition}'"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{GateType, Stage};
    use pensoul_core::StageName;

    fn make_stage(gate_type: GateType, condition: Option<&str>) -> Stage {
        Stage {
            name: StageName::new("test"),
            gate_type,
            gate_condition: condition.map(String::from),
            ..Stage::default()
        }
    }

    #[test]
    fn test_auto_gate_always_passes() {
        let stage = make_stage(GateType::Auto, None);
        let result = serde_json::json!({});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(gr.passed);
    }

    #[test]
    fn test_manual_gate_approved_out_of_band() {
        let stage = make_stage(GateType::Manual, None);
        // 人工批准来自带外通道，与 result 内容无关
        let result = serde_json::json!({});
        let gr = GateEvaluator::evaluate(&stage, &result, true).unwrap();
        assert!(gr.passed);
    }

    #[test]
    fn test_manual_gate_rejected() {
        let stage = make_stage(GateType::Manual, None);
        let result = serde_json::json!({});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_manual_gate_ignores_result_field() {
        let stage = make_stage(GateType::Manual, None);
        // AI 在 result 中伪造 human_approved 也不能放行
        let result = serde_json::json!({"human_approved": true});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_conditional_gate_score_pass() {
        let stage = make_stage(GateType::Conditional, None);
        let result = serde_json::json!({"score": 85.0});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(gr.passed);
        assert_eq!(gr.score, Some(85.0));
    }

    #[test]
    fn test_conditional_gate_score_fail() {
        let stage = make_stage(GateType::Conditional, None);
        let result = serde_json::json!({"score": 60.0});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_conditional_gate_boundary_80() {
        let stage = make_stage(GateType::Conditional, None);
        let result = serde_json::json!({"score": 80.0});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(gr.passed);
    }

    #[test]
    fn test_conditional_gate_condition_expression() {
        let stage = make_stage(GateType::Conditional, Some("consistency_score >= 80"));
        let result = serde_json::json!({"consistency_score": 90});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(gr.passed);
    }

    #[test]
    fn test_conditional_gate_condition_fail() {
        let stage = make_stage(GateType::Conditional, Some("consistency_score >= 80"));
        let result = serde_json::json!({"consistency_score": 70});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_conditional_gate_template_threshold_via_condition() {
        // 工作流模板自定义审查阈值：score >= 85 才放行（高于默认 80）
        let stage = make_stage(GateType::Conditional, Some("score >= 85"));
        let low = serde_json::json!({"score": 82.0});
        let low_gr = GateEvaluator::evaluate(&stage, &low, false).unwrap();
        assert!(!low_gr.passed);
        assert_eq!(low_gr.score, Some(82.0));

        let high = serde_json::json!({"score": 90.0});
        let high_gr = GateEvaluator::evaluate(&stage, &high, false).unwrap();
        assert!(high_gr.passed);
        assert_eq!(high_gr.score, Some(90.0));
    }

    #[test]
    fn test_conditional_gate_multi_clause_and() {
        // 黄金三章门控：总分达标 且 钩子/爽点维度均达标
        let stage = make_stage(
            GateType::Conditional,
            Some("score >= 80 && hook >= 8 && payoff >= 8"),
        );
        let pass = serde_json::json!({"score": 85.0, "hook": 9.0, "payoff": 8.0});
        let gr = GateEvaluator::evaluate(&stage, &pass, false).unwrap();
        assert!(gr.passed);
        // 钩子不达标 → 即使总分够也被拦截
        let fail_hook = serde_json::json!({"score": 88.0, "hook": 6.0, "payoff": 9.0});
        let gr = GateEvaluator::evaluate(&stage, &fail_hook, false).unwrap();
        assert!(!gr.passed);
        // 爽点不达标 → 同样拦截
        let fail_payoff = serde_json::json!({"score": 90.0, "hook": 9.0, "payoff": 4.0});
        let gr = GateEvaluator::evaluate(&stage, &fail_payoff, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_conditional_gate_no_score_no_condition() {
        let stage = make_stage(GateType::Conditional, None);
        let result = serde_json::json!({});
        let gr = GateEvaluator::evaluate(&stage, &result, false).unwrap();
        assert!(!gr.passed);
    }

    #[test]
    fn test_evaluate_condition_invalid_expr() {
        let stage = make_stage(GateType::Conditional, Some("nonsense_expression"));
        let result = serde_json::json!({});
        let err = GateEvaluator::evaluate(&stage, &result, false);
        assert!(err.is_err());
    }
}
