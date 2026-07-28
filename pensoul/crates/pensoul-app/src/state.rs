/// 全局应用状态
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

use pensoul_concurrency::ConcurrencyController;
use pensoul_consistency::IncrementalChecker;
use pensoul_cda::ImpactGraph;
use pensoul_harness::HarnessEngine;
use pensoul_llm::ModelRouter;
use pensoul_memory::{ColdMemory, HotMemory, NarrativeMemory, WarmMemory};
use pensoul_plugin::PluginRegistry;
use pensoul_core::{ProjectId, NovelOntology};
use anyhow::Result;

/// 全局应用状态
///
/// 使用 Arc<RwLock<...>> 保证线程安全。
#[derive(Clone)]
pub struct AppState {
    /// 项目目录路径
    pub project_dir: PathBuf,
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
    /// 创建新的应用状态
    pub fn new(project_dir: PathBuf) -> Self {
        let project_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let ontology = NovelOntology::new(project_id, String::new());

        Self {
            project_dir: project_dir.clone(),
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
        }
    }

    /// 从磁盘加载项目
    pub fn load(path: &Path) -> Result<Self> {
        let project_file = path.join("pensoul-project.json");

        if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            let ontology: NovelOntology = serde_json::from_str(&data)?;

            Ok(Self {
                project_dir: path.to_path_buf(),
                ontology: Arc::new(RwLock::new(ontology)),
                harness: Arc::new(RwLock::new(HarnessEngine::new(path))),
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
        } else {
            Ok(Self::new(path.to_path_buf()))
        }
    }

    /// 保存项目到磁盘
    pub fn save(&self) -> Result<()> {
        let ontology = self.ontology.read();
        let data = serde_json::to_string_pretty(&*ontology)?;
        let project_file = self.project_dir.join("pensoul-project.json");
        std::fs::create_dir_all(&self.project_dir)?;
        std::fs::write(&project_file, data)?;
        Ok(())
    }
}
