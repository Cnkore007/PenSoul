//! 模型能力档案：按模型名称匹配官方文档参数体系
//!
//! 档案数据来源（2026-08-04 官方文档调研）：
//! - 智谱 GLM-5.2：1M 上下文、最大输出 128K、thinking.type 可开关（默认 enabled）、
//!   reasoning_effort 支持 max/xhigh/high/medium/low/minimal/none（none/minimal 放弃思考，
//!   low/medium 映射 high、xhigh 映射 max）、temperature 可调
//! - Moonshot Kimi K3：1M 上下文、单次最大输出 131072、始终思考不可关闭、
//!   reasoning_effort 仅 low/high/max（默认 max）、temperature/top_p 等采样参数固定
//! - DeepSeek V4：1M 上下文、最大输出 384K（默认 64K）、思考模式默认开启可关闭、
//!   默认 effort=high；flash 支持 low/high/max 三档，pro 当前仅 high/max（low 映射 high、
//!   xhigh 映射 max，预计 2026-08 初支持三档）；思考模式下采样参数被忽略
//!
//! 档案优先级：models.json 中每个模型的 `capability` 字段（用户可编辑、可持久化）优先，
//! 其次内置档案库，最后保守默认档。新增模型时在 `builtin_capability` 中按名称前缀补一条。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// 思考模式能力
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingMode {
    /// 不支持思考
    None,
    /// 始终思考，不可关闭（kimi-k3 / deepseek-reasoner）
    Always,
    /// 可开关（glm-5.x / deepseek-v4 / kimi-k2）
    Toggleable,
}

/// 模型能力档案（对应 models.json 中每个模型的 `capability` 字段）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCapability {
    /// 上下文窗口（输入 + 输出总 token 上限）
    pub context_window: u32,
    /// 单次输出硬上限，调用方预算超过时夹到这里
    pub max_output_tokens: u32,
    /// 预算字段名：max_tokens / max_completion_tokens / max_output_tokens
    pub budget_field: String,
    /// 思考模式
    pub thinking_mode: ThinkingMode,
    /// 该模型支持的思考强度档位（空数组表示不支持调档）
    pub reasoning_effort_options: Vec<String>,
    /// 深度任务默认思考强度（用户可在模型设置中调整）
    pub default_reasoning_effort: String,
    /// 深度任务默认是否开启思考（仅 toggleable 生效；用户可调整）
    pub thinking_enabled: bool,
    /// 思考开关的请求字段名：thinking（对象 {"type": ...}）或 enable_thinking（布尔）
    pub thinking_field: String,
    /// 思考强度的请求字段名：reasoning_effort（顶层）或 reasoning（OpenAI 嵌套对象）
    pub effort_field: String,
    /// 采样参数固定（Kimi 系列显式传 temperature/top_p 会被拒绝）
    pub fixed_sampling: bool,
    /// 官方文档地址
    pub docs_url: String,
    /// 官方文档要点备注（刷新档案时由内置库回填）
    pub notes: String,
    /// 档案更新时间
    pub updated_at: String,
}

/// 任务深度：决定推理参数策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTask {
    /// 深度创作（蒸馏/讨论/写作/优化）：按模型档案的用户配置注入思考强度
    Deep,
    /// 轻量结构任务（评审 JSON / 纪要）：尽量关思考或降到最低档，提速省费
    Light,
}

// ── 内置档案库（官方文档数据） ──

