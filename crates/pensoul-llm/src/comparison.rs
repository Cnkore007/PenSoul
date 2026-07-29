/// PenSoul LLM 多模型对比
use crate::model::{ModelConfig, TaskType};
use crate::router::ModelRouter;

/// 模型对比结果
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// 模型 ID
    pub model_id: String,
    /// 输出内容
    pub output: String,
    /// 延迟（毫秒）
    pub latency_ms: u64,
    /// 质量评分
    pub quality_score: f32,
}

/// 模型对比
#[derive(Debug, Clone)]
pub struct ModelComparison {
    /// 参与对比的模型
    pub models: Vec<ModelConfig>,
    /// 对比结果
    pub results: Vec<ComparisonResult>,
}

/// 对比多个模型
pub fn compare_models(router: &ModelRouter, task_type: TaskType, prompt: &str) -> ModelComparison {
    // 获取推荐模型列表
    let recommended_models = router.get_recommendation(task_type);

    // 这里只是示例实现，实际中应该调用每个模型并收集结果
    // 由于 provider 尚未实现，我们返回一个模拟的对比结果

    let models: Vec<ModelConfig> = recommended_models.into_iter().cloned().collect();

    // 模拟对比结果
    let results = models
        .iter()
        .map(|model| ComparisonResult {
            model_id: model.model_id.clone(),
            output: format!(
                "模型 {} 对提示 \"{}\" 的响应（模拟）",
                model.display_name, prompt
            ),
            latency_ms: model.avg_latency_ms as u64,
            quality_score: model.avg_quality_score,
        })
        .collect();

    ModelComparison { models, results }
}
