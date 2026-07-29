/// 四层本体 NovelOntology 类型定义
use crate::aesthetic::AestheticLayer;
use crate::chapter::{Chapter, Volume};
use crate::character::{Character, CharacterLayer};
use crate::id::{ChapterId, CharacterId, ProjectId};
use crate::narrative::{Foreshadow, ForeshadowStatus, NarrativeLayer};
use crate::world::WorldLayer;

/// 四层本体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NovelOntology {
    /// 项目 ID
    pub project_id: ProjectId,
    /// 项目标题
    pub title: String,
    /// Layer 1: 世界层
    pub world: WorldLayer,
    /// Layer 2: 角色层
    pub characters: CharacterLayer,
    /// Layer 3: 叙事层
    pub narrative: NarrativeLayer,
    /// Layer 4: 审美层
    pub aesthetic: AestheticLayer,
    /// 章节列表
    pub chapters: Vec<Chapter>,
    /// 卷列表
    pub volumes: Vec<Volume>,
    /// 创作设定
    pub settings: crate::settings::ProjectSettings,
    /// 核心概念 / 高概念种子
    pub core_concept: crate::concept::CoreConcept,
    /// 萌芽数据
    pub sprout: crate::sprout::SproutData,
}

impl NovelOntology {
    /// 创建空白项目
    pub fn new(project_id: ProjectId, title: String) -> Self {
        Self {
            project_id,
            title,
            world: WorldLayer {
                world_id: crate::id::WorldId::default(),
                name: String::new(),
                spatial_model: crate::world::SpatialModel {
                    locations: Vec::new(),
                    hierarchy: Vec::new(),
                },
                timeline: crate::world::Timeline {
                    events: Vec::new(),
                    epoch_markers: Vec::new(),
                },
                setting_rules: Vec::new(),
                glossary: Vec::new(),
                item_graph: Vec::new(),
            },
            characters: CharacterLayer {
                characters: Vec::new(),
                relationships: Vec::new(),
            },
            narrative: NarrativeLayer {
                plot_graph: Vec::new(),
                foreshadows: Vec::new(),
                conflicts: Vec::new(),
                emotional_arcs: Vec::new(),
            },
            aesthetic: AestheticLayer {
                style_fingerprint: crate::aesthetic::StyleFingerprint {
                    sentence_length_avg: 0.0,
                    vocabulary_richness: 0.0,
                    rhetorical_frequency: 0.0,
                    dialogue_ratio: 0.0,
                    paragraph_length_avg: 0.0,
                    sample_texts: Vec::new(),
                },
                pacing_model: crate::aesthetic::PacingModel {
                    tension_curve: Vec::new(),
                    scene_length_avg: 0.0,
                    action_ratio: 0.0,
                },
                anti_ai_rules: Vec::new(),
            },
            chapters: Vec::new(),
            volumes: Vec::new(),
            settings: crate::settings::ProjectSettings::new(),
            core_concept: crate::concept::CoreConcept::new(),
            sprout: crate::sprout::SproutData::new(),
        }
    }

    /// 根据 ID 获取章节
    pub fn get_chapter(&self, chapter_id: &ChapterId) -> Option<&Chapter> {
        self.chapters.iter().find(|ch| ch.chapter_id == *chapter_id)
    }

    /// 根据 ID 获取角色
    pub fn get_character(&self, character_id: &CharacterId) -> Option<&Character> {
        self.characters
            .characters
            .iter()
            .find(|c| c.id == *character_id)
    }

    /// 获取活跃的伏笔（Planted 或 Progressing 状态）
    pub fn active_foreshadows(&self) -> Vec<&Foreshadow> {
        self.narrative
            .foreshadows
            .iter()
            .filter(|f| {
                f.status == ForeshadowStatus::Planted || f.status == ForeshadowStatus::Progressing
            })
            .collect()
    }
}
