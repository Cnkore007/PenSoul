/// PenSoul LLM crate - LLM 模型管理
pub mod comparison;
pub mod inspiration;
pub mod model;
pub mod provider;
pub mod router;

// 重新导出主要类型
pub use comparison::{ComparisonResult, ModelComparison, compare_models};
pub use inspiration::{InspirationItem, generate_inspiration};
pub use model::{ModelConfig, RoutingResult, TaskType};
pub use provider::{AnthropicProvider, LlmProvider, OpenAiProvider};
pub use router::ModelRouter;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 创建测试用的模型配置
    fn create_test_model(id: &str, available: bool, failure_count: u32) -> ModelConfig {
        ModelConfig {
            model_id: id.to_string(),
            provider: "test_provider".to_string(),
            display_name: format!("Test Model {}", id),
            max_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            cost_per_1k_tokens: 0.01,
            avg_quality_score: 0.8,
            avg_latency_ms: 100,
            is_available: available,
            failure_count,
            last_failure_time: 0.0,
            cooldown_seconds: 300,
            api_key: None,
            endpoint: None,
        }
    }

    /// 创建带冷却时间的模型配置
    fn create_cooled_model(id: &str, cooldown_remaining: f64) -> ModelConfig {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        ModelConfig {
            model_id: id.to_string(),
            provider: "test_provider".to_string(),
            display_name: format!("Cooled Model {}", id),
            max_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            cost_per_1k_tokens: 0.02,
            avg_quality_score: 0.9,
            avg_latency_ms: 50,
            is_available: true,
            failure_count: 3,
            last_failure_time: current_time - (300.0 - cooldown_remaining),
            cooldown_seconds: 300,
            api_key: None,
            endpoint: None,
        }
    }

    // ── 验收 #1: route 返回 task_preferences 中第一个可用模型 ──
    #[test]
    fn test_route_returns_first_preferred_available_model() {
        let mut router = ModelRouter::new();

        // 注册两个模型
        router.register_model(create_test_model("model_a", true, 0));
        router.register_model(create_test_model("model_b", true, 0));

        // 设置偏好：model_b 在前
        router.set_task_preference(
            TaskType::Drafting,
            vec!["model_b".to_string(), "model_a".to_string()],
        );

        // 路由应该返回 model_b
        let result = router.route(TaskType::Drafting);
        assert!(result.is_ok());

        let routing_result = result.unwrap();
        assert_eq!(routing_result.chosen_model.model_id, "model_b");
        assert!(!routing_result.fallback_used);
        assert!(
            routing_result
                .attempt_chain
                .contains(&"model_b".to_string())
        );
    }

    // ── 验收 #2: report_failure 3 次后 route 跳过该模型 ──
    #[test]
    fn test_report_failure_three_times_skips_model() {
        let mut router = ModelRouter::new();

        // 注册两个模型
        router.register_model(create_test_model("model_a", true, 0));
        router.register_model(create_test_model("model_b", true, 0));

        // 设置偏好：model_a 在前
        router.set_task_preference(
            TaskType::Revision,
            vec!["model_a".to_string(), "model_b".to_string()],
        );

        // 报告 model_a 失败 3 次
        for _ in 0..3 {
            router.report_failure("model_a");
        }

        // 路由应该跳过 model_a，返回 model_b
        let result = router.route(TaskType::Revision);
        assert!(result.is_ok());

        let routing_result = result.unwrap();
        assert_eq!(routing_result.chosen_model.model_id, "model_b");
        assert!(routing_result.fallback_used);
        assert!(
            routing_result
                .attempt_chain
                .contains(&"model_a".to_string())
        );
        assert!(
            routing_result
                .attempt_chain
                .contains(&"model_b".to_string())
        );
    }

    // ── 验收 #3: 所有模型失败返回 LlmAllModelsFailed ──
    #[test]
    fn test_all_models_failed_returns_error() {
        let mut router = ModelRouter::new();

        // 注册两个模型
        router.register_model(create_test_model("model_a", true, 0));
        router.register_model(create_test_model("model_b", true, 0));

        // 设置偏好
        router.set_task_preference(
            TaskType::Consistency,
            vec!["model_a".to_string(), "model_b".to_string()],
        );

        // 报告两个模型都失败 3 次
        for _ in 0..3 {
            router.report_failure("model_a");
            router.report_failure("model_b");
        }

        // 路由应该返回错误
        let result = router.route(TaskType::Consistency);
        assert!(result.is_err());

        match result.unwrap_err() {
            pensoul_core::PensoulError::LlmAllModelsFailed { chain } => {
                assert!(chain.len() >= 2);
                assert!(chain.contains(&"model_a".to_string()));
                assert!(chain.contains(&"model_b".to_string()));
            }
            other => panic!("期望 LlmAllModelsFailed，实际: {:?}", other),
        }
    }

    // ── 验收 #4: 超过 cooldown_seconds 后模型恢复可用 ──
    #[test]
    fn test_model_recovers_after_cooldown() {
        let mut router = ModelRouter::new();

        // 注册一个冷却中的模型（剩余 0 秒冷却）
        router.register_model(create_cooled_model("model_a", 0.0));

        // 设置偏好
        router.set_task_preference(TaskType::Style, vec!["model_a".to_string()]);

        // 路由应该成功，因为冷却已过
        let result = router.route(TaskType::Style);
        assert!(result.is_ok());

        let routing_result = result.unwrap();
        assert_eq!(routing_result.chosen_model.model_id, "model_a");
    }

    // ── 验收 #5: get_recommendation 返回偏好模型列表 ──
    #[test]
    fn test_get_recommendation_returns_preferred_models() {
        let mut router = ModelRouter::new();

        // 注册三个模型
        router.register_model(create_test_model("model_a", true, 0));
        router.register_model(create_test_model("model_b", true, 0));
        router.register_model(create_test_model("model_c", true, 0));

        // 设置偏好
        router.set_task_preference(
            TaskType::Outline,
            vec!["model_c".to_string(), "model_a".to_string()],
        );

        // 获取推荐
        let recommendations = router.get_recommendation(TaskType::Outline);
        assert_eq!(recommendations.len(), 2);
        assert_eq!(recommendations[0].model_id, "model_c");
        assert_eq!(recommendations[1].model_id, "model_a");
    }

    // ── 验收 #6: 每次路由都记录在 routing_log 中 ──
    #[test]
    fn test_routing_log_records_all_routes() {
        let mut router = ModelRouter::new();

        // 注册模型
        router.register_model(create_test_model("model_a", true, 0));
        router.register_model(create_test_model("model_b", true, 0));

        // 设置偏好
        router.set_task_preference(TaskType::Drafting, vec!["model_a".to_string()]);

        // 执行两次路由
        let _ = router.route(TaskType::Drafting);
        let _ = router.route(TaskType::Revision);

        // 检查路由日志
        let log = router.get_routing_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].chosen_model.model_id, "model_a");
        // 第二次路由没有偏好，会路由到任意可用模型
        assert!(
            log[1].chosen_model.model_id == "model_a" || log[1].chosen_model.model_id == "model_b"
        );
    }

    // ── 额外测试：provider trait 基本功能 ──
    #[test]
    fn test_provider_basic_functionality() {
        let openai_provider = OpenAiProvider::new("test_api_key".to_string());
        assert_eq!(openai_provider.name(), "openai");

        let anthropic_provider = AnthropicProvider::new("test_api_key".to_string());
        assert_eq!(anthropic_provider.name(), "anthropic");

        // 测试调用返回错误（尚未实现）
        let model = create_test_model("test", true, 0);
        let result = openai_provider.call(&model, "test prompt");
        assert!(result.is_err());

        let result = anthropic_provider.call(&model, "test prompt");
        assert!(result.is_err());
    }

    // ── 额外测试：模型注册和状态管理 ──
    #[test]
    fn test_model_registration_and_status() {
        let mut router = ModelRouter::new();

        // 注册模型
        let model = create_test_model("model_a", true, 0);
        router.register_model(model);

        // 验证模型已注册
        let recommendations = router.get_recommendation(TaskType::General);
        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].model_id, "model_a");

        // 报告失败
        router.report_failure("model_a");

        // 验证失败计数增加
        let recommendations = router.get_recommendation(TaskType::General);
        assert_eq!(recommendations[0].failure_count, 1);
    }
}
