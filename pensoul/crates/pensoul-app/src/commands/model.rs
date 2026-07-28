/// 模型路由命令
use crate::state::AppState;
use pensoul_llm::TaskType;

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

/// 设置任务模型
#[tauri::command]
pub async fn set_task_model(
    state: tauri::State<'_, AppState>,
    task_type: String,
    model_id: String,
) -> Result<(), String> {
    let task = parse_task_type(&task_type)?;
    let mut router = state.model_router.write();

    let current: Vec<String> = router
        .get_recommendation(task.clone())
        .into_iter()
        .map(|m| m.model_id.clone())
        .collect();

    // 添加新模型到列表前面
    let mut new_prefs = vec![model_id];
    new_prefs.extend(current);

    router.set_task_preference(task, new_prefs);
    Ok(())
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

/// 解析任务类型字符串
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
