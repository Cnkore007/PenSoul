// llm.rs — 全局 LLM 配置管理 API
// 一个模块控制全部供应商/模型配置：新增、更新、删除、拉取、测试、上下文检测

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_infra::llm::{
    LlmClient, LlmConfigStore, LlmMessage, LlmRequest, Provider, ProviderConfig,
    ThinkingMode,
};

/// 供应商默认地址
fn default_base_url(provider: &Provider) -> Option<&'static str> {
    match provider {
        Provider::Openai => Some("https://api.openai.com"),
        Provider::Moonshot => Some("https://api.moonshot.cn"),
        Provider::Deepseek => Some("https://api.deepseek.com"),
        Provider::Anthropic => Some("https://api.anthropic.com"),
        Provider::Custom => None,
    }
}

fn parse_optional_f32(input: Option<String>, field: &str, min: f32, max: f32) -> Result<Option<f32>, ApiError> {
    let Some(value) = input else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let parsed: f32 = value
        .parse()
        .map_err(|_| ApiError::bad_request(format!("{field} 必须是数字")))?;
    if !(min..=max).contains(&parsed) {
        return Err(ApiError::bad_request(format!(
            "{field} 必须在 {min} ~ {max} 之间"
        )));
    }
    Ok(Some(parsed))
}

