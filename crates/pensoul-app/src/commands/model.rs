/// 模型路由命令 — 支持持久化偏好设置
use crate::state::AppState;
use pensoul_llm::TaskType;
use std::collections::HashMap;

/// 从磁盘加载模型偏好设置
fn load_preferences_from_disk(state: &AppState) -> HashMap<String, Vec<String>> {
    let file = state.config_dir().join("model-preferences.json");
    if file.exists() {
        if let Ok(data) = std::fs::read_to_string(&file) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<String>>>(&data) {
                return map;
            }
        }
    }
    HashMap::new()
}

/// 保存模型偏好设置到磁盘
fn save_preferences_to_disk(state: &AppState, prefs: &HashMap<String, Vec<String>>) -> Result<(), String> {
    let config_dir = state.config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("model-preferences.json"), data).map_err(|e| e.to_string())
}

/// 将 TaskType 转为字符串 key
fn task_type_key(t: &TaskType) -> String {
    match t {
        TaskType::Outline => "outline",
        TaskType::Drafting => "drafting",
        TaskType::Revision => "revision",
        TaskType::Consistency => "consistency",
        TaskType::Style => "style",
        TaskType::General => "general",
    }
    .to_string()
}

/// 从字符串 key 还原 TaskType
fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "outline" => Ok(TaskType::Outline),
        "drafting" | "draft" => Ok(TaskType::Drafting),
        "revision" | "revise" => Ok(TaskType::Revision),
        "consistency" => Ok(TaskType::Consistency),
        "style" => Ok(TaskType::Style),
        _ => Ok(TaskType::General),
    }
}

/// 启动时将磁盘偏好同步到内存路由器
fn sync_preferences_to_router(state: &AppState) {
    let prefs = load_preferences_from_disk(state);
    let mut router = state.model_router.write();
    for (key, model_ids) in prefs {
        if let Ok(task) = parse_task_type(&key) {
            router.set_task_preference(task, model_ids);
        }
    }
}

/// 获取可用模型列表
#[tauri::command]
pub async fn get_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let router = state.model_router.read();

    let models: Vec<serde_json::Value> = router
        .get_recommendation(TaskType::General)
        .into_iter()
        .filter_map(|m| serde_json::to_value(m).ok())
        .collect();

    Ok(models)
}

/// 设置模型偏好（启用/禁用模型）
#[tauri::command]
pub async fn set_model_preference(
    state: tauri::State<'_, AppState>,
    model_id: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut router = state.model_router.write();

        if enabled {
            let current: Vec<String> = router
                .get_recommendation(TaskType::General)
                .into_iter()
                .map(|m| m.model_id.clone())
                .collect();

            if !current.contains(&model_id) {
                let mut new_prefs = vec![model_id];
                new_prefs.extend(current);
                router.set_task_preference(TaskType::General, new_prefs);
            }
        } else {
            router.remove_from_all_preferences(&model_id);
        }
    }

    // 持久化所有任务类型的偏好
    persist_all_preferences(&state)
}

/// 设置任务模型
#[tauri::command]
pub async fn set_task_model(
    state: tauri::State<'_, AppState>,
    task_type: String,
    model_id: String,
) -> Result<(), String> {
    let task = parse_task_type(&task_type)?;
    {
        let mut router = state.model_router.write();

        let current: Vec<String> = router
            .get_recommendation(task.clone())
            .into_iter()
            .map(|m| m.model_id.clone())
            .collect();

        let mut new_prefs = vec![model_id];
        new_prefs.extend(current);
        router.set_task_preference(task, new_prefs);
    }

    persist_all_preferences(&state)
}

/// 路由模型
#[tauri::command]
pub async fn route_model(
    state: tauri::State<'_, AppState>,
    task_type: String,
) -> Result<serde_json::Value, String> {
    let task = parse_task_type(&task_type)?;
    let mut router = state.model_router.write();

    match router.route(task) {
        Ok(result) => serde_json::to_value(&result).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// 将路由器中所有任务偏好持久化到磁盘
fn persist_all_preferences(state: &AppState) -> Result<(), String> {
    let router = state.model_router.read();
    let task_types = [TaskType::General, TaskType::Outline, TaskType::Drafting, TaskType::Revision, TaskType::Consistency, TaskType::Style];
    let mut prefs: HashMap<String, Vec<String>> = HashMap::new();
    for tt in &task_types {
        let model_ids: Vec<String> = router
            .get_recommendation(tt.clone())
            .into_iter()
            .map(|m| m.model_id.clone())
            .collect();
        if !model_ids.is_empty() {
            prefs.insert(task_type_key(tt), model_ids);
        }
    }
    drop(router);
    save_preferences_to_disk(&state, &prefs)
}

/// 启动时同步偏好（在 state 初始化后调用）
pub fn init_preferences(state: &AppState) {
    sync_preferences_to_router(state);
}
