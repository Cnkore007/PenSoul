/// PenSoul 插件系统
///
/// 提供 YAML 声明式工作流插件的配置、验证、注册和加载功能。
pub mod config;
pub mod loader;
pub mod registry;
pub mod validator;

pub use config::{PluginConfig, PluginStage};
pub use loader::{load_from_json, load_from_yaml};
pub use registry::PluginRegistry;
pub use validator::PluginValidator;

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::PluginId;

    /// 构造合法的插件配置用于测试
    fn valid_plugin_config() -> PluginConfig {
        PluginConfig {
            plugin_id: "test-plugin".into(),
            name: "测试插件".into(),
            version: "1.0.0".into(),
            description: "一个测试插件".into(),
            enabled: true,
            stages: vec![
                PluginStage {
                    name: "stage-a".into(),
                    tool: "llm".into(),
                    gate: "auto".into(),
                    runner: "local".into(),
                    prompt_template: "请生成内容".into(),
                    allowed_tools: vec!["llm".into()],
                    timeout_seconds: 120,
                    max_retries: 2,
                },
                PluginStage {
                    name: "stage-b".into(),
                    tool: "review".into(),
                    gate: "manual".into(),
                    runner: "delegated".into(),
                    prompt_template: String::new(),
                    allowed_tools: vec![],
                    timeout_seconds: 60,
                    max_retries: 1,
                },
            ],
            metadata: serde_json::json!({}),
        }
    }

    // ── 验收 #46: 合法插件注册成功 ──

    #[test]
    fn test_register_valid_plugin() {
        let mut registry = PluginRegistry::new();
        let config = valid_plugin_config();

        let result = registry.register(config);
        assert!(result.is_ok(), "合法插件注册应成功");

        let plugins = registry.list_plugins();
        assert!(plugins.contains(&"test-plugin"));
    }

    #[test]
    fn test_plugin_id_type_safety() {
        let config = valid_plugin_config();
        let plugin_id = PluginId::from(config.plugin_id.as_str());
        assert_eq!(plugin_id.as_str(), "test-plugin");
    }

    // ── 验收 #47: 非法插件正确拒绝 (≥4 个错误) ──

    #[test]
    fn test_reject_invalid_plugin_with_multiple_errors() {
        let mut registry = PluginRegistry::new();
        let config = PluginConfig {
            plugin_id: String::new(), // 空 → 错误 1
            name: String::new(),      // 空 → 错误 2
            version: String::new(),   // 空 → 错误 3
            description: String::new(),
            enabled: true,
            stages: vec![PluginStage {
                name: String::new(), // 空 → 错误 4
                tool: String::new(),
                gate: "invalid_gate".into(),     // 无效 → 错误 5
                runner: "invalid_runner".into(), // 无效 → 错误 6
                prompt_template: String::new(),
                allowed_tools: vec![],
                timeout_seconds: -1, // 负数 → 错误 7
                max_retries: -1,     // 负数 → 错误 8
            }],
            metadata: serde_json::json!({}),
        };

        let result = registry.register(config);
        assert!(result.is_err(), "非法插件应被拒绝");

        match result.unwrap_err() {
            pensoul_core::PensoulError::PluginValidationFailed { errors } => {
                assert!(
                    errors.len() >= 4,
                    "应报告 ≥ 4 个错误，实际: {}",
                    errors.len()
                );
            }
            other => panic!("期望 PluginValidationFailed，实际: {:?}", other),
        }
    }

    // ── 验收 #48: 重复阶段名检测 ──

    #[test]
    fn test_duplicate_stage_names() {
        let mut registry = PluginRegistry::new();
        let config = PluginConfig {
            plugin_id: "dup-test".into(),
            name: "重复阶段测试".into(),
            version: "1.0.0".into(),
            description: String::new(),
            enabled: true,
            stages: vec![
                PluginStage {
                    name: "my-stage".into(),
                    tool: "llm".into(),
                    gate: "auto".into(),
                    runner: "local".into(),
                    prompt_template: String::new(),
                    allowed_tools: vec![],
                    timeout_seconds: 300,
                    max_retries: 3,
                },
                PluginStage {
                    name: "my-stage".into(), // 重复名称
                    tool: "review".into(),
                    gate: "auto".into(),
                    runner: "local".into(),
                    prompt_template: String::new(),
                    allowed_tools: vec![],
                    timeout_seconds: 300,
                    max_retries: 3,
                },
            ],
            metadata: serde_json::json!({}),
        };

        let result = registry.register(config);
        assert!(result.is_err());

        match result.unwrap_err() {
            pensoul_core::PensoulError::PluginValidationFailed { errors } => {
                assert!(
                    errors.iter().any(|e| e.contains("重复")),
                    "错误应包含'重复'关键字，实际: {:?}",
                    errors
                );
            }
            other => panic!("期望 PluginValidationFailed，实际: {:?}", other),
        }
    }

    // ── 验收 #49: 工具白名单一致性 ──

    #[test]
    fn test_local_runner_cannot_delegate_to_expert() {
        let mut registry = PluginRegistry::new();
        let config = PluginConfig {
            plugin_id: "whitelist-test".into(),
            name: "白名单测试".into(),
            version: "1.0.0".into(),
            description: String::new(),
            enabled: true,
            stages: vec![PluginStage {
                name: "bad-stage".into(),
                tool: "llm".into(),
                gate: "auto".into(),
                runner: "local".into(),
                prompt_template: String::new(),
                allowed_tools: vec!["delegate_to_expert".into()],
                timeout_seconds: 300,
                max_retries: 3,
            }],
            metadata: serde_json::json!({}),
        };

        let result = registry.register(config);
        assert!(result.is_err());

        match result.unwrap_err() {
            pensoul_core::PensoulError::PluginValidationFailed { errors } => {
                assert!(
                    errors
                        .iter()
                        .any(|e| e.contains("local runner") && e.contains("delegate_to_expert")),
                    "应检测到 local runner + delegate_to_expert 冲突，实际: {:?}",
                    errors
                );
            }
            other => panic!("期望 PluginValidationFailed，实际: {:?}", other),
        }
    }

    // ── 验收 #50: 插件导出/导入 round-trip ──

    #[test]
    fn test_export_import_roundtrip() {
        let mut registry = PluginRegistry::new();
        let config = valid_plugin_config();
        let stages_count = config.stages.len();

        registry.register(config).unwrap();

        // 导出
        let json = registry.export_plugin("test-plugin").unwrap();
        assert!(json.contains("test-plugin"));

        // 清空后导入
        let mut registry2 = PluginRegistry::new();
        registry2.import_plugin(&json).unwrap();

        // 验证 round-trip
        let restored = registry2.get("test-plugin").unwrap();
        assert_eq!(restored.stages.len(), stages_count);
        assert_eq!(restored.name, "测试插件");
        assert_eq!(restored.version, "1.0.0");
    }

    #[test]
    fn test_export_nonexistent_plugin_fails() {
        let registry = PluginRegistry::new();
        let result = registry.export_plugin("no-such-plugin");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_invalid_json_fails() {
        let mut registry = PluginRegistry::new();
        let result = registry.import_plugin("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_invalid_config_fails() {
        let mut registry = PluginRegistry::new();
        // 缺少必填字段
        let result = registry.import_plugin(r#"{"plugin_id":"","name":"","version":""}"#);
        assert!(result.is_err());
    }

    // ── PluginId 集成测试 ──

    #[test]
    fn test_plugin_id_from_str_and_string() {
        let id1 = PluginId::from("hello");
        let id2 = PluginId::from(String::from("hello"));
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_plugin_id_default_generates_uuid() {
        let id = PluginId::default();
        // Default 现在生成 UUID，非空
        assert!(!id.as_str().is_empty());
        assert_eq!(id.as_str().len(), 36); // UUID 格式 xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    }

    #[test]
    fn test_plugin_id_display() {
        let id = PluginId::new("my-plugin");
        assert_eq!(format!("{}", id), "my-plugin");
    }
}
