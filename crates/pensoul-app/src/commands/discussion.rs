/// 概念讨论命令 — 调用真实 LLM 执行多 Agent 讨论
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub model: String,
    pub prompt: String,
    pub perspective: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct AgentDiscussionResult {
    pub agent_id: String,
    pub agent_name: String,
    pub perspective: String,
    pub response: String,
}

/// 概念讨论 — 为每个启用的 Agent 调用 LLM
#[tauri::command]
pub async fn discuss_concept(
    state: tauri::State<'_, AppState>,
    idea_description: String,
    agents: Vec<AgentConfig>,
) -> Result<Vec<AgentDiscussionResult>, String> {
    // 确保 API Key 已从磁盘加载
    let _ = state.load_api_keys();

    // 从磁盘加载供应商和模型配置
    let saved_providers = load_providers(&state);
    let saved_models = load_models(&state);

    // 构建 model_id → provider_id 映射
    let model_to_provider: HashMap<String, String> = saved_models.iter()
        .filter_map(|m| {
            let model_id = m.get("model_id")?.as_str()?;
            let provider_id = m.get("provider_id")?.as_str()?;
            Some((model_id.to_string(), provider_id.to_string()))
        })
        .collect();

    // 构建 provider_id → api_base 映射
    let provider_api_bases: HashMap<String, String> = saved_providers.iter()
        .filter_map(|p| {
            let pid = p.get("provider_id")?.as_str()?.to_string();
            let api_base = p.get("api_base")?.as_str()?.to_string();
            Some((pid, api_base))
        })
        .collect();

    // 克隆 API keys
    let api_keys: HashMap<String, String> = {
        let keys = state.api_keys.read();
        keys.clone()
    };

    let mut results = Vec::new();

    for agent in agents.iter().filter(|a| a.enabled) {
        // 优先从 models.json 查找供应商；找不到则从模型名回退推断
        let provider_id = model_to_provider.get(&agent.model)
            .map(|s| s.as_str())
            .or_else(|| infer_provider_from_model(&agent.model));

        let provider_id = match provider_id {
            Some(id) => id,
            None => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("⚠️ 无法确定模型「{}」所属的供应商，请先在「模型设置」中配置", agent.model),
                });
                continue;
            }
        };

        // 优先从 providers.json 取 api_base；找不到则硬编码回退
        let api_base = provider_api_bases.get(provider_id)
            .cloned()
            .unwrap_or_else(|| match provider_id {
                "openai" => "https://api.openai.com/v1".to_string(),
                "anthropic" => "https://api.anthropic.com".to_string(),
                "deepseek" => "https://api.deepseek.com".to_string(),
                "moonshot" => "https://api.moonshot.cn/v1".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            });

        let api_key = match api_keys.get(provider_id) {
            Some(key) => key.clone(),
            None => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("⚠️ 未配置「{}」的 API Key，请在「模型设置」中配置", provider_id),
                });
                continue;
            }
        };

        // 构建请求
        let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": agent.model,
            "messages": [
                {
                    "role": "system",
                    "content": agent.prompt
                },
                {
                    "role": "user",
                    "content": idea_description
                }
            ],
            "temperature": 0.85,
            "max_tokens": 1024
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

        let mut request_builder = client.post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        // Anthropic 使用不同的认证头
        if provider_id == "anthropic" {
            request_builder = request_builder
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            request_builder = request_builder
                .header("Authorization", format!("Bearer {}", api_key));
        }

        let response = match request_builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("❌ 请求失败: {}", e),
                });
                continue;
            }
        };

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            results.push(AgentDiscussionResult {
                agent_id: agent.id.clone(),
                agent_name: agent.name.clone(),
                perspective: agent.perspective.clone(),
                response: format!("❌ API 错误 ({}): {}", status, body_text),
            });
            continue;
        }

        // 解析响应
        let json: serde_json::Value = match serde_json::from_str(&body_text) {
            Ok(v) => v,
            Err(e) => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("❌ 解析响应失败: {}", e),
                });
                continue;
            }
        };

        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("(无响应内容)")
            .to_string();

        results.push(AgentDiscussionResult {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            perspective: agent.perspective.clone(),
            response: text,
        });
    }

    Ok(results)
}

/// 从模型名回退推断供应商 ID
fn infer_provider_from_model(model: &str) -> Option<&'static str> {
    if model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3") {
        Some("openai")
    } else if model.starts_with("claude-") {
        Some("anthropic")
    } else if model.starts_with("deepseek") {
        Some("deepseek")
    } else if model.starts_with("moonshot") {
        Some("moonshot")
    } else {
        None
    }
}

/// 从磁盘加载供应商列表（与 llm.rs 逻辑一致）
fn load_providers(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("providers.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                return list;
            }
        }
    }
    Vec::new()
}

/// 从磁盘加载模型列表（与 llm.rs 逻辑一致）
fn load_models(state: &AppState) -> Vec<serde_json::Value> {
    let file = state.config_dir().join("models.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                return list;
            }
        }
    }
    Vec::new()
}
