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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter::ChapterStatus;
    use crate::character::{
        CharacterKnowledgeBase, DecayModel, DialogueStyle, Emotion, KnowledgeSet, PersonalityVector,
    };
    use crate::id::{ForeshadowId, VolumeId};

    fn make_chapter(chapter_id: &str) -> Chapter {
        Chapter {
            chapter_id: ChapterId::new(chapter_id),
            volume_id: VolumeId::new("vol-1"),
            title: format!("第{chapter_id}章"),
            content: String::new(),
            word_count: 0,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 0.0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn make_character(id: &str) -> Character {
        Character {
            id: CharacterId::new(id),
            name: format!("角色{id}"),
            core_personality: PersonalityVector { traits: vec![] },
            current_mood: Emotion {
                primary: String::new(),
                intensity: 0.0,
                secondary: String::new(),
            },
            current_location: String::new(),
            current_knowledge: KnowledgeSet { facts: vec![] },
            state_history: vec![],
            transition_rules: vec![],
            dialogue_style: DialogueStyle {
                patterns: vec![],
                vocabulary_level: String::new(),
                sentence_length_avg: 0.0,
                catchphrases: vec![],
            },
            growth_curve: vec![],
            knowledge_base: CharacterKnowledgeBase {
                known_facts: vec![],
                knowledge_sources: vec![],
                decay_model: DecayModel {
                    half_life_chapters: 0,
                    min_reliability: 0.0,
                },
            },
        }
    }

    fn make_foreshadow(id: &str, status: ForeshadowStatus) -> Foreshadow {
        Foreshadow {
            id: ForeshadowId::new(id),
            name: format!("伏笔{id}"),
            description: String::new(),
            status,
            planted_chapter: ChapterId::new("1"),
            expected_resolve_chapter: None,
            actual_resolve_chapter: None,
            related_characters: vec![],
            related_items: vec![],
        }
    }

    #[test]
    fn test_new_creates_empty_ontology() {
        let onto = NovelOntology::new(ProjectId::new("proj-1"), "测试项目".to_string());
        assert_eq!(onto.project_id.as_str(), "proj-1");
        assert_eq!(onto.title, "测试项目");
        assert!(onto.world.spatial_model.locations.is_empty());
        assert!(onto.characters.characters.is_empty());
        assert!(onto.narrative.foreshadows.is_empty());
        assert!(onto.aesthetic.anti_ai_rules.is_empty());
        assert!(onto.chapters.is_empty());
        assert!(onto.volumes.is_empty());
    }

    #[test]
    fn test_get_chapter_found_and_missing() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        onto.chapters.push(make_chapter("1"));
        onto.chapters.push(make_chapter("2"));

        let found = onto.get_chapter(&ChapterId::new("2"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "第2章");

        assert!(onto.get_chapter(&ChapterId::new("999")).is_none());
    }

    #[test]
    fn test_get_character_found_and_missing() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        onto.characters.characters.push(make_character("c-1"));

        let found = onto.get_character(&CharacterId::new("c-1"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "角色c-1");

        assert!(onto.get_character(&CharacterId::new("c-x")).is_none());
    }

    #[test]
    fn test_active_foreshadows_only_planted_or_progressing() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-planted", ForeshadowStatus::Planted));
        onto.narrative.foreshadows.push(make_foreshadow(
            "fs-progressing",
            ForeshadowStatus::Progressing,
        ));
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-resolved", ForeshadowStatus::Resolved));
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-abandoned", ForeshadowStatus::Abandoned));
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-planned", ForeshadowStatus::Planned));
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-overdue", ForeshadowStatus::Overdue));

        let active: Vec<&str> = onto
            .active_foreshadows()
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"fs-planted"));
        assert!(active.contains(&"fs-progressing"));
    }

    #[test]
    fn test_ontology_serde_round_trip() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), "回环测试".to_string());
        onto.chapters.push(make_chapter("1"));
        onto.characters.characters.push(make_character("c-1"));
        onto.narrative
            .foreshadows
            .push(make_foreshadow("fs-1", ForeshadowStatus::Planted));

        let json = serde_json::to_string(&onto).unwrap();
        let back: NovelOntology = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_id.as_str(), "proj-1");
        assert_eq!(back.title, "回环测试");
        assert_eq!(back.chapters.len(), 1);
        assert_eq!(back.characters.characters.len(), 1);
        assert_eq!(back.narrative.foreshadows.len(), 1);
    }
}
