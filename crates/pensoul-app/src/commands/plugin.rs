/// 插件管理命令 — 支持持久化
use crate::state::AppState;
use pensoul_plugin::PluginConfig;

/// 从磁盘加载插件列表到注册中心
fn load_plugins_from_disk(state: &AppState) {
    let file = state.config_dir().join("plugins.json");
    if !file.exists() {
        return;
    }
    if let Ok(data) = std::fs::read_to_string(&file) {
        if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
            let mut registry = state.plugin_registry.write();
            for item in list {
                if let Ok(json_str) = serde_json::to_string(&item) {
                    let _ = registry.import_plugin(&json_str);
                }
            }
        }
    }
}

/// 将注册中心所有插件保存到磁盘
fn save_plugins_to_disk(state: &AppState) -> Result<(), String> {
    let config_dir = state.config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let registry = state.plugin_registry.read();
    let plugin_ids: Vec<String> = registry.list_plugins().into_iter().map(|s| s.to_string()).collect();
    let mut configs = Vec::new();
    for id in &plugin_ids {
        if let Ok(json) = registry.export_plugin(id) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&json) {
                configs.push(config);
            }
        }
    }
    drop(registry);

    let data = serde_json::to_string_pretty(&configs).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("plugins.json"), data).map_err(|e| e.to_string())
}

/// 列出所有插件
#[tauri::command]
pub async fn list_plugins(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    // 确保从磁盘加载
    load_plugins_from_disk(&state);

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

/// 安装插件（从 JSON 内容解析配置）
#[tauri::command]
pub async fn install_plugin(
    state: tauri::State<'_, AppState>,
    yaml_content: String,
) -> Result<(), String> {
    let config: PluginConfig = serde_json::from_str(&yaml_content)
        .map_err(|e| format!("解析插件配置失败: {}", e))?;

    {
        let mut registry = state.plugin_registry.write();
        registry
            .register(config)
            .map_err(|e| e.to_string())?;
    }
    save_plugins_to_disk(&state)
}

/// 移除插件
#[tauri::command]
pub async fn remove_plugin(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut registry = state.plugin_registry.write();

    let plugin_ids: Vec<String> = registry
        .list_plugins()
        .into_iter()
        .filter(|id| *id != plugin_id.as_str())
        .map(|s| s.to_string())
        .collect();

    let mut configs = Vec::new();
    for id in &plugin_ids {
        if let Ok(json) = registry.export_plugin(id) {
            configs.push(json);
        }
    }

    *registry = pensoul_plugin::PluginRegistry::new();
    for json in configs {
        let _ = registry.import_plugin(&json);
    }
    drop(registry);

    save_plugins_to_disk(&state)
}

/// 启用/禁用插件
#[tauri::command]
pub async fn toggle_plugin(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let json = {
        let registry = state.plugin_registry.read();
        registry
            .export_plugin(&plugin_id)
            .map_err(|e| e.to_string())?
    };

    let mut config: PluginConfig =
        serde_json::from_str(&json).map_err(|e| format!("解析插件配置失败: {}", e))?;
    config.enabled = enabled;

    {
        let mut registry = state.plugin_registry.write();
        registry
            .register(config)
            .map_err(|e| e.to_string())?;
    }
    save_plugins_to_disk(&state)
}
