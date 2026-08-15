// state.rs — AppState 全局状态

use pensoul_domain::ontology::NovelOntology;
use pensoul_graph::EntityGraph;
use pensoul_constraints::ConstraintEngine;
use pensoul_memory::MemoryRetrievalPipeline;
use pensoul_infra::events::EventBus;
use pensoul_infra::persistence::project::is_valid_project_id;
use pensoul_domain::entity::Entity;

/// 上次打开项目的持久化文件（启动时自动恢复，避免重启后前端页面仍开着
/// 项目、但后端内存态被清空导致的「没有打开的项目」）
fn last_project_path(base_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(base_dir)
        .join("_config")
        .join("last-project.json")
}

/// 读取上次打开的项目 id；文件不存在或损坏时返回 None
pub fn load_last_project_id(base_dir: &str) -> Option<String> {
    let text = std::fs::read_to_string(last_project_path(base_dir)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let id = value.get("project_id")?.as_str()?.to_string();
    (!id.is_empty()).then_some(id)
}

/// 持久化上次打开的项目 id
pub fn save_last_project_id(base_dir: &str, project_id: &str) -> Result<(), String> {
    let path = last_project_path(base_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::json!({ "project_id": project_id }).to_string())
        .map_err(|e| e.to_string())
}

/// 清除上次打开项目记录（删除项目时调用，避免启动时恢复已删除项目）
pub fn clear_last_project_id(base_dir: &str) {
    let _ = std::fs::remove_file(last_project_path(base_dir));
}

/// AppState — 全局状态
pub struct AppState {
    /// 当前项目本体（唯一正典）
    pub ontology: Option<NovelOntology>,
    /// 实体图谱
    pub graph: EntityGraph,
    /// 约束引擎
    pub constraints: ConstraintEngine,
    /// 记忆检索管线
    pub memory: MemoryRetrievalPipeline,
    /// 事件总线
    pub events: EventBus,
    /// 数据目录
    pub base_dir: String,
}

impl AppState {
    pub fn new(base_dir: impl Into<String>) -> Self {
        let graph = EntityGraph::new();

        Self {
            ontology: None,
            graph: graph.clone(),
            constraints: ConstraintEngine::new(graph.clone()),
            memory: MemoryRetrievalPipeline::new(graph),
            events: EventBus::new(),
            base_dir: base_dir.into(),
        }
    }

    /// 加载项目
    pub fn load_project(&mut self, project_id: &str) -> Result<(), String> {
        if !is_valid_project_id(project_id) {
            return Err("非法项目 ID（仅允许字母、数字、下划线、连字符）".to_string());
        }
        let store = pensoul_infra::persistence::ProjectStore::new(&self.base_dir);
        let ontology = store
            .load(project_id)
            .map_err(|e| format!("加载项目失败: {}", e))?;

        self.ontology = Some(ontology);
        self.rebuild_derived();
        Ok(())
    }

    /// 关闭当前项目
    pub fn close_project(&mut self) {
        self.ontology = None;
        self.rebuild_derived();
    }

    /// 启动时恢复上次打开的项目；成功返回项目 id，失败返回 None（由调用方忽略）
    pub fn restore_last_project(&mut self) -> Option<String> {
        let id = load_last_project_id(&self.base_dir)?;
        self.load_project(&id).ok().map(|_| id)
    }

    /// 从正典（唯一数据源）重建全部派生状态：图谱、约束引擎、记忆管线
    pub fn rebuild_derived(&mut self) {
        let mut graph = EntityGraph::new();
        if let Some(ontology) = &self.ontology {
            for character in &ontology.characters.characters {
                graph.add_entity(Entity::Character(character.clone()));
            }
            for event in &ontology.world.timeline {
                graph.add_entity(Entity::Event(event.clone()));
            }
            for setting in &ontology.world.locations {
                graph.add_entity(Entity::Setting(setting.clone()));
            }
            for foreshadow in &ontology.narrative.foreshadows {
                graph.add_entity(Entity::Foreshadow(foreshadow.clone()));
            }
            for org in &ontology.world.organizations {
                graph.add_entity(Entity::Organization(org.clone()));
            }
        }

        self.graph = graph.clone();
        self.constraints = ConstraintEngine::new(graph.clone());
        self.memory = MemoryRetrievalPipeline::new(graph);
    }

    /// 保存项目
    pub fn save_project(&self) -> Result<(), String> {
        let ontology = self
            .ontology
            .as_ref()
            .ok_or("没有打开的项目")?;

        let store = pensoul_infra::persistence::ProjectStore::new(&self.base_dir);
        store
            .save(ontology)
            .map_err(|e| format!("保存项目失败: {}", e))
    }

    /// 章节保存后的集成层钩子（on_chapter_saved）：
    /// 用最新派生状态做全量一致性审计，更新章节评分并发布事件
    pub fn on_chapter_saved(&mut self, chapter_id: &str) {
        let report = self.constraints.full_audit();
        let consistency_score = if report.error_count() == 0 { 1.0 } else { 0.5 };

        if let Some(ontology) = &mut self.ontology {
            if let Some(chapter) = ontology
                .chapters
                .iter_mut()
                .find(|c| c.chapter_id.to_string() == chapter_id)
            {
                chapter.consistency_score = consistency_score;
            }
        }

        self.events.emit(
            "chapter_saved",
            serde_json::json!({
                "chapter_id": chapter_id,
                "error_count": report.error_count(),
                "warning_count": report.warning_count(),
                "consistency_score": consistency_score,
            }),
        );
    }
}
