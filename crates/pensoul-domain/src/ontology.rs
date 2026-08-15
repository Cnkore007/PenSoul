// ontology.rs — NovelOntology 核心本体
// 唯一正典数据结构，四层本体 + 项目配置

use crate::blueprint::BookBlueprint;
use crate::chapter::{Chapter, Volume};
use crate::concept::CoreConcept;
use crate::entity::{Annotation, Character, Event, Foreshadow, Organization, Setting};
use crate::id::*;
use crate::narrative::OutlineArc;
use crate::settings::ProjectSettings;
use crate::sprout::SoulSproutSession;
use serde::{Deserialize, Serialize};

/// 世界层
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldLayer {
    pub name: String,
    pub locations: Vec<Setting>,
    pub timeline: Vec<Event>,
    pub rules: Vec<String>,
    /// 组织档案（势力/宗门/家族等，P0 新增）
    #[serde(default)]
    pub organizations: Vec<Organization>,
}

/// 角色层
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterLayer {
    pub characters: Vec<Character>,
}

/// 叙事层
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NarrativeLayer {
    pub foreshadows: Vec<Foreshadow>,
    pub conflicts: Vec<String>,
    pub emotional_arcs: Vec<String>,
}

/// 美学层
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AestheticLayer {
    pub style_notes: String,
    pub pacing_notes: String,
}

/// NovelOntology — 唯一正典
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelOntology {
    pub project_id: ProjectId,
    pub title: String,
    pub description: String,

    // 四层本体
    pub world: WorldLayer,
    pub characters: CharacterLayer,
    pub narrative: NarrativeLayer,
    pub aesthetic: AestheticLayer,

    // 章节与卷
    pub chapters: Vec<Chapter>,
    pub volumes: Vec<Volume>,
    pub outline_arcs: Vec<OutlineArc>,

    // 项目配置
    pub settings: ProjectSettings,
    pub core_concept: CoreConcept,

    // 蓝图
    pub blueprint: BookBlueprint,

    // 灵魂萌芽（对话式创作工作台）
    #[serde(default)]
    pub soul_sprout: SoulSproutSession,

    // 批注
    pub annotations: Vec<Annotation>,
}

impl NovelOntology {
    /// 创建新项目
    pub fn new(project_id: ProjectId, title: impl Into<String>) -> Self {
        Self {
            project_id,
            title: title.into(),
            description: String::new(),
            world: WorldLayer::default(),
            characters: CharacterLayer::default(),
            narrative: NarrativeLayer::default(),
            aesthetic: AestheticLayer::default(),
            chapters: Vec::new(),
            volumes: Vec::new(),
            outline_arcs: Vec::new(),
            settings: ProjectSettings::default(),
            core_concept: CoreConcept::new(),
            blueprint: BookBlueprint::default(),
            soul_sprout: SoulSproutSession::new(),
            annotations: Vec::new(),
        }
    }

    /// 获取指定章节
    pub fn get_chapter(&self, chapter_id: &ChapterId) -> Option<&Chapter> {
        self.chapters.iter().find(|c| &c.chapter_id == chapter_id)
    }

    /// 获取指定章节（可变引用）
    pub fn get_chapter_mut(&mut self, chapter_id: &ChapterId) -> Option<&mut Chapter> {
        self.chapters.iter_mut().find(|c| &c.chapter_id == chapter_id)
    }

    /// 获取指定角色
    pub fn get_character(&self, character_id: &CharacterId) -> Option<&Character> {
        self.characters
            .characters
            .iter()
            .find(|c| &c.id == character_id)
    }

    /// 活跃伏笔（未回收的）
    pub fn active_foreshadows(&self) -> Vec<&Foreshadow> {
        self.narrative
            .foreshadows
            .iter()
            .filter(|f| {
                !matches!(
                    f.status,
                    crate::entity::ForeshadowStatus::Resolved
                        | crate::entity::ForeshadowStatus::Abandoned
                )
            })
            .collect()
    }

    /// 按章节号排序的章节列表
    pub fn chapters_in_order(&self) -> Vec<&Chapter> {
        let mut chapters: Vec<&Chapter> = self.chapters.iter().collect();
        chapters.sort_by_key(|c| c.chapter_no);
        chapters
    }

    /// 回填章节序号（按数组顺序）
    pub fn backfill_chapter_numbers(&mut self) {
        for (i, chapter) in self.chapters.iter_mut().enumerate() {
            if chapter.chapter_no == 0 {
                chapter.chapter_no = (i + 1) as i64;
            }
        }
    }
}
