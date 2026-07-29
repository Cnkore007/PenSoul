/// PenSoul LLM 模型路由器
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use pensoul_core::{PensoulError, Result};

use crate::model::{ModelConfig, RoutingResult, TaskType};

/// 模型路由器
pub struct ModelRouter {
    /// 已注册的模型，按 model_id 索引
    models: HashMap<String, ModelConfig>,
    /// 任务类型到模型 ID 列表的偏好映射
    task_preferences: HashMap<TaskType, Vec<String>>,
    /// 路由日志
    routing_log: Vec<RoutingResult>,
}

impl ModelRouter {
    /// 创建新的路由器
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            task_preferences: HashMap::new(),
            routing_log: Vec::new(),
        }
    }

    /// 注册模型
    pub fn register_model(&mut self, model: ModelConfig) {
        self.models.insert(model.model_id.clone(), model);
    }

    /// 设置任务偏好
    pub fn set_task_preference(&mut self, task_type: TaskType, model_ids: Vec<String>) {
        self.task_preferences.insert(task_type, model_ids);
    }

    /// 路由到可用模型
    pub fn route(&mut self, task_type: TaskType) -> Result<RoutingResult> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间应该在 UNIX 纪元之后")
            .as_millis() as u64;

        let mut attempt_chain = Vec::new();
        let mut fallback_used = false;
        let mut fallback_reason = String::new();

        // 获取当前时间戳（秒）
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间应该在 UNIX 纪元之后")
            .as_secs_f64();

        // 1. 首先尝试任务偏好中的模型
        if let Some(preferred_ids) = self.task_preferences.get(&task_type) {
            for model_id in preferred_ids {
                attempt_chain.push(model_id.clone());

                if let Some(model) = self.models.get(model_id) {
                    // 检查模型是否可用
                    if !model.is_available {
                        fallback_reason = format!("模型 {} 不可用", model_id);
                        fallback_used = true;
                        continue;
                    }

                    // 检查冷却时间
                    if model.failure_count >= 3 {
                        let time_since_failure = current_time - model.last_failure_time;
                        if time_since_failure < model.cooldown_seconds as f64 {
                            fallback_reason = format!(
                                "模型 {} 在冷却中，剩余 {:.0} 秒",
                                model_id,
                                model.cooldown_seconds as f64 - time_since_failure
                            );
                            fallback_used = true;
                            continue;
                        }
                    }

                    // 找到可用模型
                    let routing_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("时间应该在 UNIX 纪元之后")
                        .as_millis() as u64
                        - start_time;

                    let result = RoutingResult {
                        chosen_model: model.clone(),
                        fallback_used,
                        fallback_reason,
                        attempt_chain,
                        routing_time_ms: routing_time,
                    };

                    self.routing_log.push(result.clone());
                    return Ok(result);
                }
            }
        }

        // 2. 如果偏好中的模型都不可用，尝试所有注册模型
        for (model_id, model) in &self.models {
            // 跳过已经尝试过的模型
            if attempt_chain.contains(model_id) {
                continue;
            }

            attempt_chain.push(model_id.clone());

            // 检查模型是否可用
            if !model.is_available {
                fallback_reason = format!("模型 {} 不可用", model_id);
                continue;
            }

            // 检查冷却时间
            if model.failure_count >= 3 {
                let time_since_failure = current_time - model.last_failure_time;
                if time_since_failure < model.cooldown_seconds as f64 {
                    fallback_reason = format!(
                        "模型 {} 在冷却中，剩余 {:.0} 秒",
                        model_id,
                        model.cooldown_seconds as f64 - time_since_failure
                    );
                    continue;
                }
            }

            // 找到可用模型
            let routing_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("时间应该在 UNIX 纪元之后")
                .as_millis() as u64
                - start_time;

            let result = RoutingResult {
                chosen_model: model.clone(),
                fallback_used: true,
                fallback_reason,
                attempt_chain,
                routing_time_ms: routing_time,
            };

            self.routing_log.push(result.clone());
            return Ok(result);
        }

        // 3. 所有模型都不可用
        let routing_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间应该在 UNIX 纪元之后")
            .as_millis() as u64
            - start_time;

        // 创建一个虚拟的 ModelConfig 用于错误结果
        let dummy_model = ModelConfig {
            model_id: "none".to_string(),
            provider: "none".to_string(),
            display_name: "No Model".to_string(),
            max_tokens: 0,
            supports_tools: false,
            supports_streaming: false,
            cost_per_1k_tokens: 0.0,
            avg_quality_score: 0.0,
            avg_latency_ms: 0,
            is_available: false,
            failure_count: 0,
            last_failure_time: 0.0,
            cooldown_seconds: 0,
            api_key: None,
            endpoint: None,
        };

        let result = RoutingResult {
            chosen_model: dummy_model,
            fallback_used: true,
            fallback_reason: "所有模型都不可用".to_string(),
            attempt_chain,
            routing_time_ms: routing_time,
        };

        self.routing_log.push(result.clone());
        Err(PensoulError::LlmAllModelsFailed {
            chain: result.attempt_chain.clone(),
        })
    }

    /// 报告模型失败
    pub fn report_failure(&mut self, model_id: &str) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.failure_count += 1;
            model.last_failure_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("时间应该在 UNIX 纪元之后")
                .as_secs_f64();

            // 当失败次数 >= 3 时，设置不可用
            if model.failure_count >= 3 {
                model.is_available = false;
            }
        }
    }

    /// 获取推荐模型
    pub fn get_recommendation(&self, task_type: TaskType) -> Vec<&ModelConfig> {
        if let Some(preferred_ids) = self.task_preferences.get(&task_type) {
            preferred_ids
                .iter()
                .filter_map(|model_id| self.models.get(model_id))
                .collect()
        } else {
            self.models.values().collect()
        }
    }

    /// 从所有任务偏好列表中移除指定模型
    pub fn remove_from_all_preferences(&mut self, model_id: &str) {
        let task_types: Vec<TaskType> = self.task_preferences.keys().cloned().collect();
        for task_type in task_types {
            if let Some(preferred) = self.task_preferences.get_mut(&task_type) {
                preferred.retain(|id| id != model_id);
            }
        }
    }

    /// 获取路由日志
    pub fn get_routing_log(&self) -> &[RoutingResult] {
        &self.routing_log
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}
