/// 概念讨论命令 — 调用真实 LLM 执行多 Agent 讨论
use crate::state::AppState;
use serde::{Deserialize, Serialize};

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
    // 先克隆数据，释放锁后再做异步操作
    let api_keys: std::collections::HashMap<String, String> = {
        let keys = state.api_keys.read();
        keys.clone()
    };
    let mut results = Vec::new();

    for agent in agents.iter().filter(|a| a.enabled) {
        // 根据模型名确定供应商和 API key
        let (provider_id, api_base) = match agent.model.as_str() {
            m if m.starts_with("gpt-") => ("openai", "https://api.openai.com/v1"),
            m if m.starts_with("claude-") => ("anthropic", "https://api.anthropic.com"),
            m if m.starts_with("deepseek") => ("deepseek", "https://api.deepseek.com"),
            m if m.starts_with("moonshot") => ("moonshot", "https://api.moonshot.cn/v1"),
            _ => ("openai", "https://api.openai.com/v1"),
        };

        let api_key = match api_keys.get(provider_id) {
            Some(key) => key.clone(),
            None => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("⚠️ 未配置 {} 的 API Key，请在「模型设置」中配置", provider_id),
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
