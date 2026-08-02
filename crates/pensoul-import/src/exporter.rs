/// 导出模块
use pensoul_core::{Chapter, NovelOntology, Result};

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    PlainText,
    Markdown,
}

/// 导出整个小说为 TXT 格式
pub fn export_to_txt(ontology: &NovelOntology) -> Result<String> {
    let mut output = String::new();

    // 添加标题
    output.push_str(&format!("{}\n\n", ontology.title));

    // 按卷和章节导出
    if ontology.volumes.is_empty() {
        // 没有卷，直接导出所有章节
        for chapter in &ontology.chapters {
            output.push_str(&export_chapter(chapter, ExportFormat::PlainText)?);
            output.push_str("\n\n");
        }
    } else {
        // 按卷导出
        for volume in &ontology.volumes {
            output.push_str(&format!("{}\n\n", volume.title));

            // 导出该卷的章节
            for chapter_id in &volume.chapter_ids {
                if let Some(chapter) = ontology.get_chapter(chapter_id) {
                    output.push_str(&export_chapter(chapter, ExportFormat::PlainText)?);
                    output.push_str("\n\n");
                }
            }
        }
    }

    Ok(output)
}

/// 导出单个章节
pub fn export_chapter(chapter: &Chapter, format: ExportFormat) -> Result<String> {
    match format {
        ExportFormat::PlainText => {
            let mut output = String::new();
            output.push_str(&format!("{} {}\n\n", "第", chapter.title));
            output.push_str(&chapter.content);
            Ok(output)
        }
        ExportFormat::Markdown => {
            let mut output = String::new();
            output.push_str(&format!("## {} {}\n\n", "第", chapter.title));
            output.push_str(&chapter.content);
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::{ChapterId, ChapterStatus, ProjectId, VolumeId};

    fn create_test_chapter(id: &str, title: &str, content: &str) -> Chapter {
        Chapter {
            chapter_id: ChapterId::new(id),
            chapter_no: 1,
            volume_id: VolumeId::new("vol1"),
            title: title.to_string(),
            summary: String::new(),
            content: content.to_string(),
            word_count: content.len() as u32,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 1.0,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
            annotations: Vec::new(),
            revisions: Vec::new(),
        }
    }

    #[test]
    fn test_export_chapter_plain_text() {
        let chapter = create_test_chapter("ch1", "标题一", "这是内容");
        let output = export_chapter(&chapter, ExportFormat::PlainText).unwrap();
        assert!(output.contains("第 标题一"));
        assert!(output.contains("这是内容"));
    }

    #[test]
    fn test_export_chapter_markdown() {
        let chapter = create_test_chapter("ch1", "标题一", "这是内容");
        let output = export_chapter(&chapter, ExportFormat::Markdown).unwrap();
        assert!(output.contains("## 第 标题一"));
        assert!(output.contains("这是内容"));
    }

    #[test]
    fn test_export_to_txt() {
        let chapter1 = create_test_chapter("ch1", "标题一", "内容一");
        let chapter2 = create_test_chapter("ch2", "标题二", "内容二");

        let ontology = NovelOntology {
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
            chapters: vec![chapter1, chapter2],
            volumes: Vec::new(),
            settings: pensoul_core::ProjectSettings::new(),
            core_concept: pensoul_core::CoreConcept::new(),
            sprout: pensoul_core::SproutData::new(),
            outline_arcs: Vec::new(),
            workflow_skills: serde_json::Value::Null,
            workflow_ref: serde_json::Value::Null,
            writing_lessons: Vec::new(),
            pending_edit_samples: Vec::new(),
            page_snapshots: Vec::new(),
            page_edit_before: std::collections::HashMap::new(),
        };

        let output = export_to_txt(&ontology).unwrap();
        assert!(output.contains("测试小说"));
        assert!(output.contains("第 标题一"));
        assert!(output.contains("第 标题二"));
    }
}
