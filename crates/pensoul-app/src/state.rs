//! 全局应用状态 — 支持多项目管理
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;

use crate::anti_ai::AntiAiRuleConfig;
use pensoul_cda::ImpactGraph;
use pensoul_concurrency::ConcurrencyController;
use pensoul_consistency::IncrementalChecker;
use pensoul_core::workflow::{WorkflowTemplate, builtin_workflow_templates};
use pensoul_core::{NovelOntology, ProjectId};
use pensoul_harness::HarnessEngine;
use pensoul_memory::{EditingMode, MemoryPipeline};

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
    /// 全局工作流模板库（作品库层面定义，项目通过引用 + 覆盖使用）
    pub workflow_templates: Arc<RwLock<Vec<WorkflowTemplate>>>,
    /// 项目文件保存锁：前端并发保存（人物/世界观/设定/概念/萌芽/工作流引用）
    /// 会同时触发 `save()`，原子写的临时文件必须串行，否则 rename 竞态报 os error 2
    save_lock: Arc<Mutex<()>>,
    /// 一致性检查器
    pub consistency_checker: Arc<RwLock<IncrementalChecker>>,
    /// 连写管线控制面（运行/暂停/停止旗标 + 事件缓冲 + 模型选择）
    pub pipeline: Arc<crate::pipeline::PipelineControl>,
    /// 概念讨论控制面（运行旗标 + 事件缓冲，支持页面切换后重连）
    pub discussion: Arc<crate::commands::discussion::DiscussionControl>,
    /// 细纲展开控制面（后台全部展开任务：运行旗标 + 进度 + 取消 + 事件缓冲，
    /// 支持页面切换后重连与一键展开全部章节）
    pub outline_expand: Arc<crate::commands::outline::OutlineExpandControl>,
    /// 蒸馏控制面（书籍/方法论/专家蒸馏共用，支持页面切换后重连）
    pub distills: Arc<crate::commands::expert_distill::DistillControl>,
    /// 反 AI 味规则配置（全局，墨韵页可编辑，注入工作流）
    pub anti_ai: Arc<RwLock<AntiAiRuleConfig>>,
    /// 文风指纹缓存（章节变更后置 None 触发重算）
    pub style_fp: Arc<RwLock<Option<crate::style_fingerprint::StyleFingerprint>>>,
}

impl AppState {
    /// 创建新的应用状态（无活跃项目）
    pub fn new(base_dir: PathBuf) -> Self {
        let project_id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let ontology = NovelOntology::new(project_id, String::new());
        let workflow_templates = load_workflow_templates_from_disk(&base_dir);
        let anti_ai_cfg = crate::anti_ai::load_or_default(&base_dir.join("_config"));

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
            workflow_templates: Arc::new(RwLock::new(workflow_templates)),
            save_lock: Arc::new(Mutex::new(())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
            pipeline: Arc::new(crate::pipeline::PipelineControl::new()),
            discussion: Arc::new(crate::commands::discussion::DiscussionControl::new()),
            outline_expand: Arc::new(crate::commands::outline::OutlineExpandControl::new()),
            distills: Arc::new(crate::commands::expert_distill::DistillControl::new()),
            anti_ai: Arc::new(RwLock::new(anti_ai_cfg)),
            style_fp: Arc::new(RwLock::new(None)),
        }
    }

    /// 从磁盘加载指定项目
    pub fn load(base_dir: PathBuf, project_id: &str) -> Result<Self> {
        validate_project_id(project_id)?;
        let project_dir = base_dir.join(project_id);
        let project_file = project_dir.join("pensoul-project.json");

        let mut ontology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            let pid = ProjectId::new(project_id.to_string());
            NovelOntology::new(pid, String::new())
        };
        // 旧项目章节没有 chapter_no，按数组顺序回填，保证记忆/影响图索引可用
        let backfilled = ontology.backfill_chapter_numbers();
        // 历史「伪章节」（脉络节点误建为章节）还原为情节脉络
        let migrated = ontology.migrate_arc_chapters();

        let state = Self {
            anti_ai: Arc::new(RwLock::new(crate::anti_ai::load_or_default(
                &base_dir.join("_config"),
            ))),
            base_dir: base_dir.clone(),
            active_project_id: Arc::new(RwLock::new(Some(project_id.to_string()))),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            ontology: Arc::new(RwLock::new(ontology)),
            harness: Arc::new(RwLock::new(HarnessEngine::new(&project_dir))),
            impact_graph: Arc::new(RwLock::new(ImpactGraph::new())),
            memory: Arc::new(RwLock::new(new_memory_pipeline())),
            concurrency: Arc::new(RwLock::new(ConcurrencyController::new())),
            workflow_templates: Arc::new(RwLock::new(load_workflow_templates_from_disk(&base_dir))),
            save_lock: Arc::new(Mutex::new(())),
            consistency_checker: Arc::new(RwLock::new(IncrementalChecker::new())),
            pipeline: Arc::new(crate::pipeline::PipelineControl::new()),
            discussion: Arc::new(crate::commands::discussion::DiscussionControl::new()),
            outline_expand: Arc::new(crate::commands::outline::OutlineExpandControl::new()),
            distills: Arc::new(crate::commands::expert_distill::DistillControl::new()),
            style_fp: Arc::new(RwLock::new(None)),
        };
        crate::integration::rebuild_derived_state(&state);
        // 回填/迁移过的数据立即落盘，避免下次启动重复处理
        if backfilled || migrated {
            state.save()?;
        }
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

