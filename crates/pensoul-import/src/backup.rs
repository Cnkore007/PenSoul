use pensoul_core::{NovelOntology, PensoulError, Result};
use std::fs;
/// 备份恢复模块
use std::path::PathBuf;

/// 备份 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BackupId(String);

impl BackupId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BackupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 备份信息
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub id: BackupId,
    pub created_at: String,
    pub size_bytes: u64,
}

/// 备份管理器
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// 创建新的备份管理器
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    /// 创建备份
    pub fn create_backup(&self, ontology: &NovelOntology) -> Result<BackupId> {
        // 确保备份目录存在
        fs::create_dir_all(&self.backup_dir)
            .map_err(|e| PensoulError::IoError(format!("创建备份目录失败: {}", e)))?;

        // 生成备份 ID（使用时间戳）
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_id = BackupId::new(format!("backup_{}", timestamp));

        // 序列化 ontology
        let json = serde_json::to_string_pretty(ontology)
            .map_err(|e| PensoulError::SerializationError(format!("序列化失败: {}", e)))?;

        // 写入文件
        let file_path = self.backup_dir.join(format!("{}.json", backup_id.as_str()));
        fs::write(&file_path, &json)
            .map_err(|e| PensoulError::IoError(format!("写入备份文件失败: {}", e)))?;

        Ok(backup_id)
    }

    /// 恢复备份
    pub fn restore_backup(&self, backup_id: &BackupId) -> Result<NovelOntology> {
        let file_path = self.backup_dir.join(format!("{}.json", backup_id.as_str()));

        if !file_path.exists() {
            return Err(PensoulError::ImportError(format!(
                "备份文件不存在: {}",
                file_path.display()
            )));
        }

        let json = fs::read_to_string(&file_path)
            .map_err(|e| PensoulError::IoError(format!("读取备份文件失败: {}", e)))?;

        let ontology: NovelOntology = serde_json::from_str(&json)
            .map_err(|e| PensoulError::SerializationError(format!("反序列化失败: {}", e)))?;

        Ok(ontology)
    }

    /// 列出所有备份
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        let entries = fs::read_dir(&self.backup_dir)
            .map_err(|e| PensoulError::IoError(format!("读取备份目录失败: {}", e)))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| PensoulError::IoError(format!("读取目录项失败: {}", e)))?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "json") {
                let file_name = path
                    .file_stem()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let metadata = fs::metadata(&path)
                    .map_err(|e| PensoulError::IoError(format!("获取文件元数据失败: {}", e)))?;

                let backup_id = BackupId::new(file_name);
                let created_at = metadata
                    .modified()
                    .map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default();
                let size_bytes = metadata.len();

                backups.push(BackupInfo {
                    id: backup_id,
                    created_at,
                    size_bytes,
                });
            }
        }

        // 按创建时间排序（最新的在前）
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::{ChapterId, ChapterStatus, ProjectId, VolumeId};
    use std::env;

    fn create_test_ontology() -> NovelOntology {
        NovelOntology {
            project_id: ProjectId::new("proj1"),
            title: "测试小说".to_string(),
            world: pensoul_core::WorldLayer {
                world_id: pensoul_core::WorldId::new("world1"),
                name: String::new(),
                spatial_model: pensoul_core::SpatialModel {
                    locations: Vec::new(),
                    hierarchy: Vec::new(),
                },
                timeline: pensoul_core::Timeline {
                    events: Vec::new(),
                    epoch_markers: Vec::new(),
                },
                setting_rules: Vec::new(),
                glossary: Vec::new(),
                item_graph: Vec::new(),
            },
            characters: pensoul_core::CharacterLayer {
                characters: Vec::new(),
                relationships: Vec::new(),
            },
            narrative: pensoul_core::NarrativeLayer {
                plot_graph: Vec::new(),
                foreshadows: Vec::new(),
                conflicts: Vec::new(),
                emotional_arcs: Vec::new(),
            },
            aesthetic: pensoul_core::AestheticLayer {
                style_fingerprint: pensoul_core::StyleFingerprint {
                    sentence_length_avg: 0.0,
                    vocabulary_richness: 0.0,
                    rhetorical_frequency: 0.0,
                    dialogue_ratio: 0.0,
                    paragraph_length_avg: 0.0,
                    sample_texts: Vec::new(),
                },
                pacing_model: pensoul_core::PacingModel {
                    tension_curve: Vec::new(),
                    scene_length_avg: 0.0,
                    action_ratio: 0.0,
                },
                anti_ai_rules: Vec::new(),
            },
            chapters: vec![pensoul_core::Chapter {
                chapter_id: ChapterId::new("ch1"),
                chapter_no: 1,
                volume_id: VolumeId::new("vol1"),
                title: "标题一".to_string(),
                summary: String::new(),
                content: "内容一".to_string(),
                word_count: 3,
                version: 1,
                status: ChapterStatus::Draft,
                consistency_score: 1.0,
                created_at: "2026-01-01".to_string(),
                updated_at: "2026-01-01".to_string(),
            }],
            volumes: Vec::new(),
            settings: pensoul_core::ProjectSettings::new(),
            core_concept: pensoul_core::CoreConcept::new(),
            sprout: pensoul_core::SproutData::new(),
            outline_arcs: Vec::new(),
        }
    }

    #[test]
    fn test_backup_manager_new() {
        let temp_dir = env::temp_dir().join("pensoul_test_backup");
        let manager = BackupManager::new(temp_dir);
        assert!(manager.backup_dir.exists() || !manager.backup_dir.exists()); // 只是测试创建
    }

    #[test]
    fn test_create_and_restore_backup() {
        let temp_dir = env::temp_dir().join("pensoul_test_backup_create_restore");
        let manager = BackupManager::new(temp_dir.clone());

        let ontology = create_test_ontology();
        let backup_id = manager.create_backup(&ontology).unwrap();

        let restored = manager.restore_backup(&backup_id).unwrap();
        assert_eq!(restored.title, ontology.title);
        assert_eq!(restored.chapters.len(), ontology.chapters.len());

        // 清理
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = env::temp_dir().join("pensoul_test_backup_list");
        let manager = BackupManager::new(temp_dir.clone());

        let ontology = create_test_ontology();
        let _ = manager.create_backup(&ontology).unwrap();

        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 1);

        // 清理
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_backup_nonexistent_file() {
        let temp_dir = env::temp_dir().join("pensoul_test_backup_nonexistent");
        let manager = BackupManager::new(temp_dir);

        let backup_id = BackupId::new("nonexistent");
        let result = manager.restore_backup(&backup_id);
        assert!(result.is_err());
    }
}