/// 按模型名匹配内置档案（小写前缀匹配；未收录返回 None）
pub fn builtin_capability(model_id: &str) -> Option<ModelCapability> {
    let m = model_id.to_lowercase();
    let cap = |context_window: u32,
               max_output_tokens: u32,
               budget_field: &str,
               thinking_mode: ThinkingMode,
               reasoning_effort_options: &[&str],
               default_reasoning_effort: &str,
               thinking_enabled: bool,
               thinking_field: &str,
               effort_field: &str,
               fixed_sampling: bool,
               docs_url: &str,
               notes: &str| ModelCapability {
        context_window,
        max_output_tokens,
        budget_field: budget_field.to_string(),
        thinking_mode,
        reasoning_effort_options: reasoning_effort_options
            .iter()
            .map(|s| s.to_string())
            .collect(),
        default_reasoning_effort: default_reasoning_effort.to_string(),
        thinking_enabled,
        thinking_field: thinking_field.to_string(),
        effort_field: effort_field.to_string(),
        fixed_sampling,
        docs_url: docs_url.to_string(),
        notes: notes.to_string(),
        updated_at: "2026-08-04".to_string(),
    };

    // ── 智谱 GLM-5 系（thinking 可开关 + reasoning_effort 7 档） ──
    if matches!(
        m.as_str(),
        "glm-5.2" | "glm-5.1" | "glm-5" | "glm-5-turbo" | "glm-5v-turbo"
    ) {
        return Some(cap(
            1_048_576,
            131_072,
            "max_tokens",
            ThinkingMode::Toggleable,
            &["max", "xhigh", "high", "medium", "low", "minimal", "none"],
            "max",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://docs.bigmodel.cn/cn/guide/start/concept-param",
            "GLM-5 系：1M 上下文，最大输出 128K；thinking.type 默认 enabled（模型自动判断），\
             reasoning_effort 支持 max/xhigh/high/medium/low/minimal/none（none/minimal 放弃思考，\
             low/medium 映射 high，xhigh 映射 max）；temperature 可调。",
        ));
    }
    // GLM-4.7：强制思考，不支持 effort 调档
    if m == "glm-4.7" {
        return Some(cap(
            131_072,
            131_072,
            "max_tokens",
            ThinkingMode::Always,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://docs.bigmodel.cn/cn/guide/capabilities/thinking",
            "GLM-4.7：thinking 强制开启不可关闭，不支持 reasoning_effort；最大输出 128K。",
        ));
    }
    // GLM-4.6 / 4.5 系：thinking 可开关，无 effort 调档
    if m.starts_with("glm-4.6") || m.starts_with("glm-4.5") {
        let max_out = if m.starts_with("glm-4.5") { 98_304 } else { 131_072 };
        return Some(cap(
            131_072,
            max_out,
            "max_tokens",
            ThinkingMode::Toggleable,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://docs.bigmodel.cn/cn/guide/start/concept-param",
            "GLM-4.x 系：thinking.type 可开关（默认 enabled）；不支持 reasoning_effort；\
             temperature 可调。",
        ));
    }
    if m.starts_with("glm") {
        return Some(cap(
            131_072,
            16_384,
            "max_tokens",
            ThinkingMode::Toggleable,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://docs.bigmodel.cn/cn/guide/start/concept-param",
            "GLM 其他版本：按 128K 上下文、16K 输出的保守档案处理，建议按具体型号核验。",
        ));
    }

    // ── Moonshot Kimi ──
    if m == "kimi-k3" {
        return Some(cap(
            1_048_576,
            131_072,
            "max_tokens",
            ThinkingMode::Always,
            &["low", "high", "max"],
            "max",
            true,
            "thinking",
            "reasoning_effort",
            true,
            "https://platform.kimi.com/docs/pricing/chat-k3",
            "Kimi K3：1M 上下文，单次最大输出 131072；始终思考不可关闭，\
             reasoning_effort 仅 low/high/max（默认 max）；temperature/top_p 等采样参数固定，\
             显式传入会被拒绝；多轮/工具调用需回传 reasoning_content。",
        ));
    }
    if m.starts_with("kimi-k2") {
        return Some(cap(
            262_144,
            32_768,
            "max_tokens",
            ThinkingMode::Toggleable,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            true,
            "https://platform.kimi.com/docs/pricing/chat-k2",
            "Kimi K2.x：thinking 可开关（默认开），不支持 reasoning_effort 调档；\
             采样参数固定，显式传入会被拒绝。",
        ));
    }
    if m.starts_with("moonshot-v1") {
        let ctx = if m.contains("128") { 131_072 } else { 32_768 };
        return Some(cap(
            ctx,
            8_192,
            "max_tokens",
            ThinkingMode::None,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://platform.kimi.com/docs/pricing/chat-v1",
            "Moonshot V1 系列：经典模型，不支持思考模式。",
        ));
    }

    // ── DeepSeek ──
    if m.starts_with("deepseek-v4-flash") {
        return Some(cap(
            1_048_576,
            393_216,
            "max_tokens",
            ThinkingMode::Toggleable,
            &["low", "high", "max"],
            "high",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://api-docs.deepseek.com/zh-cn/api/create-chat-completion/",
            "DeepSeek V4 Flash：1M 上下文，最大输出 384K（默认 64K）；思考模式默认开启可关闭，\
             reasoning_effort 支持 low/high/max（默认 high）；思考模式下 temperature 等采样参数被忽略。",
        ));
    }
    if m.starts_with("deepseek-v4-pro") || m.starts_with("deepseek-v4") {
        return Some(cap(
            1_048_576,
            393_216,
            "max_tokens",
            ThinkingMode::Toggleable,
            &["high", "max"],
            "high",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://api-docs.deepseek.com/zh-cn/api/create-chat-completion/",
            "DeepSeek V4 Pro：1M 上下文，最大输出 384K；思考模式默认开启，effort 当前仅 high/max\
             （low 映射 high、xhigh 映射 max，预计 2026-08 初支持三档）；思考模式下采样参数被忽略。",
        ));
    }
    if m.starts_with("deepseek-reasoner") || m.starts_with("deepseek-r1") {
        return Some(cap(
            131_072,
            65_536,
            "max_tokens",
            ThinkingMode::Always,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://api-docs.deepseek.com/zh-cn/guides/thinking_mode/",
            "DeepSeek Reasoner/R1：思考模式固定开启，不支持调档；temperature 可传但通常被忽略。",
        ));
    }
    if m.starts_with("deepseek-chat") || m.starts_with("deepseek-v3") {
        return Some(cap(
            131_072,
            8_192,
            "max_tokens",
            ThinkingMode::None,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://api-docs.deepseek.com/zh-cn/quick_start/pricing/",
            "DeepSeek Chat/V3：非思考模型，输出上限 8K。",
        ));
    }
    if m.starts_with("deepseek") {
        return Some(cap(
            131_072,
            8_192,
            "max_tokens",
            ThinkingMode::None,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://api-docs.deepseek.com/",
            "DeepSeek 其他版本：按非思考、8K 输出保守处理，建议按具体型号核验。",
        ));
    }

    // ── OpenAI GPT-5 / o 系列（reasoning 嵌套对象） ──
    if m.starts_with("gpt-5") || m.starts_with("gpt-4.1") {
        return Some(cap(
            400_000,
            128_000,
            "max_output_tokens",
            ThinkingMode::Toggleable,
            &["none", "low", "medium", "high"],
            "medium",
            true,
            "reasoning",
            "reasoning",
            false,
            "https://developers.openai.com/api/docs/models/gpt-5",
            "GPT-5 系：预算字段 max_output_tokens；思考通过嵌套 reasoning.effort 控制\
             （none/low/medium/high），budget 字段 max_output_tokens；temperature 可调。",
        ));
    }
    if m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4") {
        return Some(cap(
            200_000,
            100_000,
            "max_completion_tokens",
            ThinkingMode::Always,
            &["minimal", "low", "medium", "high"],
            "medium",
            true,
            "reasoning",
            "reasoning",
            false,
            "https://developers.openai.com/api/docs/models/",
            "OpenAI o 系列：始终推理，预算字段 max_completion_tokens，\
             reasoning.effort 支持 minimal/low/medium/high；temperature 固定不可调。",
        ));
    }
    if m.starts_with("gpt-") {
        return Some(cap(
            128_000,
            16_384,
            "max_tokens",
            ThinkingMode::None,
            &[],
            "",
            true,
            "reasoning",
            "reasoning",
            false,
            "https://developers.openai.com/api/docs/models/",
            "OpenAI GPT 经典系列：非思考模型，按 128K 上下文、16K 输出保守处理。",
        ));
    }

    // ── Anthropic Claude（后端走 Anthropic Messages API，thinking 暂不注入） ──
    if m.starts_with("claude-") {
        return Some(cap(
            200_000,
            64_000,
            "max_tokens",
            ThinkingMode::Toggleable,
            &[],
            "",
            true,
            "thinking",
            "reasoning_effort",
            false,
            "https://docs.anthropic.com/en/docs/about-claude/models",
            "Claude 4/5 系：200K 上下文，最大输出 64K；扩展思考通过 thinking.budget_tokens 控制，\
             当前 Anthropic 分支未注入思考参数，按默认行为调用。",
        ));
    }

    // ── 阿里云 Qwen3 系（enable_thinking 布尔开关） ──
    if m.starts_with("qwen3") || m.starts_with("qwen-max") || m.starts_with("qwen-plus") {
        return Some(cap(
            262_144,
            65_536,
            "max_tokens",
            ThinkingMode::Toggleable,
            &[],
            "",
            true,
            "enable_thinking",
            "reasoning_effort",
            false,
            "https://www.alibabacloud.com/help/en/model-studio/",
            "Qwen3 系：thinking 通过顶层 enable_thinking 布尔开关，不支持 reasoning_effort；\
             上下文/输出按保守值处理，建议按具体型号核验。",
        ));
    }

    None
}

