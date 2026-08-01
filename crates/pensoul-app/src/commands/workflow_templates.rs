//! 全局工作流模板 + 项目工作流引用 IPC 命令。
//!
//! 模板是作品库层面的全局资产（`data/workflows/templates.json`），
//! 项目只保存引用（模板 ID + 版本）与项目级覆盖（各环节模型/技法卡），
//! 造化工坊启动时按「项目引用 → 全局模板 → 覆盖」解析实际执行配置。
use crate::state::AppState;
use pensoul_core::workflow::{WorkflowTemplate, builtin_workflow_templates};

/// 列出全部全局工作流模板（从磁盘重新加载，保证跨页面/进程一致）
#[tauri::command]
pub async fn list_workflow_templates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WorkflowTemplate>, String> {
    state.reload_workflow_templates();
    Ok(state.workflow_templates.read().clone())
}

/// 整体保存工作流模板列表。
///
/// 保护规则：
/// - 模板 ID 非空且唯一；
/// - 内置模板不可被删除（缺了自动补回）也不可被改成非内置。
#[tauri::command]
pub async fn save_workflow_templates(
    state: tauri::State<'_, AppState>,
    templates: Vec<WorkflowTemplate>,
) -> Result<(), String> {
    // 校验：ID 非空且唯一
    let mut seen = std::collections::HashSet::new();
    for t in &templates {
        if t.template_id.trim().is_empty() {
            return Err("模板 ID 不能为空".to_string());
        }
        if !seen.insert(t.template_id.clone()) {
            return Err(format!("模板 ID 重复：{}", t.template_id));
        }
    }

    // 内置模板保护：缺失的内置模板自动补回，内置标志不可被篡改
    let builtins = builtin_workflow_templates();
    let mut merged = templates;
    for b in builtins {
        if let Some(existing) = merged.iter_mut().find(|t| t.template_id == b.template_id) {
            existing.builtin = true;
        } else {
            merged.push(b);
        }
    }

    state
        .save_workflow_templates(&merged)
        .map_err(|e| e.to_string())?;
    state.reload_workflow_templates();
    Ok(())
}

/// 恢复内置模板（用户自定义模板保留，冲突时以后台补全规则处理）
#[tauri::command]
pub async fn reset_workflow_templates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WorkflowTemplate>, String> {
    let builtins = builtin_workflow_templates();
    // 保留用户模板：只恢复/补全内置部分
    let mut current = state.workflow_templates.read().clone();
    for b in builtins {
        if let Some(existing) = current.iter_mut().find(|t| t.template_id == b.template_id) {
            *existing = b;
        } else {
            current.push(b);
        }
    }
    state
        .save_workflow_templates(&current)
        .map_err(|e| e.to_string())?;
    state.reload_workflow_templates();
    Ok(state.workflow_templates.read().clone())
}

/// 保存项目工作流引用（模板 ID + 版本 + 覆盖；null = 清除引用）
#[tauri::command]
pub async fn save_workflow_ref(
    state: tauri::State<'_, AppState>,
    config: serde_json::Value,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.workflow_ref = config;
    }
    state.save().map_err(|e| e.to_string())
}

/// 读取项目工作流引用（未配置过返回 null）
#[tauri::command]
pub async fn load_workflow_ref(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    Ok(ontology.workflow_ref.clone())
}
