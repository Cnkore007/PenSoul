/// 全局应用状态 — 支持多项目管理
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;

use pensoul_cda::ImpactGraph;
use pensoul_concurrency::ConcurrencyController;
use pensoul_consistency::IncrementalChecker;
use pensoul_core::{NovelOntology, ProjectId};
use pensoul_harness::HarnessEngine;
use pensoul_llm::ModelRouter;
use pensoul_memory::{ColdMemory, HotMemory, NarrativeMemory, WarmMemory};
use pensoul_plugin::PluginRegistry;

use anyhow::Result;

/// 项目元数据 — 用于列表展示
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectMeta {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub total_chapters: usize,
    pub total_words: u64,
}

/// 项目摘要信息 — 用于 get_project 返回
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    pub project_id: String,
    pub title: String,
    pub total_chapters: usize,
    pub total_words: u64,
    pub volume_count: usize,
}

/// 全局应用状态
///
/// 支持多项目管理。`base_dir` 是所有项目的根目录，
/// 每个项目存储在 `base_dir/<project_id>/` 下。
/// `active_project_id` 标记当前活跃的项目。
#[derive(Clone)]
pub struct AppState {
    /// 所有项目的根目录
    pub base_dir: PathBuf,
    /// 当前活跃的项目 ID
    pub active_project_id: Arc<RwLock<Option<String>>>,
    /// API 密钥存储
    pub api_keys: Arc<RwLock<HashMap<String, String>>>,
    /// 四层本体
    pub ontology: Arc<RwLock<NovelOntology>>,
    /// Harness 引擎
    pub harness: Arc<RwLock<HarnessEngine>>,
    /// 影响图
    pub impact_graph: Arc<RwLock<ImpactGraph>>,
    /// 热记忆
    pub hot_memory: Arc<RwLock<HotMemory>>,
    /// 温记忆
    pub warm_memory: Arc<RwLock<WarmMemory>>,
    /// 冷记忆
    pub cold_memory: Arc<RwLock<ColdMemory>>,
    /// 叙事记忆
    pub narrative_memory: Arc<RwLock<NarrativeMemory>>,
    /// 并发控制器
    pub concurrency: Arc<RwLock<ConcurrencyController>>,
    /// 模型路由器
    pub model_router: Arc<RwLock<ModelRouter>>,
    /// 插件注册中心
    pub plugin_registry: Arc<RwLock<PluginRegistry>>,
    /// 一致性检查器
    pub consistency_checker: Arc<RwLock<IncrementalChecker>>,
}

impl AppState {
    /// 创建新的应用状态（无活跃项目）
    pub fn new(base_dir: PathBuf) -> Self {
        let project_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let ontology = NovelOntology::new(project_id, String::new());

        Self {
            base_dir,
            active_project_id: Arc::new(RwLock::new(None)),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            ontology: Arc::new(RwLock::new(ontology)),
            harness: Arc::new(RwLock::new(HarnessEngine::new(&PathBuf::new()))),
            impact_graph: Arc::new(RwLock::new(ImpactGraph::new())),
            hot_memory: Arc::new(RwLock::new(HotMemory::new(2))),
            warm_memory: Arc::new(RwLock::new(WarmMemory::new())),
            cold_memory: Arc::new(RwLock::new(ColdMemory::new())),
            narrative_memory: Arc::new(RwLock::new(NarrativeMemory::new())),
            concurrency: Arc::new(RwLock::new(ConcurrencyController::new())),
            model_router: Arc::new(RwLock::new(ModelRouter::new())),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
        }
    }

    /// 从磁盘加载指定项目
    pub fn load(base_dir: PathBuf, project_id: &str) -> Result<Self> {
        let project_dir = base_dir.join(project_id);
        let project_file = project_dir.join("pensoul-project.json");

        let ontology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            let pid = ProjectId::new(project_id.to_string());
            NovelOntology::new(pid, String::new())
        };

