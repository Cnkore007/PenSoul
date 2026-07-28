/// 插件文件加载器
use pensoul_core::{PensoulError, Result};

use crate::config::PluginConfig;
use std::path::Path;

/// 检查路径是否在允许的范围内（防止目录遍历）
fn validate_path(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(PensoulError::IoError(
            "路径包含 '..' 目录遍历，已拒绝".to_string(),
        ));
    }
    if p.is_absolute() {
        return Err(PensoulError::IoError(
            "不支持绝对路径，请使用相对路径".to_string(),
        ));
    }
    Ok(())
}

/// 从 YAML 文件加载插件配置
pub fn load_from_yaml(path: &str) -> Result<PluginConfig> {
    validate_path(path)?;
    let content = std::fs::read_to_string(path)
        .map_err(|e| PensoulError::IoError(format!("读取文件 {} 失败: {}", path, e)))?;
    serde_yaml::from_str(&content)
        .map_err(|e| PensoulError::SerializationError(format!("YAML 解析失败: {}", e)))
}

/// 从 JSON 文件加载插件配置
pub fn load_from_json(path: &str) -> Result<PluginConfig> {
    validate_path(path)?;
    let content = std::fs::read_to_string(path)
        .map_err(|e| PensoulError::IoError(format!("读取文件 {} 失败: {}", path, e)))?;
    serde_json::from_str(&content)
        .map_err(|e| PensoulError::SerializationError(format!("JSON 解析失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_rejects_parent_dir() {
        let result = validate_path("../../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("目录遍历"));
    }

    #[test]
    fn test_validate_path_rejects_absolute() {
        let result = validate_path("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_allows_relative() {
        let result = validate_path("plugins/my-plugin.yaml");
        assert!(result.is_ok());
    }
}