/// 未收录模型的保守默认档案
fn default_capability() -> ModelCapability {
    ModelCapability {
        context_window: 128_000,
        max_output_tokens: 16_384,
        budget_field: "max_tokens".to_string(),
        thinking_mode: ThinkingMode::None,
        reasoning_effort_options: Vec::new(),
        default_reasoning_effort: String::new(),
        thinking_enabled: true,
        thinking_field: "thinking".to_string(),
        effort_field: "reasoning_effort".to_string(),
        fixed_sampling: false,
        docs_url: String::new(),
        notes: "未收录官方档案：按 OpenAI 兼容保守默认处理（128K 上下文、16K 输出、非思考），\
                可在模型设置中手动校准或补充文档链接。"
            .to_string(),
        updated_at: String::new(),
    }
}

// ── models.json 能力缓存 ──

fn capability_cache() -> &'static RwLock<HashMap<String, ModelCapability>> {
    static CACHE: OnceLock<RwLock<HashMap<String, ModelCapability>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 从 models.json 数组同步能力缓存（list_models / save_models / 刷新档案时调用）
pub fn sync_capabilities(models: &[serde_json::Value]) {
    let mut map = HashMap::new();
    for model in models {
        if let Some(obj) = model.as_object()
            && let Some(mid) = obj.get("model_id").and_then(|v| v.as_str())
            && let Some(cap) = obj.get("capability")
            && let Ok(c) = serde_json::from_value::<ModelCapability>(cap.clone())
        {
            map.insert(mid.to_string(), c);
        }
    }
    if let Ok(mut guard) = capability_cache().write() {
        *guard = map;
    }
}

