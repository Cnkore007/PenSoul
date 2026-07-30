//! PenSoul LLM 模型路由器
//!
//! 可用性状态机：
//! - `failure_count < 3`：正常可用。
//! - `failure_count >= 3` 且冷却未到期：跳过（冷却中）。
//! - `failure_count >= 3` 且冷却已到期：半开（half-open），允许尝试；
//!   成功后应通过 `report_success` 清零失败计数，恢复完全可用。
//!
//! 故障转移遍历按 model_id 排序，保证路由结果的确定性。
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
        let start_time = now_millis();
        let current_time = now_secs_f64();

        let mut attempt_chain = Vec::new();
        let mut fallback_used = false;
        let mut fallback_reason = String::new();

        // 1. 首先尝试任务偏好中的模型（按声明顺序）
        if let Some(preferred_ids) = self.task_preferences.get(&task_type) {
            for model_id in preferred_ids {
                attempt_chain.push(model_id.clone());

                if let Some(model) = self.models.get(model_id) {
                    match availability(model, current_time) {
                        Availability::Available => {
                            let result = RoutingResult {
                                chosen_model: model.clone(),
                                fallback_used,
                                fallback_reason,
                                attempt_chain,
                                routing_time_ms: now_millis() - start_time,
                            };
                            self.routing_log.push(result.clone());
                            return Ok(result);
                        }
                        Availability::Unavailable(reason) => {
                            fallback_reason = reason;
                            fallback_used = true;
                        }
                    }
                }
            }
        }

        // 2. 偏好中的模型都不可用：按 model_id 排序遍历其余模型（确定性）
        let mut remaining: Vec<&String> = self
            .models
            .keys()
            .filter(|id| !attempt_chain.contains(id))
            .collect();
        remaining.sort();

        for model_id in remaining {
            attempt_chain.push(model_id.clone());
            let model = &self.models[model_id];

            match availability(model, current_time) {
                Availability::Available => {
                    let result = RoutingResult {
                        chosen_model: model.clone(),
                        fallback_used: true,
                        fallback_reason,
                        attempt_chain,
                        routing_time_ms: now_millis() - start_time,
                    };
                    self.routing_log.push(result.clone());
                    return Ok(result);
                }
                Availability::Unavailable(reason) => {
                    fallback_reason = reason;
                }
            }
        }

        // 3. 所有模型都不可用 — 只记录尝试链，不伪造模型
        let _ = fallback_reason;
        Err(PensoulError::LlmAllModelsFailed {
            chain: attempt_chain,
        })
    }

    /// 报告模型失败：累加失败计数并记录时间。
    ///
    /// 失败计数达到阈值后进入冷却；冷却到期自动转为半开，
    /// 成功后由 `report_success` 恢复。模型不会被永久禁用。
    pub fn report_failure(&mut self, model_id: &str) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.failure_count += 1;
            model.last_failure_time = now_secs_f64();
        }
    }

    /// 报告模型成功：清零失败计数，恢复完全可用。
    pub fn report_success(&mut self, model_id: &str) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.failure_count = 0;
            model.is_available = true;
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

/// 模型可用性判定结果
enum Availability {
    Available,
    Unavailable(String),
}

/// 判定模型当前是否可用（含冷却/半开逻辑）。
fn availability(model: &ModelConfig, current_time: f64) -> Availability {
    // 显式下线（运维摘除）——与冷却无关，直接不可用
    if !model.is_available && model.failure_count < 3 {
        return Availability::Unavailable(format!("模型 {} 不可用", model.model_id));
    }

    // 冷却判定：失败次数达阈值后，冷却期内跳过，到期后半开
    if model.failure_count >= 3 {
        let time_since_failure = current_time - model.last_failure_time;
        if time_since_failure < model.cooldown_seconds as f64 {
            return Availability::Unavailable(format!(
                "模型 {} 在冷却中，剩余 {:.0} 秒",
                model.model_id,
                model.cooldown_seconds as f64 - time_since_failure
            ));
        }
        // 冷却到期：半开，允许尝试
    }

    Availability::Available
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn now_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
