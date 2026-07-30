/// 概念讨论命令 — 调用真实 LLM 执行多 Agent 讨论
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::llm_helper as lh;

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
    // 使用共享辅助模块
    lh::ensure_api_keys_loaded(&state);

    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };

    let mut results = Vec::new();

    for agent in agents.iter().filter(|a| a.enabled) {
        // 解析该模型的供应商 / API Key / API Base
        let (provider_id, api_key, api_base) = match lh::resolve_provider(
            &agent.model,
            &model_to_provider,
            &provider_api_bases,
            &api_keys,
        ) {
            Ok(v) => v,
            Err(msg) => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("⚠️ {}", msg),
                });
                continue;
            }
        };

        // 调用 LLM（自动处理 OpenAI / Anthropic 认证格式）
        let text = match lh::call_llm(
            &lh::ProviderAuth {
                provider_id: &provider_id,
                api_key: &api_key,
                api_base: &api_base,
            },
            &agent.model,
            &agent.prompt,
            &idea_description,
            0.85,
            1024,
        )
        .await
        {
            Ok(t) => t,
            Err(msg) => {
                results.push(AgentDiscussionResult {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    perspective: agent.perspective.clone(),
                    response: format!("❌ {}", msg),
                });
                continue;
            }
        };

        results.push(AgentDiscussionResult {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            perspective: agent.perspective.clone(),
            response: text,
        });
    }

    Ok(results)
}