/// 获取模型能力：磁盘档案（用户可编辑）优先，其次内置档案，最后保守默认
pub fn capability_for(model_id: &str) -> ModelCapability {
    if let Ok(cache) = capability_cache().read()
        && let Some(c) = cache.get(model_id)
    {
        return c.clone();
    }
    builtin_capability(model_id).unwrap_or_else(default_capability)
}

/// 预算翻倍上限：夹在模型硬上限与 65536 之间（避免失控）
pub fn doubled_budget_cap(model_id: &str) -> u32 {
    capability_for(model_id).max_output_tokens.min(65_536)
}

// ── 请求体构建 ──

/// 思考强度档位权重（越低越省思考），用于 Always 模型 Light 任务选最低档
fn effort_rank(effort: &str) -> usize {
    match effort.to_lowercase().as_str() {
        "none" => 0,
        "minimal" => 1,
        "low" => 2,
        "medium" => 3,
        "high" => 4,
        "xhigh" => 5,
        "max" => 6,
        _ => 7,
    }
}

fn lowest_effort(capability: &ModelCapability) -> Option<String> {
    capability
        .reasoning_effort_options
        .iter()
        .min_by_key(|e| effort_rank(e))
        .cloned()
}

/// 按模型能力 + 任务构建 OpenAI 兼容请求体
pub fn plan_request_with_capability(
    model_id: &str,
    capability: &ModelCapability,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    task: LlmTask,
) -> serde_json::Value {
    let budget = max_tokens.min(capability.max_output_tokens);
    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), serde_json::json!(model_id));
    body.insert(
        "messages".to_string(),
        serde_json::json!([
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]),
    );
    body.insert(capability.budget_field.clone(), serde_json::json!(budget));
    body.insert("stream".to_string(), serde_json::json!(true));
    // 采样参数固定的模型不传 temperature，避免被官方拒绝
    if !capability.fixed_sampling {
        body.insert("temperature".to_string(), serde_json::json!(temperature));
    }

    let thinking_field = capability.thinking_field.clone();
    let effort_field = capability.effort_field.clone();
    match (task, capability.thinking_mode) {
        // 深度任务：按用户配置注入思考开关与强度
        (LlmTask::Deep, ThinkingMode::Toggleable) => {
            if capability.thinking_enabled {
                if thinking_field == "enable_thinking" {
                    body.insert("enable_thinking".to_string(), serde_json::json!(true));
                } else {
                    body.insert(
                        thinking_field.clone(),
                        serde_json::json!({ "type": "enabled" }),
                    );
                }
                if !capability.default_reasoning_effort.is_empty() {
                    insert_effort(&mut body, &effort_field, &capability.default_reasoning_effort);
                }
            } else if thinking_field == "enable_thinking" {
                body.insert("enable_thinking".to_string(), serde_json::json!(false));
            } else {
                body.insert(
                    thinking_field.clone(),
                    serde_json::json!({ "type": "disabled" }),
                );
            }
        }
        (LlmTask::Deep, ThinkingMode::Always) => {
            if !capability.default_reasoning_effort.is_empty() {
                insert_effort(&mut body, &effort_field, &capability.default_reasoning_effort);
            }
        }
        (LlmTask::Deep, ThinkingMode::None) => {}
        // 轻量任务：可开关的关掉，始终思考的降到最低档，提速省费
        (LlmTask::Light, ThinkingMode::Toggleable) => {
            if thinking_field == "enable_thinking" {
                body.insert("enable_thinking".to_string(), serde_json::json!(false));
            } else {
                body.insert(
                    thinking_field.clone(),
                    serde_json::json!({ "type": "disabled" }),
                );
            }
        }
        (LlmTask::Light, ThinkingMode::Always) => {
            if let Some(low) = lowest_effort(capability) {
                insert_effort(&mut body, &effort_field, &low);
            }
        }
        (LlmTask::Light, ThinkingMode::None) => {}
    }
    serde_json::Value::Object(body)
}

