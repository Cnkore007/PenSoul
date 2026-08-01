//! 模型档案：按模型名称自动匹配 API 参数体系
//!
//! 档案依据各厂商官方接口文档（2026-07 调研）：
//! - kimi-k3：永远思考，仅 reasoning_effort 可调档（low/high/max，默认 max）；
//!   预算字段为 max_completion_tokens；temperature/top_p/penalties 为固定值，
//!   显式传入会被拒绝
//! - kimi-k2.x：thinking 可开关（默认开）；采样参数同样为固定值
//! - glm-5.x：thinking 开关（默认开）+ reasoning_effort 调档，max_tokens 上限 65536
//! - deepseek-v4：thinking 开关（默认开）+ reasoning_effort（flash 默认 high）；
//!   思考模式下采样参数被服务端静默忽略
//!
//! 新模型按名称前缀自动落档；未命中走保守默认档。
//! 中转代理可能不透传扩展参数 —— 由 llm_helper 的 4xx 降级重试兜底。

/// 推理模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reasoning {
    /// 非推理模型
    None,
    /// 永远思考（kimi-k3）：Light 任务注入 reasoning_effort 最低档
    AlwaysEffort,
    /// thinking.type 可开关（glm / deepseek-v4 / kimi-k2）：Light 任务注入 disabled
    Toggleable,
}

/// 模型档案
#[derive(Debug, Clone, Copy)]
pub struct ModelProfile {
    /// 推理模式
    pub reasoning: Reasoning,
    /// 预算字段名（kimi-k3 用 max_completion_tokens，其余用 max_tokens）
    pub budget_field: &'static str,
    /// 单次输出硬上限（官方文档安全值），调用方预算超过时夹到这里
    pub max_output_tokens: u32,
    /// 采样参数固定（Kimi 系列显式传 temperature/top_p 会被拒绝）
    pub fixed_sampling: bool,
}

/// 任务深度：决定推理参数策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTask {
    /// 深度创作（蒸馏/讨论/写作/优化）：不注入推理参数，用官方默认思考强度
    Deep,
    /// 轻量结构任务（评审 JSON / 纪要）：尽量关思考或降到最低档，提速省费
    Light,
}

/// 按模型名匹配档案（小写前缀匹配；同族模型共用一档）
pub fn profile_for(model_id: &str) -> ModelProfile {
    let m = model_id.to_lowercase();
    let p = |reasoning, budget_field, max_output_tokens, fixed_sampling| ModelProfile {
        reasoning,
        budget_field,
        max_output_tokens,
        fixed_sampling,
    };
    if m.starts_with("kimi-k3") {
        p(
            Reasoning::AlwaysEffort,
            "max_completion_tokens",
            131_072,
            true,
        )
    } else if m.starts_with("kimi-k2") {
        p(Reasoning::Toggleable, "max_tokens", 32_768, true)
    } else if m.starts_with("moonshot") || m.starts_with("kimi") {
        p(Reasoning::None, "max_tokens", 8_192, false)
    } else if m.starts_with("glm-5") {
        p(Reasoning::Toggleable, "max_tokens", 65_536, false)
    } else if m.starts_with("glm") {
        p(Reasoning::Toggleable, "max_tokens", 8_192, false)
    } else if m.starts_with("deepseek-v4") {
        p(Reasoning::Toggleable, "max_tokens", 65_536, false)
    } else if m.starts_with("deepseek-reasoner") || m.starts_with("deepseek-r1") {
        // 老 R1 系：思考关不掉、无调档参数；降级重试会剔除不识别的字段
        p(Reasoning::AlwaysEffort, "max_tokens", 65_536, false)
    } else if m.starts_with("deepseek") {
        p(Reasoning::None, "max_tokens", 8_192, false)
    } else {
        // 默认档：OpenAI 兼容、非推理、保守输出上限
        p(Reasoning::None, "max_tokens", 16_384, false)
    }
}

/// 预算翻倍上限：夹在模型硬上限与 65536 之间（避免失控）
pub fn doubled_budget_cap(model_id: &str) -> u32 {
    profile_for(model_id).max_output_tokens.min(65_536)
}

