// agent.rs — Agent 注册表与按角色选模型（设计第十一章 / P0b）
// 写作、审校、事实提取、细纲、蒸馏等角色可各自绑定不同的 LLM 配置，
// 未绑定的角色回退到全局默认配置（向后兼容）。
// 配置存储：_config/agent-config.json（全局；字段预留 project_id 供作品级覆盖）。
// 所有 Agent 的 LLM 调用仍统一走 llm_helper（AGENTS.md）。

use axum::extract::{Form, State};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::llm::default_provider;
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_infra::llm::{LlmConfigStore, ProviderConfig};

/// Agent 角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// 写作：章节初稿 / 续写
    Writer,
    /// 审校：章节质量把关 / 改写
    Reviewer,
    /// 事实提取：章节 → 档案事实（全自动）
    Extractor,
    /// 细纲：大纲 → 带标题细纲
    Outliner,
    /// 蒸馏：书籍 → 风格配方
    Distiller,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Writer => "writer",
            Self::Reviewer => "reviewer",
            Self::Extractor => "extractor",
            Self::Outliner => "outliner",
            Self::Distiller => "distiller",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "writer" => Some(Self::Writer),
            "reviewer" => Some(Self::Reviewer),
            "extractor" => Some(Self::Extractor),
            "outliner" => Some(Self::Outliner),
            "distiller" => Some(Self::Distiller),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Writer => "写作",
            Self::Reviewer => "审校",
            Self::Extractor => "事实提取",
            Self::Outliner => "细纲",
            Self::Distiller => "书籍蒸馏",
        }
    }
}

/// Agent 角色-模型绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role_id: String,
    pub display_name: String,
    /// 绑定的 LLM 配置 id；null = 回退全局默认
    pub llm_config_id: Option<String>,
    /// 预留：作品级覆盖（project_id -> 配置 id），当前为空
    #[serde(default)]
    pub project_overrides: std::collections::HashMap<String, String>,
}

/// Agent 配置集合（全局）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigSet {
    pub agents: Vec<AgentConfig>,
}

impl Default for AgentConfigSet {
    fn default() -> Self {
        Self {
            agents: vec![
                AgentConfig {
                    role_id: AgentRole::Writer.as_str().to_string(),
                    display_name: AgentRole::Writer.display_name().to_string(),
                    llm_config_id: None,
                    project_overrides: Default::default(),
                },
                AgentConfig {
                    role_id: AgentRole::Reviewer.as_str().to_string(),
                    display_name: AgentRole::Reviewer.display_name().to_string(),
                    llm_config_id: None,
                    project_overrides: Default::default(),
                },
                AgentConfig {
                    role_id: AgentRole::Extractor.as_str().to_string(),
                    display_name: AgentRole::Extractor.display_name().to_string(),
                    llm_config_id: None,
                    project_overrides: Default::default(),
                },
                AgentConfig {
                    role_id: AgentRole::Outliner.as_str().to_string(),
                    display_name: AgentRole::Outliner.display_name().to_string(),
                    llm_config_id: None,
                    project_overrides: Default::default(),
                },
                AgentConfig {
                    role_id: AgentRole::Distiller.as_str().to_string(),
                    display_name: AgentRole::Distiller.display_name().to_string(),
                    llm_config_id: None,
                    project_overrides: Default::default(),
                },
            ],
        }
    }
}

impl AgentConfigSet {
    fn get_mut(&mut self, role_id: &str) -> Option<&mut AgentConfig> {
        self.agents.iter_mut().find(|a| a.role_id == role_id)
    }
}

/// Agent 配置存储（_config/agent-config.json）
pub struct AgentConfigStore {
    config_dir: std::path::PathBuf,
}

impl AgentConfigStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: Path::new(&base_dir.into()).join("_config"),
        }
    }

    fn path(&self) -> std::path::PathBuf {
        self.config_dir.join("agent-config.json")
    }

    pub fn load(&self) -> AgentConfigSet {
        match std::fs::read_to_string(self.path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => AgentConfigSet::default(),
        }
    }

    pub fn save(&self, config: &AgentConfigSet) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::write(self.path(), serde_json::to_string_pretty(config).unwrap_or_default())
    }
}