/// 注入思考强度：顶层字段（reasoning_effort）或 OpenAI 嵌套对象（reasoning.effort）
fn insert_effort(body: &mut serde_json::Map<String, serde_json::Value>, field: &str, effort: &str) {
    if field == "reasoning" {
        body.insert(
            "reasoning".to_string(),
            serde_json::json!({ "effort": effort }),
        );
    } else {
        body.insert(field.to_string(), serde_json::json!(effort));
    }
}

/// 按模型名 + 任务构建请求体（能力自动取自磁盘档案/内置库）
pub fn plan_request(
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
    task: LlmTask,
) -> serde_json::Value {
    let capability = capability_for(model_id);
    plan_request_with_capability(
        model_id,
        &capability,
        system_prompt,
        user_prompt,
        temperature,
        max_tokens,
        task,
    )
}

/// 降级请求体：剔除扩展参数（thinking / reasoning_effort / max_completion_tokens /
/// max_output_tokens / enable_thinking），预算字段回退 max_tokens、采样参数强制带上 ——
/// 用于中转代理不透传扩展参数时的 4xx 重试
pub fn plan_fallback_request(
    model_id: &str,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f64,
    max_tokens: u32,
) -> serde_json::Value {
    let capability = capability_for(model_id);
    let budget = max_tokens.min(capability.max_output_tokens);
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
    use std::sync::Mutex;

    /// 能力缓存是全局状态，测试并行会互相污染。
    /// 所有涉及缓存/请求体构建的测试统一串行执行并在开头清理。
    fn with_clean_cache<T>(f: impl FnOnce() -> T) -> T {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut cache) = capability_cache().write() {
            cache.clear();
        }
        f()
    }

    #[test]
    fn test_kimi_k3_builtin_capability() {
        let c = builtin_capability("kimi-k3").unwrap();
        assert_eq!(c.thinking_mode, ThinkingMode::Always);
        assert_eq!(c.budget_field, "max_tokens");
        assert!(c.fixed_sampling);
        assert_eq!(c.max_output_tokens, 131_072);
        assert_eq!(c.context_window, 1_048_576);
        assert_eq!(c.default_reasoning_effort, "max");
    }

    #[test]
    fn test_glm_and_deepseek_v4_builtin_capability() {
        let glm = builtin_capability("glm-5.2").unwrap();
        assert_eq!(glm.thinking_mode, ThinkingMode::Toggleable);
        assert_eq!(glm.max_output_tokens, 131_072);
        assert_eq!(glm.default_reasoning_effort, "max");
        let flash = builtin_capability("deepseek-v4-flash").unwrap();
        assert_eq!(flash.thinking_mode, ThinkingMode::Toggleable);
        assert_eq!(flash.default_reasoning_effort, "high");
        let pro = builtin_capability("deepseek-v4-pro").unwrap();
        assert_eq!(pro.reasoning_effort_options, vec!["high", "max"]);
    }

    #[test]
    fn test_unknown_model_falls_back_to_default() {
        let c = default_capability();
        assert_eq!(c.thinking_mode, ThinkingMode::None);
        assert_eq!(c.budget_field, "max_tokens");
        assert_eq!(c.max_output_tokens, 16_384);
        assert!(!c.fixed_sampling);
    }

    #[test]
    fn test_disk_capability_overrides_builtin() {
        with_clean_cache(|| {
            let models = vec![serde_json::json!({
                "model_id": "glm-5.2",
                "provider_id": "x",
                "capability": {
                    "context_window": 999,
                    "max_output_tokens": 777,
                    "budget_field": "max_tokens",
                    "thinking_mode": "toggleable",
                    "reasoning_effort_options": ["low", "high"],
                    "default_reasoning_effort": "low",
                    "thinking_enabled": false,
                    "thinking_field": "thinking",
                    "effort_field": "reasoning_effort",
                    "fixed_sampling": false,
                    "docs_url": "",
                    "notes": "",
                    "updated_at": ""
                }
            })];
            sync_capabilities(&models);
            let c = capability_for("glm-5.2");
            assert_eq!(c.context_window, 999);
            assert_eq!(c.default_reasoning_effort, "low");
            assert!(!c.thinking_enabled);
        });
    }

    #[test]
    fn test_plan_request_k3_omits_temperature_and_uses_max_tokens() {
        with_clean_cache(|| {
            let body = plan_request("kimi-k3", "s", "u", 0.7, 16_384, LlmTask::Deep);
            assert!(body.get("temperature").is_none());
            assert_eq!(body["max_tokens"], 16_384);
            assert!(body.get("max_completion_tokens").is_none());
            // Always 模型 Deep 任务按用户配置注入默认强度
            assert_eq!(body["reasoning_effort"], "max");
            assert!(body.get("thinking").is_none());
        });
    }

    #[test]
    fn test_plan_request_light_task_controls_reasoning() {
        with_clean_cache(|| {
            // 可开关模型：Light 注入 thinking=disabled
            let body = plan_request("glm-5.2", "s", "u", 0.3, 8_192, LlmTask::Light);
            assert_eq!(body["thinking"]["type"], "disabled");
            // 永远思考模型：Light 注入最低档
            let body = plan_request("kimi-k3", "s", "u", 0.3, 8_192, LlmTask::Light);
            assert_eq!(body["reasoning_effort"], "low");
            // 非推理模型：不注入任何推理参数
            let body = plan_request("some-model", "s", "u", 0.3, 8_192, LlmTask::Light);
            assert!(body.get("thinking").is_none());
            assert!(body.get("reasoning_effort").is_none());
        });
    }

    #[test]
    fn test_plan_request_deep_uses_user_configured_effort() {
        with_clean_cache(|| {
            // 用户把 GLM 默认思考等级调为 low 且关闭思考
            let models = vec![serde_json::json!({
                "model_id": "glm-5.2",
                "provider_id": "x",
                "capability": {
                    "context_window": 1_048_576,
                    "max_output_tokens": 131_072,
                    "budget_field": "max_tokens",
                    "thinking_mode": "toggleable",
                    "reasoning_effort_options": ["max", "high", "low"],
                    "default_reasoning_effort": "low",
                    "thinking_enabled": false,
                    "thinking_field": "thinking",
                    "effort_field": "reasoning_effort",
                    "fixed_sampling": false,
                    "docs_url": "",
                    "notes": "",
                    "updated_at": ""
                }
            })];
            sync_capabilities(&models);
            let body = plan_request("glm-5.2", "s", "u", 0.3, 8_192, LlmTask::Deep);
            assert_eq!(body["thinking"]["type"], "disabled");
            assert!(body.get("reasoning_effort").is_none());
        });
    }

    #[test]
    fn test_plan_request_gpt5_uses_nested_reasoning() {
        with_clean_cache(|| {
            let body = plan_request("gpt-5.2", "s", "u", 0.7, 8_192, LlmTask::Deep);
            assert_eq!(body["max_output_tokens"], 8_192);
            assert_eq!(body["reasoning"]["effort"], "medium");
        });
    }

    #[test]
    fn test_plan_request_clamps_budget_to_model_cap() {
        with_clean_cache(|| {
            // kimi-k2 上限 32768，超出被夹住
            let body = plan_request("kimi-k2.6", "s", "u", 0.7, 65_536, LlmTask::Deep);
            assert_eq!(body["max_tokens"], 32_768);
        });
    }

    #[test]
    fn test_fallback_request_strips_extensions() {
        with_clean_cache(|| {
            let body = plan_fallback_request("kimi-k3", "s", "u", 0.7, 16_384);
            assert_eq!(body["max_tokens"], 16_384);
            assert!(body.get("max_completion_tokens").is_none());
            assert!(body.get("max_output_tokens").is_none());
            assert!(body.get("thinking").is_none());
            assert!(body.get("reasoning_effort").is_none());
            assert!(body.get("reasoning").is_none());
            // 降级体强制带 temperature（中转按标准 OpenAI 处理）
            assert_eq!(body["temperature"], 0.7);
        });
    }
}
