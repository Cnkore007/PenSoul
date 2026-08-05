/// LLM 供应商与模型管理命令 — 支持持久化配置
use crate::state::AppState;

/// 默认供应商列表（首次启动时写入配置）
fn built_in_providers() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"provider_id": "openai", "name": "openai", "display_name": "OpenAI", "api_base": "https://api.openai.com/v1", "requires_api_key": true}),
        serde_json::json!({"provider_id": "anthropic", "name": "anthropic", "display_name": "Anthropic", "api_base": "https://api.anthropic.com", "requires_api_key": true}),
        serde_json::json!({"provider_id": "deepseek", "name": "deepseek", "display_name": "DeepSeek", "api_base": "https://api.deepseek.com", "requires_api_key": true}),
        serde_json::json!({"provider_id": "moonshot", "name": "moonshot", "display_name": "Moonshot (Kimi)", "api_base": "https://api.moonshot.cn/v1", "requires_api_key": true}),
        serde_json::json!({"provider_id": "local", "name": "local", "display_name": "本地模型 (Ollama)", "api_base": "http://localhost:11434/v1", "requires_api_key": false}),
    ]
}

/// 默认模型列表（首次启动时写入配置）
fn built_in_models() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"model_id": "gpt-4o", "provider_id": "openai", "display_name": "GPT-4o", "max_tokens": 128000, "supports_tools": true, "cost_per_1k_tokens": 0.005, "avg_quality_score": 0.92, "avg_latency_ms": 1200, "is_available": false, "is_default": false, "api_key_configured": false}),
        serde_json::json!({"model_id": "claude-sonnet-4-20250514", "provider_id": "anthropic", "display_name": "Claude Sonnet 4", "max_tokens": 200000, "supports_tools": true, "cost_per_1k_tokens": 0.003, "avg_quality_score": 0.94, "avg_latency_ms": 1500, "is_available": false, "is_default": false, "api_key_configured": false}),
        serde_json::json!({"model_id": "deepseek-chat", "provider_id": "deepseek", "display_name": "DeepSeek V3", "max_tokens": 64000, "supports_tools": true, "cost_per_1k_tokens": 0.0002, "avg_quality_score": 0.88, "avg_latency_ms": 2000, "is_available": false, "is_default": false, "api_key_configured": false}),
        serde_json::json!({"model_id": "moonshot-v1-8k", "provider_id": "moonshot", "display_name": "Moonshot V1 8K", "max_tokens": 8000, "supports_tools": false, "cost_per_1k_tokens": 0.000012, "avg_quality_score": 0.85, "avg_latency_ms": 1000, "is_available": false, "is_default": false, "api_key_configured": false}),
        serde_json::json!({"model_id": "qwen-2.5-72b", "provider_id": "local", "display_name": "Qwen 2.5 72B (本地)", "max_tokens": 32000, "supports_tools": false, "cost_per_1k_tokens": 0.0, "avg_quality_score": 0.80, "avg_latency_ms": 3000, "is_available": false, "is_default": false, "api_key_configured": false}),
    ]
}

/// 从磁盘加载供应商列表，不存在则用内置默认值
fn load_providers_from_disk(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("providers.json");
    if file.exists()
        && let Ok(data) = std::fs::read_to_string(&file)
        && let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data)
    {
        return list;
    }
    // 首次运行：写入内置默认值
    let defaults = built_in_providers();
    let _ = save_providers_to_disk(state, &defaults);
    defaults
}

/// 保存供应商列表到磁盘
fn save_providers_to_disk(state: &AppState, providers: &[serde_json::Value]) -> Result<(), String> {
    let config_dir = state.config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(providers).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("providers.json"), data).map_err(|e| e.to_string())
}

/// 从磁盘加载模型列表，不存在则用内置默认值
fn load_models_from_disk(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("models.json");
    if file.exists()
        && let Ok(data) = std::fs::read_to_string(&file)
        && let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data)
    {
        return list;
    }
    // 首次运行：写入内置默认值
    let defaults = built_in_models();
    let _ = save_models_to_disk(state, &defaults);
    defaults
}

/// 保存模型列表到磁盘
fn save_models_to_disk(state: &AppState, models: &[serde_json::Value]) -> Result<(), String> {
    let config_dir = state.config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(models).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("models.json"), data).map_err(|e| e.to_string())
}

/// 获取供应商列表（从磁盘读取）
#[tauri::command]
pub async fn list_providers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(load_providers_from_disk(&state))
}

/// 保存供应商列表到磁盘
#[tauri::command]
pub async fn save_providers(
    state: tauri::State<'_, AppState>,
    providers: Vec<serde_json::Value>,
) -> Result<(), String> {
    save_providers_to_disk(&state, &providers)
}