    /// 全局工作流模板文件路径（`data/workflows/templates.json`，跨项目共享）
    pub fn workflow_templates_file(&self) -> PathBuf {
        self.base_dir.join("workflows").join("templates.json")
    }

    /// 重新从磁盘加载全局工作流模板（模板库页面前端刷新时调用）
    pub fn reload_workflow_templates(&self) {
        let list = load_workflow_templates_from_disk(&self.base_dir);
        *self.workflow_templates.write() = list;
    }

    /// 保存全局工作流模板到磁盘（原子写入）
    pub fn save_workflow_templates(&self, templates: &[WorkflowTemplate]) -> Result<()> {
        let _guard = self.save_lock.lock().unwrap_or_else(|e| e.into_inner());
        let file = self.workflow_templates_file();
        std::fs::create_dir_all(
            file.parent()
                .ok_or_else(|| anyhow!("工作流模板目录路径无效"))?,
        )?;
        let data = serde_json::to_string_pretty(templates)?;
        atomic_write(&file, data.as_bytes())
    }

    /// 一键清空所有项目的项目级覆盖（保留模板引用）。
    ///
    /// 覆盖层退役后，各环节绑定统一由全局模板绑定接管；
    /// 仅对有非空覆盖的项目写盘，返回处理的项目数。
    pub fn clear_all_project_overrides(&self) -> Result<usize> {
        let _guard = self.save_lock.lock().unwrap_or_else(|e| e.into_inner());
        if !self.base_dir.exists() {
            return Ok(0);
        }
        let active = self.active_project_id.read().clone();
        let mut changed = 0usize;
        for entry in std::fs::read_dir(&self.base_dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if dir_name == "_config" {
                continue;
            }
            let project_file = path.join("pensoul-project.json");
            if !project_file.exists() {
                continue;
            }
            let Ok(data) = std::fs::read_to_string(&project_file) else {
                continue;
            };
            let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&data) else {
                continue;
            };
            let Some(wf) = json.get_mut("workflow_ref") else {
                continue;
            };
            if !wf.is_object() {
                continue;
            }
            // 仅当覆盖层非空才写盘
            let has_overrides = wf
                .get("overrides")
                .and_then(|v| v.as_object())
                .map(|o| !o.is_empty())
                .unwrap_or(false);
            if !has_overrides {
                continue;
            }
            if let Some(obj) = wf.as_object_mut() {
                obj.insert("overrides".to_string(), serde_json::json!({}));
            }
            // 当前活跃项目：同步内存，避免后续保存把磁盘结果覆盖回去
            if active.as_deref() == Some(dir_name.as_str()) {
                self.ontology.write().workflow_ref = wf.clone();
            }
            let out = serde_json::to_string_pretty(&json)?;
            atomic_write(&project_file, out.as_bytes())?;
            changed += 1;
        }
        Ok(changed)
    }

    /// 保存当前活跃项目到磁盘（原子写入：临时文件 + rename，
    /// 避免写入中途崩溃导致项目文件损坏）。
    pub fn save(&self) -> Result<()> {
        // 串行化落盘：并发保存共用同一原子写，必须排队，避免 tmp 文件互相踩踏
        let _guard = self.save_lock.lock().unwrap_or_else(|e| e.into_inner());
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
                // 旧数据：简介曾存于 core_concept.inspiration，兼容读取
                description: if ontology.description.is_empty() {
                    ontology.core_concept.inspiration.clone()
                } else {
                    ontology.description.clone()
                },
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

        let mut ontology: NovelOntology = if project_file.exists() {
            let data = std::fs::read_to_string(&project_file)?;
            serde_json::from_str(&data)?
        } else {
            return Err(anyhow!("项目文件不存在: {}", project_file.display()));
        };
        // 旧项目章节没有 chapter_no，按数组顺序回填
        let backfilled = ontology.backfill_chapter_numbers();
        // 历史「伪章节」（脉络节点误建为章节）还原为情节脉络
        let migrated = ontology.migrate_arc_chapters();

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

        // 回填/迁移过的数据立即落盘
        if backfilled || migrated {
            self.save()?;
        }

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

/// 从磁盘加载全局工作流模板；文件缺失或为空时用内置模板播种。
fn load_workflow_templates_from_disk(base_dir: &Path) -> Vec<WorkflowTemplate> {
    let file = base_dir.join("workflows").join("templates.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(list) = serde_json::from_str::<Vec<WorkflowTemplate>>(&data)
                && !list.is_empty()
            {
                return list;
            }
        }
    }
    // 首次启动（或文件损坏）：用内置模板播种并落盘
    let builtins = builtin_workflow_templates();
    if let Some(parent) = file.parent()
        && let Ok(()) = std::fs::create_dir_all(parent)
        && let Ok(data) = serde_json::to_string_pretty(&builtins)
    {
        let _ = atomic_write(&file, data.as_bytes());
    }
    builtins
}

/// 原子写入：先写临时文件，再 rename 覆盖目标。
fn atomic_write(target: &Path, contents: &[u8]) -> Result<()> {
    // 唯一临时文件名（进程号 + 纳秒时间戳）：并发原子写即使不走统一锁也不会互相覆盖
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let tmp = target.with_file_name(format!("{name}.{pid}.{nanos}.tmp"));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}
