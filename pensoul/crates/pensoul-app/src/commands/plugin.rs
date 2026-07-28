/// 插件管理命令
use crate::state::AppState;
use pensoul_plugin::PluginConfig;

/// 列出所有插件
#[tauri::command]
pub async fn list_plugins(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let registry = state.plugin_registry.read();

    let plugins: Vec<serde_json::Value> = registry
        .list_plugins()
        .into_iter()
        .filter_map(|id| {
            registry
                .get(id)
                .and_then(|config| serde_json::to_value(config).ok())
        })
        .collect();

    Ok(plugins)
}

/// 注册插件
#[tauri::command]
pub async fn register_plugin(
    state: tauri::State<'_, AppState>,
    plugin_path: String,
) -> Result<(), String> {
    // 规范化路径，防止目录遍历攻击
    let canonical = std::fs::canonicalize(&plugin_path)
        .map_err(|e| format!("插件路径无效: {}", e))?;

    let data = std::fs::read_to_string(&canonical)
        .map_err(|e| format!("读取插件文件失败: {}", e))?;

    let config: PluginConfig = serde_json::from_str(&data)
        .map_err(|e| format!("解析插件配置失败: {}", e))?;

    let mut registry = state.plugin_registry.write();
    registry
        .register(config)
        .map_err(|e| e.to_string())
}

/// 注销插件
#[tauri::command]
pub async fn unregister_plugin(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    // PluginRegistry 没有 unregister 方法，需要重建
    // 这里简化处理：导出其他插件，清空，重新导入
    let mut registry = state.plugin_registry.write();

    let plugin_ids: Vec<String> = registry
        .list_plugins()
        .into_iter()
        .filter(|id| *id != plugin_id.as_str())
        .map(|s| s.to_string())
        .collect();

    // 导出所有要保留的插件
    let mut configs = Vec::new();
    for id in &plugin_ids {
        if let Ok(json) = registry.export_plugin(id) {
            configs.push(json);
        }
    }

    // 重建注册中心
    *registry = pensoul_plugin::PluginRegistry::new();

    // 导入保留的插件
    for json in configs {
        let _ = registry.import_plugin(&json);
    }

    Ok(())
}
