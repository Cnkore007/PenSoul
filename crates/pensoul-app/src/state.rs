//! 全局应用状态 — 支持多项目管理
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;

use pensoul_cda::ImpactGraph;
use pensoul_concurrency::ConcurrencyController;
use pensoul_consistency::IncrementalChecker;
use pensoul_core::{NovelOntology, ProjectId};
use pensoul_harness::HarnessEngine;
use pensoul_llm::ModelRouter;
use pensoul_memory::{EditingMode, MemoryPipeline};
use pensoul_plugin::PluginRegistry;

use anyhow::{Result, anyhow};

/// 记忆管道的热记忆窗口大小（章）
pub const MEMORY_WINDOW_SIZE: usize = 2;
/// 记忆管道的总 token 预算
pub const MEMORY_TOTAL_BUDGET: usize = 8000;

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

/// 校验项目 ID 是否安全（防目录遍历）。
///
/// 项目 ID 会拼接到文件路径中，必须是单个安全路径组件：
/// 非空、不含路径分隔符、不含 `..`。
pub fn validate_project_id(project_id: &str) -> Result<()> {
    let invalid = project_id.is_empty()
        || project_id.contains('/')
        || project_id.contains('\\')
        || project_id.contains("..")
        || project_id == "_config";
    if invalid {
        Err(anyhow!("非法的项目 ID: {project_id:?}"))
    } else {
        Ok(())
    }
}

/// 全局应用状态
///
/// 支持多项目管理。`base_dir` 是所有项目的根目录，
/// 每个项目存储在 `base_dir/<project_id>/` 下。
/// `active_project_id` 标记当前活跃的项目。
///
/// 派生子系统（记忆管道 / 影响图 / 一致性检查器 / 并发版本）
/// 由 `crate::integration` 在数据变更时重建与增量更新。
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
    /// 记忆管道（热/温/冷/叙事四层记忆的统一入口）
    pub memory: Arc<RwLock<MemoryPipeline>>,
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
            harness: Arc::new(RwLock::new(HarnessEngine::new(&scratch_harness_dir(
                &base_dir,
            )))),
            base_dir,
            active_project_id: Arc::new(RwLock::new(None)),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            ontology: Arc::new(RwLock::new(ontology)),
            impact_graph: Arc::new(RwLock::new(ImpactGraph::new())),
            memory: Arc::new(RwLock::new(new_memory_pipeline())),
            concurrency: Arc::new(RwLock::new(ConcurrencyController::new())),
            model_router: Arc::new(RwLock::new(ModelRouter::new())),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
        }
    }

    /// 从磁盘加载指定项目
    pub fn load(base_dir: PathBuf, project_id: &str) -> Result<Self> {
        validate_project_id(project_id)?;
        let project_dir = base_dir.join(project_id);
        let project_file = project_dir.join("pensoul-project.json");

        let ontology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            let pid = ProjectId::new(project_id.to_string());
            NovelOntology::new(pid, String::new())
        };

        let state = Self {
            base_dir,
            active_project_id: Arc::new(RwLock::new(Some(project_id.to_string()))),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            ontology: Arc::new(RwLock::new(ontology)),
            harness: Arc::new(RwLock::new(HarnessEngine::new(&project_dir))),
            impact_graph: Arc::new(RwLock::new(ImpactGraph::new())),
            memory: Arc::new(RwLock::new(new_memory_pipeline())),
            concurrency: Arc::new(RwLock::new(ConcurrencyController::new())),
            model_router: Arc::new(RwLock::new(ModelRouter::new())),
            plugin_registry: Arc::new(RwLock::new(PluginRegistry::new())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
        };
        crate::integration::rebuild_derived_state(&state);
        Ok(state)
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

    /// 保存当前活跃项目到磁盘（原子写入：临时文件 + rename，
    /// 避免写入中途崩溃导致项目文件损坏）。
    pub fn save(&self) -> Result<()> {
        let project_dir = self.active_project_dir();
        std::fs::create_dir_all(&project_dir)?;

        let ontology = self.ontology.read();
        let data = serde_json::to_string_pretty(&*ontology)?;
        atomic_write(&project_dir.join("pensoul-project.json"), data.as_bytes())
    }

    /// 保存 API 密钥到配置目录。
    ///
    /// TODO(security): 目前为受限权限（0600）的 JSON 文件，
    /// 后续应迁移到操作系统 keychain（keyring crate）。
    pub fn save_api_keys(&self) -> Result<()> {
        let config_dir = self.config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let keys = self.api_keys.read();
        let data = serde_json::to_string_pretty(&*keys)?;
        let keys_file = config_dir.join("api-keys.json");
        std::fs::write(&keys_file, data)?;

        // 密钥文件仅属主可读写
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&keys_file, std::fs::Permissions::from_mode(0o600))?;
        }
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

    /// 加载指定项目到内存，设为活跃项目并重建引擎组件与派生状态
    pub fn switch_to_project(&self, project_id: &str) -> Result<()> {
        validate_project_id(project_id)?;
        let project_dir = self.base_dir.join(project_id);
        let project_file = project_dir.join("pensoul-project.json");

        let ontology: NovelOntology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            return Err(anyhow!("项目文件不存在: {}", project_file.display()));
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

        // 重置派生引擎组件
        {
            *self.impact_graph.write() = ImpactGraph::new();
            *self.memory.write() = new_memory_pipeline();
            *self.concurrency.write() = ConcurrencyController::new();
            *self.consistency_checker.write() = IncrementalChecker::new();
        }

        // 从本体全量重建派生状态（记忆/影响图/一致性状态/并发版本）
        crate::integration::rebuild_derived_state(self);

        Ok(())
    }
}

/// 创建记忆管道实例（统一窗口与预算配置）。
fn new_memory_pipeline() -> MemoryPipeline {
    MemoryPipeline::new(
        MEMORY_WINDOW_SIZE,
        EditingMode::Drafting,
        MEMORY_TOTAL_BUDGET,
    )
}

/// 无活跃项目时 Harness 引擎的暂存目录。
///
/// 历史上这里传的是空路径，导致引擎在进程的当前工作目录里
/// 创建 `.harness/` 并写 WAL —— 隐蔽的副作用。
fn scratch_harness_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("_config").join("harness_scratch")
}

/// 原子写入：先写临时文件，再 rename 覆盖目标。
fn atomic_write(target: &Path, contents: &[u8]) -> Result<()> {
    let tmp = target.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}