        Ok(Self {
            base_dir,
            active_project_id: Arc::new(RwLock::new(Some(project_id.to_string()))),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            ontology: Arc::new(RwLock::new(ontology)),
            harness: Arc::new(RwLock::new(HarnessEngine::new(&project_dir))),
            impact_graph: Arc::new(RwLock::new(ImpactGraph::new())),
            hot_memory: Arc::new(RwLock::new(HotMemory::new(2))),
            warm_memory: Arc::new(RwLock::new(WarmMemory::new())),
            cold_memory: Arc::new(RwLock::new(ColdMemory::new())),
            narrative_memory: Arc::new(RwLock::new(NarrativeMemory::new())),
            concurrency: Arc::new(RwLock::new(ConcurrencyController::new())),
            model_router: Arc::new(RwLock::new(ModelRouter::new())),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
        })
    }

    /// 获取当前活跃项目目录路径
    pub fn active_project_dir(&self) -> PathBuf {
        let pid = self.active_project_id.read();
        match pid.as_deref() {
            Some(id) => self.base_dir.join(id),
            None => self.base_dir.join("__no_active_project__"),
        }
    }

    /// 获取配置目录路径 (`_config/`)
    pub fn config_dir(&self) -> PathBuf {
        self.base_dir.join("_config")
    }

    /// 保存当前活跃项目到磁盘
    pub fn save(&self) -> Result<()> {
        let project_dir = self.active_project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let ontology = self.ontology.read();
        let data = serde_json::to_string_pretty(&*ontology)?;
        let project_file = project_dir.join("pensoul-project.json");
        std::fs::write(&project_file, data)?;
        Ok(())
    }

    /// 保存 API 密钥到配置目录
    pub fn save_api_keys(&self) -> Result<()> {
        let config_dir = self.config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let keys = self.api_keys.read();
        let data = serde_json::to_string_pretty(&*keys)?;
        let keys_file = config_dir.join("api-keys.json");
        std::fs::write(&keys_file, data)?;
        Ok(())
    }

    /// 从配置目录加载 API 密钥
    pub fn load_api_keys(&self) -> Result<()> {
        let keys_file = self.config_dir().join("api-keys.json");
        if keys_file.exists() {
            let data = std::fs::read_to_string(&keys_file)?;
            let keys: HashMap<String, String> = serde_json::from_str(&data)?;
            let mut stored = self.api_keys.write();
            *stored = keys;
        }
        Ok(())
    }

    /// 扫描 base_dir 获取所有项目的元数据列表
    pub fn list_project_metas(&self) -> Result<Vec<ProjectMeta>> {
        let mut metas = Vec::new();

        if !self.base_dir.exists() {
            return Ok(metas);
        }

        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过 _config 目录
            if !path.is_dir() {
                continue;
            }
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name == "_config" {
                continue;
            }

            let project_file = path.join("pensoul-project.json");
            if !project_file.exists() {
                continue;
            }

            let data = match std::fs::read_to_string(&project_file) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let ontology: NovelOntology = match serde_json::from_str(&data) {
                Ok(o) => o,
                Err(_) => continue,
            };

            let total_words: u64 = ontology
                .chapters
                .iter()
                .map(|ch| ch.word_count as u64)
                .sum();

            metas.push(ProjectMeta {
                project_id: ontology.project_id.to_string(),
                title: ontology.title.clone(),
                description: String::new(),
                created_at: String::new(),
                updated_at: String::new(),
                total_chapters: ontology.chapters.len(),
                total_words,
            });
        }

        Ok(metas)
    }

    /// 加载指定项目到内存，设为活跃项目并重建引擎组件
    pub fn switch_to_project(&self, project_id: &str) -> Result<()> {
        let project_dir = self.base_dir.join(project_id);
        let project_file = project_dir.join("pensoul-project.json");

        let ontology: NovelOntology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            return Err(anyhow::anyhow!(
                "项目文件不存在: {}",
                project_file.display()
            ));
        };

        // 更新活跃项目 ID
        {
            let mut pid = self.active_project_id.write();
            *pid = Some(project_id.to_string());
        }

        // 更新本体
        {
            let mut ont = self.ontology.write();
            *ont = ontology;
        }

        // 重建 Harness 引擎
        {
            let mut harness = self.harness.write();
            *harness = HarnessEngine::new(&project_dir);
        }

        // 重置其他引擎组件
        {
            *self.impact_graph.write() = ImpactGraph::new();
            *self.hot_memory.write() = HotMemory::new(2);
            *self.warm_memory.write() = WarmMemory::new();
            *self.cold_memory.write() = ColdMemory::new();
            *self.narrative_memory.write() = NarrativeMemory::new();
            *self.concurrency.write() = ConcurrencyController::new();
            *self.consistency_checker.write() = IncrementalChecker::new();
        }

        Ok(())
    }
}
