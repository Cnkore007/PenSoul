/// 插件验证器
use std::collections::HashSet;

use pensoul_core::{PensoulError, Result};

use crate::config::PluginConfig;

const VALID_GATE_TYPES: &[&str] = &["auto", "manual", "conditional"];
const VALID_RUNNER_TYPES: &[&str] = &["local", "delegated"];

/// 插件验证器
pub struct PluginValidator;

impl PluginValidator {
    /// 验证插件配置
    ///
    /// 检查项:
    /// 1. 必填字段非空 (plugin_id, name, version)
    /// 2. 阶段名不重复
    /// 3. gate / runner 类型合法
    /// 4. timeout > 0, max_retries >= 0
    /// 5. local runner 不能使用 delegate_to_expert
    pub fn validate(config: &PluginConfig) -> Result<()> {
        let mut errors = Vec::new();

        // 必填字段
        if config.plugin_id.is_empty() {
            errors.push("缺少必填字段: plugin_id".into());
        }
        if config.name.is_empty() {
            errors.push("缺少必填字段: name".into());
        }
        if config.version.is_empty() {
            errors.push("缺少必填字段: version".into());
        }

        // 阶段验证
        let mut stage_names = HashSet::new();
        for stage in &config.stages {
            if stage.name.is_empty() {
                errors.push("阶段 name 不能为空".into());
            }
            if !stage_names.insert(&stage.name) {
                errors.push(format!("阶段名称重复: {}", stage.name));
            }
            if !VALID_GATE_TYPES.contains(&stage.gate.as_str()) {
                errors.push(format!(
                    "阶段 {}: 无效的 gate 类型 '{}'",
                    stage.name, stage.gate
                ));
            }
            if !VALID_RUNNER_TYPES.contains(&stage.runner.as_str()) {
                errors.push(format!(
                    "阶段 {}: 无效的 runner 类型 '{}'",
                    stage.name, stage.runner
                ));
            }
            if stage.timeout_seconds <= 0 {
                errors.push(format!("阶段 {}: timeout 必须为正数", stage.name));
            }
            if stage.max_retries < 0 {
                errors.push(format!("阶段 {}: max_retries 不能为负", stage.name));
            }
            if stage.runner == "local"
                && stage
                    .allowed_tools
                    .iter()
                    .any(|t| t == "delegate_to_expert")
            {
                errors.push(format!(
                    "阶段 {}: local runner 不能使用 delegate_to_expert",
                    stage.name
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(PensoulError::PluginValidationFailed { errors })
        }
    }
}