/// 解析角色的 LLM 配置：绑定 id → 对应配置；未绑定 → 全局默认
pub(crate) fn resolve(base_dir: &str, role: AgentRole) -> Result<ProviderConfig, ApiError> {
    let store = AgentConfigStore::new(base_dir);
    let agents = store.load();
    let role_id = role.as_str();
    let bound = agents.agents.iter().find(|a| a.role_id == role_id);

    let Some(bound) = bound else {
        return default_provider(base_dir);
    };
    let Some(config_id) = bound.llm_config_id.as_deref() else {
        return default_provider(base_dir);
    };

    let llm_store = LlmConfigStore::new(base_dir);
    let all = llm_store.load();
    match all.get(config_id) {
        Some(p) if p.has_key() => Ok(p.clone()),
        Some(_) => Err(ApiError::bad_request(format!(
            "角色「{}」绑定的 LLM 配置尚未填写 API Key，请先编辑",
            bound.display_name
        ))),
        None => Err(ApiError::bad_request(format!(
            "角色「{}」绑定的 LLM 配置不存在（{}），请重新绑定",
            bound.display_name, config_id
        ))),
    }
}

// ---- API ----

/// 列出全部角色及其绑定的模型
pub async fn list_agent_configs(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let store = AgentConfigStore::new(&base_dir);
    let config = store.load();
    let llm_store = LlmConfigStore::new(&base_dir);
    let llm = llm_store.load();

    let result = serde_json::json!({
        "agents": config.agents.iter().map(|a| {
            serde_json::json!({
                "role_id": a.role_id,
                "display_name": a.display_name,
                "llm_config_id": a.llm_config_id,
                "bound_model": a.llm_config_id.as_ref().and_then(|id| llm.get(id)).map(|p| serde_json::json!({"name": p.name, "model_id": p.model_id})),
                "project_overrides": a.project_overrides,
            })
        }).collect::<Vec<_>>(),
        "config_file": "_config/agent-config.json",
        "note": "null = 使用全局默认 LLM 配置",
    });
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

#[derive(Deserialize)]
pub struct UpdateAgentConfigParams {
    pub role_id: String,
    /// 留空表示回退全局默认
    pub llm_config_id: Option<String>,
}

/// 绑定角色到指定 LLM 配置（或回退默认）
pub async fn update_agent_config(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateAgentConfigParams>,
) -> Result<String, ApiError> {
    if AgentRole::parse(&params.role_id).is_none() {
        return Err(ApiError::bad_request(format!("未知角色: {}", params.role_id)));
    }
    let base_dir = state.read().await.base_dir.clone();

    let llm_store = LlmConfigStore::new(&base_dir);
    let llm = llm_store.load();
    if let Some(id) = params.llm_config_id.as_deref() {
        let id = id.trim();
        if !id.is_empty() && llm.get(id).is_none() {
            return Err(ApiError::bad_request(format!("LLM 配置不存在: {id}")));
        }
    }

    let store = AgentConfigStore::new(&base_dir);
    let mut config = store.load();
    let entry = config
        .get_mut(&params.role_id)
        .ok_or(ApiError::bad_request(format!("角色不存在: {}", params.role_id)))?;
    entry.llm_config_id = match params.llm_config_id.as_deref() {
        Some(id) if !id.trim().is_empty() => Some(id.trim().to_string()),
        _ => None,
    };
    store
        .save(&config)
        .map_err(|e| ApiError::internal(format!("保存 Agent 配置失败: {e}")))?;
    Ok("ok".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_set_has_all_roles() {
        let set = AgentConfigSet::default();
        assert_eq!(set.agents.len(), 5);
        for role in [
            AgentRole::Writer,
            AgentRole::Reviewer,
            AgentRole::Extractor,
            AgentRole::Outliner,
            AgentRole::Distiller,
        ] {
            assert!(
                set.agents.iter().any(|a| a.role_id == role.as_str()),
                "缺角色 {}",
                role.as_str()
            );
        }
    }

    #[test]
    fn role_parse_roundtrip() {
        assert_eq!(AgentRole::parse("writer"), Some(AgentRole::Writer));
        assert_eq!(AgentRole::parse("reviewer"), Some(AgentRole::Reviewer));
        assert_eq!(AgentRole::parse("nope"), None);
        assert_eq!(AgentRole::Writer.as_str(), "writer");
    }

    #[test]
    fn store_roundtrip_preserves_binding() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = AgentConfigStore::new(dir.path().to_string_lossy().to_string());
        let mut set = AgentConfigSet::default();
        set.get_mut("writer").unwrap().llm_config_id = Some("cfg-1".to_string());
        store.save(&set).unwrap();
        let mut loaded = store.load();
        assert_eq!(loaded.get_mut("writer").unwrap().llm_config_id.as_deref(), Some("cfg-1"));
    }
}
