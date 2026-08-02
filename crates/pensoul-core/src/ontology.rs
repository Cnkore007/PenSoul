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
    /// 情节脉络（大纲规划层）：每个节点覆盖一个章节范围，
    /// 展开细纲后生成真正的章节；旧项目 JSON 无此字段，默认为空
    #[serde(default)]
    pub outline_arcs: Vec<crate::narrative::OutlineArc>,
    /// 工作流技能配置（环节 → 模型 + 技法卡绑定）。
    /// 结构由前端定义，后端透明存储随项目文件持久化；旧项目 JSON 无此字段，默认为 null
    #[serde(default)]
    pub workflow_skills: serde_json::Value,
    /// 项目工作流引用（模板 ID + 版本 + 项目级覆盖）。
    /// 结构由前端定义（见 crate::workflow::WorkflowRef），后端透明存储；
    /// 旧项目 JSON 无此字段，默认为 null
    #[serde(default)]
    pub workflow_ref: serde_json::Value,
    /// 项目写作经验库：批注重写沉淀的错误经验，注入章节审查 prompt
    #[serde(default)]
    pub writing_lessons: Vec<crate::narrative::WritingLesson>,
    /// 待沉淀的编辑修改样本（保存修改时自动收集，蒸馏成 WritingLesson 后清空）
    #[serde(default)]
    pub pending_edit_samples: Vec<crate::narrative::EditSample>,
    /// 页面受控保存快照栈（每次应用保存前记录，撤回时恢复；上限 10 条/页）
    #[serde(default)]
    pub page_snapshots: Vec<crate::narrative::PageSnapshot>,
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
            outline_arcs: Vec::new(),
            workflow_skills: serde_json::Value::Null,
            workflow_ref: serde_json::Value::Null,
            writing_lessons: Vec::new(),
            pending_edit_samples: Vec::new(),
            page_snapshots: Vec::new(),
        }
    }

    /// 根据 ID 获取章节
    pub fn get_chapter(&self, chapter_id: &ChapterId) -> Option<&Chapter> {
        self.chapters.iter().find(|ch| ch.chapter_id == *chapter_id)
    }

    /// 回填章节序号：`chapter_no == 0` 的章节按数组顺序，
    /// 从现有最大序号 +1 起依次分配；已有序号保持不变。
    /// 返回是否有章节被回填（调用方据此决定是否立即落盘）。
    pub fn backfill_chapter_numbers(&mut self) -> bool {
        let mut next = self
            .chapters
            .iter()
            .map(|c| c.chapter_no)
            .max()
            .unwrap_or(0)
            + 1;
        let mut changed = false;
        for ch in &mut self.chapters {
            if ch.chapter_no == 0 {
                ch.chapter_no = next;
                next += 1;
                changed = true;
            }
        }
        changed
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

    /// 迁移历史「伪章节」：早期版本把讨论产出的情节脉络节点（梗概以
    /// 【第N-M章】开头）直接建成章节，导致一个 200 章的故事段被当成
    /// 一章来写。这里把这类章节还原为情节脉络节点并移除章节实体
    /// （其正文是整段压缩产物，不是有效章节内容，一并丢弃）。
    /// 返回是否有迁移发生（调用方据此决定是否立即落盘）。
    pub fn migrate_arc_chapters(&mut self) -> bool {
        let mut migrated = false;
        let mut keep: Vec<Chapter> = Vec::with_capacity(self.chapters.len());
        for ch in std::mem::take(&mut self.chapters) {
            match parse_arc_marker(&ch.summary) {
                Some((start, end, desc)) if end > start => {
                    self.outline_arcs.push(crate::narrative::OutlineArc {
                        arc_id: format!("arc-migrated-{}", ch.chapter_id.as_str()),
                        title: ch.title.clone(),
                        description: desc,
                        chapter_start: start,
                        chapter_end: end,
                        expanded_until: 0,
                        annotations: Vec::new(),
                    });
                    migrated = true;
                }
                _ => keep.push(ch),
            }
        }
        if migrated {
            // 有章节被迁出时才需要同步卷的章节列表
            let alive: std::collections::HashSet<&str> =
                keep.iter().map(|c| c.chapter_id.as_str()).collect();
            for vol in self.volumes.iter_mut() {
                vol.chapter_ids.retain(|cid| alive.contains(cid.as_str()));
            }
        }
        // 无论是否迁移都要把章节写回（take 出来遍历后必须归还）
        self.chapters = keep;
        migrated
    }
}