/// 获取模型列表（从磁盘读取 + 标记 api_key 状态）
#[tauri::command]
pub async fn list_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let api_keys = state.api_keys.read();
    let mut models = load_models_from_disk(&state);
    // 为内置库已收录但磁盘缺档案的模型自动补齐能力档案（新增模型即插即用）
    let mut need_save = false;
    for model in models.iter_mut() {
        if let Some(obj) = model.as_object_mut()
            && let Some(model_id) = obj.get("model_id").and_then(|v| v.as_str())
            && !obj.contains_key("capability")
            && let Some(cap) = crate::llm_profile::builtin_capability(model_id)
        {
            obj.insert(
                "capability".to_string(),
                serde_json::to_value(cap).map_err(|e| e.to_string())?,
            );
            need_save = true;
        }
    }
    if need_save {
        let _ = save_models_to_disk(&state, &models);
    }
    crate::llm_profile::sync_capabilities(&models);
    // 根据已存储的密钥更新 api_key_configured 和 is_available
    for model in models.iter_mut() {
        if let Some(obj) = model.as_object_mut()
            && let Some(provider_id) = obj.get("provider_id").and_then(|v| v.as_str())
        {
            let has_key = api_keys.contains_key(provider_id);
            obj.insert(
                "api_key_configured".to_string(),
                serde_json::Value::Bool(has_key),
            );
            // 用户手动开关过的模型（user_managed=true）以磁盘值为准，
            // 避免「关闭的模型切页后又被 api_key 自动点亮」；
            // 未手动管理过的模型跟随 api_key 状态自动启用。
            let user_managed = obj
                .get("user_managed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let stored_enabled = obj
                .get("is_available")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            obj.insert(
                "is_available".to_string(),
                serde_json::Value::Bool(if user_managed { stored_enabled } else { has_key }),
            );
            // 默认模型标记缺省 false
            obj.entry("is_default".to_string()).or_insert(serde_json::Value::Bool(false));
        }
    }
    Ok(models)
}

/// 设置全局默认模型（唯一，切换后其余模型取消默认标记）
#[tauri::command]
pub async fn set_default_model(
    state: tauri::State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let mut models = load_models_from_disk(&state);
    let mut found = false;
    for model in models.iter_mut() {
        if let Some(obj) = model.as_object_mut() {
            let is_target = obj
                .get("model_id")
                .and_then(|v| v.as_str())
                == Some(model_id.as_str());
            if is_target {
                found = true;
            }
            obj.insert(
                "is_default".to_string(),
                serde_json::Value::Bool(is_target),
            );
        }
    }
    if !found {
        return Err(format!("模型 {} 不存在", model_id));
    }
    save_models_to_disk(&state, &models)
}

/// 保存模型列表到磁盘
#[tauri::command]
pub async fn save_models(
    state: tauri::State<'_, AppState>,
    models: Vec<serde_json::Value>,
) -> Result<(), String> {
    let result = save_models_to_disk(&state, &models);
    // 同步能力缓存，后续 LLM 调用立即可用新档案
    crate::llm_profile::sync_capabilities(&models);
    result
}

/// 一键更新模型能力档案（按模型名匹配内置官方文档库）
///
/// - `model_id` 为空时刷新全部模型；否则只刷新指定模型
/// - 刷新会保留用户在设置页调整过的思考等级（default_reasoning_effort）与
///   思考开关（thinking_enabled），其余字段以官方文档为准回填
#[tauri::command]
pub async fn refresh_model_capabilities(
    state: tauri::State<'_, AppState>,
    model_id: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut models = load_models_from_disk(&state);
    let mut updated = 0usize;
    for model in models.iter_mut() {
        let Some(obj) = model.as_object_mut() else {
            continue;
        };
        let Some(mid) = obj.get("model_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(only) = &model_id
            && mid != only
        {
            continue;
        }
        let Some(mut cap) = crate::llm_profile::builtin_capability(mid) else {
            continue;
        };
        // 保留用户已调整的思考等级与开关
        if let Some(old) = obj.get("capability") {
            if let Some(e) = old.get("default_reasoning_effort").and_then(|v| v.as_str()) {
                cap.default_reasoning_effort = e.to_string();
            }
            if let Some(t) = old.get("thinking_enabled").and_then(|v| v.as_bool()) {
                cap.thinking_enabled = t;
            }
        }
        obj.insert(
            "capability".to_string(),
            serde_json::to_value(cap).map_err(|e| e.to_string())?,
        );
        updated += 1;
    }
    if updated == 0 {
        let target = model_id.as_deref().unwrap_or("全部模型");
        return Err(format!(
            "内置档案库尚未收录 {target} 的官方文档，暂无可更新内容。\
             可在 models.json 的 capability 字段手动校准，或反馈给开发者补充档案。"
        ));
    }
    save_models_to_disk(&state, &models)?;
    crate::llm_profile::sync_capabilities(&models);
    Ok(models)
}

/// 保存 API 密钥
#[tauri::command]
pub async fn save_api_key(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    {
        let mut keys = state.api_keys.write();
        keys.insert(provider_id, api_key);
    }
    state.save_api_keys().map_err(|e| e.to_string())
}

/// 从后端读取已保存的 API 密钥
#[tauri::command]
pub async fn load_api_keys(
    state: tauri::State<'_, AppState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    // 先尝试从磁盘重新加载
    let _ = state.load_api_keys();
    let keys = state.api_keys.read();
    Ok(keys.clone())
}
