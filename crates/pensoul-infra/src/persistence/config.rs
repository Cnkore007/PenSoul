// config.rs — 配置文件读写

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

/// 配置存储
pub struct ConfigStore {
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
        }
    }

    /// 保存 JSON 配置
    pub fn save_json<T: serde::Serialize>(
        &self,
        name: &str,
        data: &T,
    ) -> ConfigResult<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let path = self.config_dir.join(format!("{}.json", name));
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 加载 JSON 配置
    pub fn load_json<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
    ) -> ConfigResult<T> {
        let path = self.config_dir.join(format!("{}.json", name));
        let json = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&json)?;
        Ok(data)
    }

    /// 检查配置是否存在
    pub fn exists(&self, name: &str) -> bool {
        self.config_dir.join(format!("{}.json", name)).exists()
    }
}