/// 宽松解析可选整数：空字符串视为未设置
fn parse_optional_u32(
    input: Option<String>,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, ApiError> {
    let Some(value) = input else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let parsed: u32 = value
        .trim()
        .parse()
        .map_err(|_| ApiError::bad_request(format!("{field} 必须是整数")))?;
    if !(min..=max).contains(&parsed) {
        return Err(ApiError::bad_request(format!(
            "{field} 必须在 {min} ~ {max} 之间"
        )));
    }
    Ok(Some(parsed))
}

/// 宽松解析可选布尔：空字符串视为未设置
fn parse_optional_bool(input: Option<String>, field: &str) -> Result<Option<bool>, ApiError> {
    match input.as_deref() {
        None | Some("") => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(other) => Err(ApiError::bad_request(format!(
            "{field} 必须是 true 或 false，收到: {other}"
        ))),
    }
}

#[derive(Deserialize, Default)]
pub struct UpsertConfigParams {
    /// 更新时必填；新增时忽略
    pub id: Option<String>,
    pub name: Option<String>,
    pub provider: Option<String>,
    pub model_id: Option<String>,
    pub base_url: Option<String>,
    /// 新增时必填；更新时留空表示不修改
    pub api_key: Option<String>,
    pub context_window: Option<String>,
    pub max_output_tokens: Option<String>,
    pub thinking_mode: Option<String>,
    pub supports_streaming: Option<String>,
    pub temperature: Option<String>,
    pub top_p: Option<String>,
    pub frequency_penalty: Option<String>,
    pub presence_penalty: Option<String>,
    pub stop_sequences: Option<String>,
    pub json_mode: Option<String>,
    pub thinking_budget: Option<String>,
    pub timeout_seconds: Option<String>,
    pub doc_url: Option<String>,
    pub notes: Option<String>,
    pub enabled: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteConfigParams {
    pub config_id: String,
}

#[derive(Deserialize)]
pub struct SetDefaultParams {
    pub config_id: String,
}

#[derive(Deserialize)]
pub struct TestLlmParams {
    pub config_id: String,
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct ContextCheckParams {
    pub config_id: Option<String>,
    pub text: String,
}

#[derive(Deserialize)]
pub struct PullModelsParams {
    pub config_id: String,
}

#[derive(Deserialize)]
pub struct FetchModelDocParams {
    pub config_id: String,
    pub model_id: String,
    /// 可选：覆盖建议的文档地址
    pub doc_url: Option<String>,
}

fn store(base_dir: &str) -> LlmConfigStore {
    LlmConfigStore::new(base_dir)
}

/// 读取默认 LLM 配置；未配置或无密钥时给出明确指引
pub(crate) fn default_provider(base_dir: &str) -> Result<ProviderConfig, ApiError> {
    let config = store(base_dir).load();
    let provider = config.default_provider().ok_or_else(|| {
        ApiError::bad_request("尚未设置默认 LLM 配置，请先到「设定 → LLM 配置」中配置并设为默认")
    })?;
    if !provider.has_key() {
        return Err(ApiError::bad_request(
            "默认 LLM 配置尚未填写 API Key，请先编辑配置",
        ));
    }
    Ok(provider.clone())
}

/// 构建 LLM 客户端（统一走 llm_helper）
pub(crate) fn llm_client(provider: &ProviderConfig) -> LlmClient {
    LlmClient::with_timeout(
        provider.api_key.clone(),
        provider.base_url.clone(),
        provider.timeout_seconds as u64,
    )
}

/// 结构化输出（json_mode=true）的安全 max_tokens。
///
/// 思考型模型会把 reasoning_tokens 计入 max_tokens（2026-08-14 实测 kimi-k3：
/// completion_tokens 到达上限时 JSON 在闭括号前被截断）。因此所有 JSON 调用点
/// 至少给 4096，且不越过模型配置的最大输出；需要长文本输出的调用点再单独提高下限。
pub(crate) fn structured_output_tokens(
    provider: &ProviderConfig,
    minimum: u32,
    ceiling: u32,
) -> u32 {
    provider.max_output_tokens.clamp(minimum, ceiling)
}

/// 组装统一请求（携带配置中的详细参数）
pub(crate) fn build_llm_request(
    provider: &ProviderConfig,
    messages: Vec<LlmMessage>,
    system: String,
    json_mode: bool,
    max_tokens: u32,
) -> LlmRequest {
    LlmRequest {
        model: provider.model_id.clone(),
        messages,
        max_tokens: Some(max_tokens),
        temperature: provider.temperature,
        top_p: provider.top_p,
        frequency_penalty: provider.frequency_penalty,
        presence_penalty: provider.presence_penalty,
        stop_sequences: provider
            .stop_sequences
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<String>| !v.is_empty()),
        json_mode: Some(json_mode),
        // 思考预算只在配置为 Always/Toggleable 时启用：
        // 2026-08-13 实测修复——glm-5.2 配置 thinking_mode=None 但带 thinking_budget，
        // 旧代码无条件发 thinking:enabled，导致 GLM 被迫思考、偶发吃光 max_tokens
        // 把结构化 JSON 输出截断成残缺内容，蒸馏/事实提取等偶发解析失败。
        thinking_budget: match provider.thinking_mode {
            ThinkingMode::None => None,
            _ => provider.thinking_budget,
        },
        system_prompt: Some(system),
    }
}

/// 拉取全部配置（密钥脱敏）
pub async fn list_configs(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();
    let result = serde_json::json!({
        "default_provider_id": config.default_provider_id,
        "providers": config.providers.iter().map(|p| p.to_public()).collect::<Vec<_>>(),
        "config_file": "_config/llm-config.json",
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 新增配置
pub async fn create_config(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpsertConfigParams>,
) -> Result<String, ApiError> {
    let config = build_config(None, &params)?;

    let base_dir = state.read().await.base_dir.clone();
    let config_store = store(&base_dir);
    let mut all = config_store.load();
    let id = config.id.clone();
    all.providers.push(config);
    config_store
        .save(&all)
        .map_err(|e| ApiError::internal(format!("保存配置失败: {e}")))?;
    Ok(id)
}

/// 更新配置
pub async fn update_config(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpsertConfigParams>,
) -> Result<String, ApiError> {
    let config_id = params
        .id
        .as_deref()
        .ok_or(ApiError::bad_request("缺少 config_id"))?
        .to_string();

    let base_dir = state.read().await.base_dir.clone();
    let config_store = store(&base_dir);
    let mut all = config_store.load();
    let existing = all
        .get_mut(&config_id)
        .ok_or(ApiError::not_found("配置不存在"))?;

    // 部分更新：api_key 留空表示不修改
    let api_key_keep = params
        .api_key
        .as_deref()
        .map(|k| k.trim().is_empty())
        .unwrap_or(true);
    let base = existing.clone();
    let updated = build_config(Some(&base), &params)?;
    if api_key_keep {
        let mut merged = updated;
        merged.api_key = base.api_key.clone();
        *existing = merged;
    } else {
        *existing = updated;
    }

    config_store
        .save(&all)
        .map_err(|e| ApiError::internal(format!("保存配置失败: {e}")))?;
    Ok(config_id)
}

/// 删除配置；若是默认配置则同时清空默认项
pub async fn delete_config(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteConfigParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let config_store = store(&base_dir);
    let mut all = config_store.load();
    let len_before = all.providers.len();
    all.providers.retain(|p| p.id != params.config_id);
    if all.providers.len() == len_before {
        return Err(ApiError::not_found("配置不存在"));
    }
    if all.default_provider_id.as_deref() == Some(params.config_id.as_str()) {
        all.default_provider_id = None;
    }
    config_store
        .save(&all)
        .map_err(|e| ApiError::internal(format!("保存配置失败: {e}")))?;
    Ok("ok".to_string())
}

/// 设置默认配置
pub async fn set_default(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<SetDefaultParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let config_store = store(&base_dir);
    let mut all = config_store.load();
    if all.get(&params.config_id).is_none() {
        return Err(ApiError::not_found("配置不存在"));
    }
    all.default_provider_id = Some(params.config_id);
    config_store
        .save(&all)
        .map_err(|e| ApiError::internal(format!("保存配置失败: {e}")))?;
    Ok("ok".to_string())
}

/// 测试 LLM 连接（使用配置内保存的密钥与详细参数）
pub async fn test_llm(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<TestLlmParams>,
) -> Result<String, ApiError> {
    let prompt = params.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError::bad_request("测试提示词不能为空"));
    }

    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();
    let provider_config = config
        .get(&params.config_id)
        .ok_or(ApiError::not_found("配置不存在"))?;
    if !provider_config.has_key() {
        return Err(ApiError::bad_request(
            "该配置尚未填写 API Key，请先编辑配置",
        ));
    }

    let client = LlmClient::with_timeout(
        provider_config.api_key.clone(),
        provider_config.base_url.clone(),
        provider_config.timeout_seconds as u64,
    );
    let request = LlmRequest {
        model: provider_config.model_id.clone(),
        messages: vec![LlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: Some(provider_config.max_output_tokens),
        temperature: provider_config.temperature,
        top_p: provider_config.top_p,
        frequency_penalty: provider_config.frequency_penalty,
        presence_penalty: provider_config.presence_penalty,
        stop_sequences: provider_config
            .stop_sequences
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
        json_mode: provider_config.json_mode,
        thinking_budget: provider_config.thinking_budget,
        system_prompt: Some("你是 PenSoul 的创作助手，请简短回答。".to_string()),
    };

    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("LLM 调用失败: {e}")))?;

    let result = serde_json::json!({
        "config_id": params.config_id,
        "model": response.model,
        "content": response.content,
        "usage": response.usage,
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 拉取供应商模型列表（使用配置中的地址与密钥，结果缓存到本地）
pub async fn pull_models(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<PullModelsParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();
    let provider_config = config
        .get(&params.config_id)
        .ok_or(ApiError::not_found("配置不存在"))?;

    let models = pensoul_infra::llm::remote::list_remote_models(
        &provider_config.base_url,
        &provider_config.api_key,
        provider_config.provider.as_str(),
    )
    .await
    .map_err(ApiError::bad_request)?;

    // 缓存结果，便于后续离线查看
    let cache_dir = Path::new(&base_dir).join("_config").join("llm-models");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| ApiError::internal(format!("创建缓存目录失败: {e}")))?;
    let cache = serde_json::json!({
        "config_id": params.config_id,
        "fetched_at": chrono::Utc::now().to_rfc3339(),
        "models": models,
    });
    std::fs::write(cache_dir.join(format!("{}.json", params.config_id)), cache.to_string())
        .map_err(|e| ApiError::internal(format!("缓存模型列表失败: {e}")))?;

    Ok(cache.to_string())
}

/// 为拉取的模型定位并抓取官方文档，提取详细参数供导入
pub async fn fetch_model_doc(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<FetchModelDocParams>,
) -> Result<String, ApiError> {
    let model_id = params.model_id.trim();
    if model_id.is_empty() {
        return Err(ApiError::bad_request("缺少模型 ID"));
    }

    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();
    let provider_config = config
        .get(&params.config_id)
        .ok_or(ApiError::not_found("配置不存在"))?;

    // 供应商按模型前缀归一化：custom 中转的 kimi-/claude-/gpt-/deepseek- 也能定位到官方文档
    let inferred_provider = if provider_config.provider == Provider::Custom {
        if model_id.starts_with("kimi") || model_id.starts_with("moonshot") {
            Some("moonshot")
        } else if model_id.starts_with("claude") {
            Some("anthropic")
        } else if model_id.starts_with("gpt") || model_id.starts_with("o1") || model_id.starts_with("o3") {
            Some("openai")
        } else if model_id.starts_with("deepseek") {
            Some("deepseek")
        } else {
            None
        }
    } else {
        Some(provider_config.provider.as_str())
    };

    // 文档根地址：配置 doc_url 优先，否则按供应商默认文档站
    let default_doc = match inferred_provider {
        Some("openai") => Some("https://platform.openai.com/docs/models"),
        Some("moonshot") => Some("https://platform.moonshot.cn/docs"),
        Some("deepseek") => Some("https://api-docs.deepseek.com/zh-cn"),
        Some("anthropic") => Some("https://docs.anthropic.com/en/docs/about-claude/models"),
        _ => None,
    };
    let doc_root = provider_config
        .doc_url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .or(default_doc)
        .ok_or(ApiError::bad_request(format!(
            "无法自动定位模型 {model_id} 的官方文档地址。请在表单中手动填写 doc_url；Kimi 系列可参考 https://platform.moonshot.cn/docs"
        )))?;

    // 1. 通过 llms.txt 索引发现与模型相关的页面
    let mut discovered_pages: Vec<(String, String)> = Vec::new();
    if let Ok(index) = pensoul_infra::llm::remote::fetch_llms_index(doc_root).await {
        let links = pensoul_infra::llm::remote::parse_markdown_links(&index);
        discovered_pages = pensoul_infra::llm::remote::pick_relevant_pages(&links, model_id);
    }

    // 2. 确定主文档地址：显式传入 > 索引发现的模型专属页 > 构造 URL
    let suggested_url = params
        .doc_url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
        .map(|u| u.to_string())
        .or_else(|| {
            discovered_pages
                .first()
                .map(|(_, url)| url.clone())
        })
        .or_else(|| {
            pensoul_infra::llm::remote::suggest_model_doc_url(Some(doc_root), model_id)
        })
        .ok_or(ApiError::bad_request(format!(
            "无法自动定位模型 {model_id} 的官方文档地址，请手动填写 doc_url"
        )))?;

    // 3. 抓取主文档
    let save_dir = Path::new(&base_dir).join("_config").join("llm-docs");
    let doc = pensoul_infra::llm::remote::fetch_doc(&suggested_url, &save_dir, model_id)
        .await
        .map_err(ApiError::bad_request)?;

    // 4. 提取参数（主文档）
    let html = std::fs::read_to_string(&doc.saved_file).unwrap_or_default();
    let mut extracted = pensoul_infra::llm::remote::extract_model_params(&html, model_id);
    extracted.sources.push(pensoul_infra::llm::remote::DocSource {
        title: doc.title.clone(),
        url: suggested_url.clone(),
    });

    // 5. 补充模型列表页：从 Markdown 表格中摘取该模型描述
    for (title, url) in &discovered_pages {
        if *url == suggested_url {
            continue;
        }
        let is_list_page = url.contains("models") && (url.ends_with(".md") || title.contains("模型"));
        if !is_list_page {
            continue;
        }
        if let Ok((content, _)) = pensoul_infra::llm::remote::fetch_raw(url).await {
            if let Some(description) =
                pensoul_infra::llm::remote::find_model_description(&content, model_id)
            {
                extracted.notes.push(format!("模型列表页摘取：{description}"));
                // 模型列表页是结构化权威来源：覆盖主文档的启发式猜测
                extracted.context_window =
                    pensoul_infra::llm::remote::extract_context_from_description(&description)
                        .or(extracted.context_window);
                extracted.thinking_supported = extracted.thinking_supported.or_else(|| {
                    let lower = description.to_lowercase();
                    if lower.contains("思考")
                        || lower.contains("推理")
                        || lower.contains("thinking")
                    {
                        Some(true)
                    } else {
                        None
                    }
                });
            }
            extracted.sources.push(pensoul_infra::llm::remote::DocSource {
                title: title.clone(),
                url: url.clone(),
            });
        }
    }
    extracted.sources.dedup_by(|a, b| a.url == b.url);

    let result = serde_json::json!({
        "suggested_url": suggested_url,
        "doc": doc,
        "params": extracted,
        "discovered_pages": discovered_pages,
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 配置状态概览（不含密钥）
pub async fn get_status(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();
    let result = serde_json::json!({
        "configured_count": config.providers.iter().filter(|p| p.has_key()).count(),
        "total_count": config.providers.len(),
        "has_default": config.default_provider().is_some(),
        "config_file": "_config/llm-config.json",
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 上下文检测：估算 token 占用并对照上下文窗口
pub async fn context_check(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<ContextCheckParams>,
) -> Result<String, ApiError> {
    let text = params.text.trim();
    if text.is_empty() {
        return Err(ApiError::bad_request("检测文本不能为空"));
    }

    let base_dir = state.read().await.base_dir.clone();
    let config = store(&base_dir).load();

    // 上下文窗口必须来自用户配置（内置档案已删除，模型参数以拉取+文档摘取为准）
    let config_id = params
        .config_id
        .as_deref()
        .ok_or(ApiError::bad_request("请选择要检测的配置"))?;
    let provider_config = config
        .get(config_id)
        .ok_or(ApiError::not_found("配置不存在"))?;
    let window = provider_config.context_window;

    let (cjk_chars, other_chars) = count_chars(text);
    let estimated_tokens = cjk_chars + other_chars / 4;
    let input_budget = ((window as f64) * 0.9) as u32;
    let percent = if input_budget == 0 {
        0.0
    } else {
        estimated_tokens as f64 / input_budget as f64 * 100.0
    };

    let result = serde_json::json!({
        "chars": text.chars().count(),
        "cjk_chars": cjk_chars,
        "other_chars": other_chars,
        "estimated_tokens": estimated_tokens,
        "context_window": window,
        "input_budget": input_budget,
        "usage_percent": (percent * 10.0).round() / 10.0,
        "fits": estimated_tokens <= input_budget as usize,
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 由表单参数构造配置（新增用 None，更新用 Some(现有值) 做基础）
fn build_config(
    base: Option<&ProviderConfig>,
    params: &UpsertConfigParams,
) -> Result<ProviderConfig, ApiError> {
    let provider = if let Some(p) = &params.provider {
        Provider::parse(p).ok_or(ApiError::bad_request(format!("未知供应商: {p}")))?
    } else {
        base.map(|b| b.provider.clone())
            .ok_or(ApiError::bad_request("缺少供应商 provider"))?
    };

    let name = params
        .name
        .clone()
        .or_else(|| base.map(|b| b.name.clone()))
        .ok_or(ApiError::bad_request("缺少名称 name"))?;
    if name.trim().is_empty() {
        return Err(ApiError::bad_request("名称不能为空"));
    }

    let model_id = params
        .model_id
        .clone()
        .or_else(|| base.map(|b| b.model_id.clone()))
        .ok_or(ApiError::bad_request("缺少模型 ID model_id"))?;
    if model_id.trim().is_empty() {
        return Err(ApiError::bad_request("模型 ID 不能为空"));
    }

    // 地址：显式传入优先，否则保留旧值，否则用供应商默认
    let base_url = match &params.base_url {
        Some(url) if !url.trim().is_empty() => url.trim().to_string(),
        _ => base
            .map(|b| b.base_url.clone())
            .filter(|u| !u.is_empty())
            .or_else(|| default_base_url(&provider).map(|u| u.to_string()))
            .ok_or(ApiError::bad_request(
                "custom 供应商必须填写 base_url",
            ))?,
    };

    let api_key = match &params.api_key {
        Some(key) => key.trim().to_string(),
        None => base.map(|b| b.api_key.clone()).unwrap_or_default(),
    };

    let context_window = parse_optional_u32(
        params.context_window.clone(),
        "context_window",
        1,
        10_000_000,
    )?
    .or_else(|| base.map(|b| b.context_window))
    .ok_or(ApiError::bad_request("缺少 context_window"))?;
    let max_output_tokens = parse_optional_u32(
        params.max_output_tokens.clone(),
        "max_output_tokens",
        1,
        10_000_000,
    )?
    .or_else(|| base.map(|b| b.max_output_tokens))
    .ok_or(ApiError::bad_request("缺少 max_output_tokens"))?;
    if context_window <= max_output_tokens {
        return Err(ApiError::bad_request(
            "context_window 必须大于 max_output_tokens",
        ));
    }

    let thinking_mode = if let Some(mode) = &params.thinking_mode {
        match mode.as_str() {
            "None" => ThinkingMode::None,
            "Always" => ThinkingMode::Always,
            "Toggleable" => ThinkingMode::Toggleable,
            _ => return Err(ApiError::bad_request(format!("未知思考模式: {mode}"))),
        }
    } else {
        base.map(|b| b.thinking_mode.clone())
            .unwrap_or(ThinkingMode::None)
    };

    Ok(ProviderConfig {
        id: base.map(|b| b.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name,
        provider,
        model_id,
        base_url,
        api_key,
        context_window,
        max_output_tokens,
        thinking_mode,
        supports_streaming: parse_optional_bool(
            params.supports_streaming.clone(),
            "supports_streaming",
        )?
        .or_else(|| base.map(|b| b.supports_streaming))
        .unwrap_or(true),
        temperature: parse_optional_f32(params.temperature.clone(), "temperature", 0.0, 2.0)?
            .or_else(|| base.and_then(|b| b.temperature)),
        top_p: parse_optional_f32(params.top_p.clone(), "top_p", 0.0, 1.0)?
            .or_else(|| base.and_then(|b| b.top_p)),
        frequency_penalty: parse_optional_f32(
            params.frequency_penalty.clone(),
            "frequency_penalty",
            -2.0,
            2.0,
        )?
        .or_else(|| base.and_then(|b| b.frequency_penalty)),
        presence_penalty: parse_optional_f32(
            params.presence_penalty.clone(),
            "presence_penalty",
            -2.0,
            2.0,
        )?
        .or_else(|| base.and_then(|b| b.presence_penalty)),
        stop_sequences: params
            .stop_sequences
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| base.and_then(|b| b.stop_sequences.clone())),
        json_mode: parse_optional_bool(params.json_mode.clone(), "json_mode")?
            .or_else(|| base.and_then(|b| b.json_mode)),
        thinking_budget: parse_optional_u32(
            params.thinking_budget.clone(),
            "thinking_budget",
            0,
            100_000,
        )?
        .or_else(|| base.and_then(|b| b.thinking_budget)),
        timeout_seconds: parse_optional_u32(
            params.timeout_seconds.clone(),
            "timeout_seconds",
            5,
            600,
        )?
        .or_else(|| base.map(|b| b.timeout_seconds))
        .unwrap_or(120),
        doc_url: params
            .doc_url
            .clone()
            .or_else(|| base.and_then(|b| b.doc_url.clone())),
        notes: params
            .notes
            .clone()
            .or_else(|| base.and_then(|b| b.notes.clone())),
        enabled: parse_optional_bool(params.enabled.clone(), "enabled")?
            .or_else(|| base.map(|b| b.enabled))
            .unwrap_or(true),
    })
}

/// 统计 CJK 与其他字符数（用于 token 估算）
fn count_chars(text: &str) -> (usize, usize) {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for c in text.chars() {
        match c as u32 {
            0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0xF900..=0xFAFF
            | 0x3040..=0x30FF
            | 0xAC00..=0xD7AF => cjk += 1,
            _ => other += 1,
        }
    }
    (cjk, other)
}
