// project.rs — 项目数据持久化
// JSON 文件 + 原子写（tmp + rename）

use pensoul_domain::ontology::NovelOntology;
use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("项目不存在: {0}")]
    NotFound(String),
}

pub type PersistenceResult<T> = std::result::Result<T, PersistenceError>;

/// 项目 ID 合法性校验：仅允许字母、数字、下划线、连字符
/// 这是防止路径穿越的第二道防线（handler 层也校验）
pub fn is_valid_project_id(project_id: &str) -> bool {
    !project_id.is_empty()
        && project_id.len() <= 64
        && project_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// 项目摘要（作品库列表展示用）
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub title: String,
}

/// 项目数据存储
pub struct ProjectStore {
    base_dir: PathBuf,
}

impl ProjectStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// 项目目录路径
    fn project_dir(&self, project_id: &str) -> PathBuf {
        self.base_dir.join("projects").join(project_id)
    }

    /// 校验项目 ID 并返回目录；非法时返回错误
    fn validated_project_dir(&self, project_id: &str) -> PersistenceResult<PathBuf> {
        if !is_valid_project_id(project_id) {
            return Err(PersistenceError::NotFound(project_id.to_string()));
        }
        Ok(self.project_dir(project_id))
    }

    /// 项目文件路径
    fn project_file(&self, project_id: &str) -> PathBuf {
        self.project_dir(project_id).join("pensoul-project.json")
    }

    /// 保存项目（原子写：tmp + rename）
    pub fn save(&self, ontology: &NovelOntology) -> PersistenceResult<()> {
        let dir = self.validated_project_dir(ontology.project_id.as_str())?;
        std::fs::create_dir_all(&dir)?;

        let file_path = dir.join("pensoul-project.json");
        let tmp_path = dir.join(format!(".tmp-{}", uuid::Uuid::new_v4()));

        let json = serde_json::to_string_pretty(ontology)?;
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, &file_path)?;

        Ok(())
    }

    /// 加载项目
    pub fn load(&self, project_id: &str) -> PersistenceResult<NovelOntology> {
        let dir = self.validated_project_dir(project_id)?;
        let file_path = dir.join("pensoul-project.json");
        if !file_path.exists() {
            return Err(PersistenceError::NotFound(project_id.to_string()));
        }

        let json = std::fs::read_to_string(&file_path)?;
        let ontology: NovelOntology = serde_json::from_str(&json)?;
        Ok(ontology)
    }

    /// 列出所有项目（含中文标题）
    pub fn list_projects(&self) -> Vec<ProjectSummary> {
        let projects_dir = self.base_dir.join("projects");
        if !projects_dir.exists() {
            return Vec::new();
        }

        std::fs::read_dir(&projects_dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false)
            })
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| {
                if !is_valid_project_id(&name) || !self.project_file(&name).exists() {
                    return None;
                }
                // 只读取 title 字段，避免为列表加载整个本体
                let title = std::fs::read_to_string(self.project_file(&name))
                    .ok()
                    .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                    .and_then(|v| v.get("title").and_then(|t| t.as_str()).map(String::from))
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| name.clone());
                Some(ProjectSummary {
                    project_id: name,
                    title,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_domain::id::ProjectId;

    #[test]
    fn rejects_path_traversal_ids() {
        for bad in ["..", "../..", "a/b", "a\\b", ".hidden", "", "a b"] {
            assert!(!is_valid_project_id(bad), "ID 不应通过校验: {bad:?}");
        }
        assert!(is_valid_project_id("my-project_01"));
    }

    #[test]
    fn store_rejects_unsafe_project_id() {
        let base = std::env::temp_dir().join(format!("pensoul-store-test-{}", uuid::Uuid::new_v4()));
        let store = ProjectStore::new(&base);
        let ontology = NovelOntology::new(ProjectId::new("../escape"), "非法");
        assert!(store.save(&ontology).is_err());
        assert!(store.load("../escape").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn list_projects_returns_chinese_title() {
        let base = std::env::temp_dir().join(format!("pensoul-store-test-{}", uuid::Uuid::new_v4()));
        let store = ProjectStore::new(&base);
        store
            .save(&NovelOntology::new(ProjectId::new("yishan"), "移山"))
            .expect("保存项目失败");

        let projects = store.list_projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_id, "yishan");
        assert_eq!(projects[0].title, "移山");

        let _ = std::fs::remove_dir_all(&base);
    }
}