/// 解析梗概开头的脉络标记「【第N-M章】」，返回 (起始章, 结束章, 剩余描述)。
/// 手写解析避免引入 regex 依赖；兼容 -、–、~、至 四种分隔符。
fn parse_arc_marker(summary: &str) -> Option<(i64, i64, String)> {
    let body = summary.strip_prefix('【')?;
    let body = body.strip_prefix('第')?;
    let dash = body.find(['-', '–', '~', '至'])?;
    let start: i64 = body[..dash].trim().parse().ok()?;
    let rest = &body[dash..];
    let rest = rest.strip_prefix(['-', '–', '~', '至'])?;
    let close = rest.find("章】")?;
    let end: i64 = rest[..close].trim().parse().ok()?;
    let desc = rest[close + "章】".len()..].trim().to_string();
    Some((start, end, desc))
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
            chapter_no: 1,
            volume_id: VolumeId::new("vol-1"),
            title: format!("第{chapter_id}章"),
            summary: String::new(),
            content: String::new(),
            word_count: 0,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 0.0,
            created_at: String::new(),
            updated_at: String::new(),
            annotations: Vec::new(),
            revisions: Vec::new(),
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
            annotations: vec![],
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

    #[test]
    fn test_backfill_all_zero_assigns_in_array_order() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        for id in ["ch-a", "ch-b", "ch-c"] {
            let mut ch = make_chapter(id);
            ch.chapter_no = 0;
            onto.chapters.push(ch);
        }

        assert!(onto.backfill_chapter_numbers());
        let nos: Vec<i64> = onto.chapters.iter().map(|c| c.chapter_no).collect();
        assert_eq!(nos, vec![1, 2, 3]);
        // 幂等：再次回填不再有变化
        assert!(!onto.backfill_chapter_numbers());
    }

    #[test]
    fn test_backfill_keeps_existing_and_continues_from_max() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        let mut c1 = make_chapter("ch-a");
        c1.chapter_no = 0;
        let mut c2 = make_chapter("ch-b");
        c2.chapter_no = 5;
        let mut c3 = make_chapter("ch-c");
        c3.chapter_no = 0;
        onto.chapters.extend([c1, c2, c3]);

        assert!(onto.backfill_chapter_numbers());
        let nos: Vec<i64> = onto.chapters.iter().map(|c| c.chapter_no).collect();
        assert_eq!(nos, vec![6, 5, 7]);
    }

    #[test]
    fn test_chapter_serde_default_chapter_no() {
        // 旧版项目 JSON 没有 chapter_no 字段，反序列化应为 0（待回填）
        let json = r#"{
            "chapter_id": "ch-x",
            "volume_id": "vol-1",
            "title": "旧章节",
            "content": "",
            "word_count": 0,
            "version": 1,
            "status": "Draft",
            "consistency_score": 1.0,
            "created_at": "",
            "updated_at": ""
        }"#;
        let ch: Chapter = serde_json::from_str(json).unwrap();
        assert_eq!(ch.chapter_no, 0);
    }

    #[test]
    fn test_migrate_arc_chapters_converts_pseudo_chapters() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        // 伪章节：讨论导入的脉络节点（梗概以【第N-M章】开头，还带了压缩正文）
        let mut pseudo = make_chapter("ch-pseudo");
        pseudo.title = "枯井边的勘验".to_string();
        pseudo.summary = "【第1-200章】\n主角在临渊城醒来并面对第一具尸体".to_string();
        pseudo.content = "被压缩的整段内容……".to_string();
        // 正常章节：梗概不含脉络标记
        let mut normal = make_chapter("ch-normal");
        normal.summary = "主角在井边发现尸体".to_string();
        onto.chapters.extend([pseudo, normal]);
        onto.volumes.push(crate::chapter::Volume {
            volume_id: VolumeId::new("vol-1"),
            title: "第一卷".to_string(),
            chapter_ids: vec![ChapterId::new("ch-pseudo"), ChapterId::new("ch-normal")],
            summary: String::new(),
        });

        assert!(onto.migrate_arc_chapters());
        // 伪章节变成脉络节点，正常章节保留
        assert_eq!(onto.outline_arcs.len(), 1);
        let arc = &onto.outline_arcs[0];
        assert_eq!(arc.title, "枯井边的勘验");
        assert_eq!(arc.chapter_start, 1);
        assert_eq!(arc.chapter_end, 200);
        assert_eq!(arc.expanded_until, 0);
        assert_eq!(arc.description, "主角在临渊城醒来并面对第一具尸体");
        assert_eq!(onto.chapters.len(), 1);
        assert_eq!(onto.chapters[0].chapter_id.as_str(), "ch-normal");
        // 卷的章节列表同步剔除被迁移章节
        assert_eq!(onto.volumes[0].chapter_ids.len(), 1);
        // 幂等：再次迁移无变化
        assert!(!onto.migrate_arc_chapters());
    }

    #[test]
    fn test_migrate_arc_chapters_ignores_ordinary_summaries() {
        let mut onto = NovelOntology::new(ProjectId::new("proj-1"), String::new());
        let mut ch = make_chapter("ch-a");
        ch.summary = "【关键证据】井边的脚印".to_string();
        onto.chapters.push(ch);
        assert!(!onto.migrate_arc_chapters());
        assert!(onto.outline_arcs.is_empty());
        assert_eq!(onto.chapters.len(), 1);
    }

    #[test]
    fn test_parse_arc_marker_variants() {
        assert_eq!(
            super::parse_arc_marker("【第1-200章】描述"),
            Some((1, 200, "描述".to_string()))
        );
        assert_eq!(
            super::parse_arc_marker("【第401–600章】"),
            Some((401, 600, String::new()))
        );
        assert_eq!(super::parse_arc_marker("普通梗概"), None);
        assert_eq!(super::parse_arc_marker("【第1章】单章"), None);
    }
}
