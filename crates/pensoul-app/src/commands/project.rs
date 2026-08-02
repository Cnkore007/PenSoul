/// 项目管理命令 — 多项目架构
use crate::state::{AppState, ProjectInfo, ProjectMeta, validate_project_id};
use pensoul_core::{NovelOntology, ProjectId};

/// 创建新项目
///
/// 在 `base_dir/<project_id>/` 下初始化项目并持久化。
/// 返回项目 ID 字符串。
#[tauri::command]
pub async fn create_project(
    state: tauri::State<'_, AppState>,
    title: String,
) -> Result<String, String> {
    let project_id = uuid::Uuid::new_v4().to_string();
    let project_dir = state.base_dir.join(&project_id);

    // 创建项目目录
    std::fs::create_dir_all(&project_dir).map_err(|e| format!("创建项目目录失败: {e}"))?;

    // 初始化本体
    let pid = ProjectId::new(project_id.clone());
    let ontology = NovelOntology::new(pid, title);

    // 持久化
    let data = serde_json::to_string_pretty(&ontology).map_err(|e| e.to_string())?;
    let project_file = project_dir.join("pensoul-project.json");
    std::fs::write(&project_file, data).map_err(|e| format!("保存项目文件失败: {e}"))?;

    Ok(project_id)
}

/// 列出所有项目
///
/// 扫描 `base_dir` 下的子目录，读取每个项目的元数据返回。
#[tauri::command]
pub async fn list_projects(state: tauri::State<'_, AppState>) -> Result<Vec<ProjectMeta>, String> {
    state.list_project_metas().map_err(|e| e.to_string())
}

/// 获取单个项目信息
///
/// 读取 `base_dir/<project_id>/pensoul-project.json` 并返回摘要。
#[tauri::command]
pub async fn get_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<ProjectInfo, String> {
    validate_project_id(&project_id).map_err(|e| e.to_string())?;
    let project_file = state
        .base_dir
        .join(&project_id)
        .join("pensoul-project.json");

    if !project_file.exists() {
        return Err(format!("项目不存在: {project_id}"));
    }

    let data =
        std::fs::read_to_string(&project_file).map_err(|e| format!("读取项目文件失败: {e}"))?;
    let ontology: NovelOntology =
        serde_json::from_str(&data).map_err(|e| format!("解析项目文件失败: {e}"))?;

    let total_words: u64 = ontology
        .chapters
        .iter()
        .map(|ch| ch.word_count as u64)
        .sum();

    Ok(ProjectInfo {
        project_id: ontology.project_id.to_string(),
        title: ontology.title,
        total_chapters: ontology.chapters.len(),
        total_words,
        volume_count: ontology.volumes.len(),
    })
}

/// 更新项目标题和描述
///
/// 读取 → 修改 → 回写 `pensoul-project.json`。
#[tauri::command]
pub async fn update_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
    title: String,
    description: String,
) -> Result<(), String> {
    validate_project_id(&project_id).map_err(|e| e.to_string())?;
    let project_file = state
        .base_dir
        .join(&project_id)
        .join("pensoul-project.json");

    if !project_file.exists() {
        return Err(format!("项目不存在: {project_id}"));
    }

    let data =
        std::fs::read_to_string(&project_file).map_err(|e| format!("读取项目文件失败: {e}"))?;
    let mut ontology: NovelOntology =
        serde_json::from_str(&data).map_err(|e| format!("解析项目文件失败: {e}"))?;

    ontology.title = title;
    ontology.description = description;

    let data = serde_json::to_string_pretty(&ontology).map_err(|e| e.to_string())?;
    std::fs::write(&project_file, data).map_err(|e| format!("保存项目文件失败: {e}"))?;

    // 如果正在编辑的就是这个项目，同步更新内存中的本体
    let active = state.active_project_id.read();
    if active.as_deref() == Some(&project_id) {
        drop(active);
        let mut ont = state.ontology.write();
        *ont = ontology;
    }

    Ok(())
}

/// 删除项目
///
/// 递归删除 `base_dir/<project_id>/` 目录。
#[tauri::command]
pub async fn delete_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    validate_project_id(&project_id).map_err(|e| e.to_string())?;
    // 不允许删除当前活跃项目
    {
        let active = state.active_project_id.read();
        if active.as_deref() == Some(&project_id) {
            return Err("不能删除当前打开的项目，请先切换到其他项目".to_string());
        }
    }

    let project_dir = state.base_dir.join(&project_id);
    if !project_dir.exists() {
        return Err(format!("项目不存在: {project_id}"));
    }

    std::fs::remove_dir_all(&project_dir).map_err(|e| format!("删除项目目录失败: {e}"))?;

    Ok(())
}

/// 打开项目
///
/// 加载指定项目到内存，设为活跃项目并重建引擎组件。
#[tauri::command]
pub async fn open_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    state
        .switch_to_project(&project_id)
        .map_err(|e| e.to_string())
}

/// 保存当前项目
#[tauri::command]
pub async fn save_project(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.save().map_err(|e| e.to_string())
}