/// 按档案 + 任务构建 OpenAI 兼容请求体
pub fn plan_request(
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    task: LlmTask,
) -> serde_json::Value {
    let profile = profile_for(model_id);
    let budget = max_tokens.min(profile.max_output_tokens);
    let mut body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        profile.budget_field: budget,
        "stream": true,
    });
    // 采样参数固定的模型（Kimi 系列）不传 temperature，避免被官方拒绝
    if !profile.fixed_sampling {
        body["temperature"] = serde_json::json!(temperature);
    }
    // Light 任务：按档案关闭或降低思考，轻量任务提速省费
    if task == LlmTask::Light {
        match profile.reasoning {
            Reasoning::Toggleable => {
                body["thinking"] = serde_json::json!({ "type": "disabled" });
            }
            Reasoning::AlwaysEffort => {
                body["reasoning_effort"] = serde_json::json!("low");
            }
            Reasoning::None => {}
        }
    }
    body
}

/// 降级请求体：剔除扩展参数（thinking / reasoning_effort / max_completion_tokens），
/// 预算字段回退 max_tokens、采样参数强制带上 ——
/// 用于中转代理不透传扩展参数时的 4xx 重试
pub fn plan_fallback_request(
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
) -> serde_json::Value {
    let profile = profile_for(model_id);
    let budget = max_tokens.min(profile.max_output_tokens);
    serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "max_tokens": budget,
        "temperature": temperature,
        "stream": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kimi_k3_profile() {
        let p = profile_for("kimi-k3");
        assert_eq!(p.reasoning, Reasoning::AlwaysEffort);
        assert_eq!(p.budget_field, "max_completion_tokens");
        assert!(p.fixed_sampling);
        assert_eq!(p.max_output_tokens, 131_072);
    }

    #[test]
    fn test_glm_and_deepseek_v4_profiles() {
        let glm = profile_for("glm-5.2");
        assert_eq!(glm.reasoning, Reasoning::Toggleable);
        assert_eq!(glm.max_output_tokens, 65_536);
        let ds = profile_for("deepseek-v4-flash");
        assert_eq!(ds.reasoning, Reasoning::Toggleable);
        // pro 同族落档
        let pro = profile_for("deepseek-v4-pro");
        assert_eq!(pro.reasoning, Reasoning::Toggleable);
    }

    #[test]
    fn test_unknown_model_falls_back_to_default() {
        let p = profile_for("gpt-5-turbo");
        assert_eq!(p.reasoning, Reasoning::None);
        assert_eq!(p.budget_field, "max_tokens");
        assert_eq!(p.max_output_tokens, 16_384);
        assert!(!p.fixed_sampling);
    }

    #[test]
    fn test_plan_request_k3_omits_temperature_and_uses_completion_field() {
        let body = plan_request("kimi-k3", "s", "u", 0.7, 16_384, LlmTask::Deep);
        assert!(body.get("temperature").is_none());
        assert_eq!(body["max_completion_tokens"], 16_384);
        assert!(body.get("max_tokens").is_none());
        // Deep 任务不注入推理参数，用官方默认强度
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_plan_request_light_task_controls_reasoning() {
        // 可开关模型：Light 注入 thinking=disabled
        let body = plan_request("glm-5.2", "s", "u", 0.3, 8_192, LlmTask::Light);
        assert_eq!(body["thinking"]["type"], "disabled");
        // 永远思考模型：Light 注入 reasoning_effort=low
        let body = plan_request("kimi-k3", "s", "u", 0.3, 8_192, LlmTask::Light);
        assert_eq!(body["reasoning_effort"], "low");
        // 非推理模型：不注入任何推理参数
        let body = plan_request("some-model", "s", "u", 0.3, 8_192, LlmTask::Light);
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_plan_request_clamps_budget_to_model_cap() {
        // kimi-k2 上限 32768，超出被夹住
        let body = plan_request("kimi-k2.6", "s", "u", 0.7, 65_536, LlmTask::Deep);
        assert_eq!(body["max_tokens"], 32_768);
    }

    #[test]
    fn test_fallback_request_strips_extensions() {
        let body = plan_fallback_request("kimi-k3", "s", "u", 0.7, 16_384);
        assert_eq!(body["max_tokens"], 16_384);
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
        // 降级体强制带 temperature（中转按标准 OpenAI 处理）
        assert_eq!(body["temperature"], 0.7);
    }
}
