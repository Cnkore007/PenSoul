//! pensoul-core 领域模型集成测试
//!
//! 覆盖跨模块的 serde round-trip 与旧版数据兼容（flexible_id）。

use pensoul_core::*;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let json = serde_json::to_string(value).expect("序列化失败");
    serde_json::from_str(&json).expect("反序列化失败")
}

// ── Chapter ─────────────────────────────────────────────────────────

#[test]
fn test_chapter_all_status_variants_round_trip() {
    let statuses = [
        ChapterStatus::Draft,
        ChapterStatus::Reviewing,
        ChapterStatus::Reviewed,
        ChapterStatus::Polished,
        ChapterStatus::Published,
    ];
    for (i, status) in statuses.into_iter().enumerate() {
        let ch = Chapter {
            chapter_id: ChapterId::from_i64(i as i64 + 1),
            chapter_no: i as i64 + 1,
            volume_id: VolumeId::new("vol-1"),
            title: "标题".to_string(),
            summary: String::new(),
            content: "正文".to_string(),
            word_count: 2500,
            version: 1,
            status,
            consistency_score: 0.95,
            created_at: "2026-07-30T00:00:00Z".to_string(),
            updated_at: "2026-07-30T01:00:00Z".to_string(),
        };
        let back = round_trip(&ch);
        assert_eq!(back.chapter_id, ch.chapter_id);
        assert_eq!(back.status, ch.status);
        assert_eq!(back.word_count, 2500);
    }
}

// ── Expert / ExpertList ─────────────────────────────────────────────

#[test]
fn test_expert_list_round_trip() {
    let list = ExpertList {
        experts: vec![
            Expert {
                id: "e-1".to_string(),
                name: "结构大师".to_string(),
                description: "叙事结构分析".to_string(),
                source_persona: "某作家".to_string(),
                model_id: "kimi".to_string(),
                perspective: "结构".to_string(),
                default_prompt: "分析结构".to_string(),
                created_at: "2026-07-30".to_string(),
                skill_path: Some("skills/structure.md".to_string()),
                skill_summary: None,
            },
            Expert {
                id: "e-2".to_string(),
                name: "文风侦探".to_string(),
                description: String::new(),
                source_persona: String::new(),
                model_id: "kimi".to_string(),
                perspective: "文风".to_string(),
                default_prompt: String::new(),
                created_at: String::new(),
                skill_path: None,
                skill_summary: None,
            },
        ],
    };
    let back = round_trip(&list);
    assert_eq!(back.experts.len(), 2);
    assert_eq!(
        back.experts[0].skill_path.as_deref(),
        Some("skills/structure.md")
    );
    assert!(back.experts[1].skill_path.is_none());
}

// ── 旧版数字 chapter_id 兼容（flexible_id）────────────────────────

#[test]
fn test_state_transition_accepts_legacy_numeric_chapter_id() {
    // 旧版数据 chapter_id 是 JSON 数字
    let json = r#"{
        "from": "平静",
        "to": "愤怒",
        "trigger": "背叛",
        "chapter_id": 3,
        "story_time": "第三年春",
        "causality": "被挚友出卖"
    }"#;
    let st: StateTransition = serde_json::from_str(json).unwrap();
    assert_eq!(st.chapter_id.as_str(), "3");
    assert_eq!(st.chapter_id.as_i64(), Some(3));
}

#[test]
fn test_growth_point_accepts_legacy_numeric_chapter_id() {
    let json = r#"{"chapter_id": 7, "dimension": "勇气", "value": 0.8, "note": "首次直面强敌"}"#;
    let gp: GrowthPoint = serde_json::from_str(json).unwrap();
    assert_eq!(gp.chapter_id.as_str(), "7");
    assert!((gp.value - 0.8).abs() < f32::EPSILON);
}

#[test]
fn test_relationship_change_accepts_legacy_numeric_chapter_id() {
    let json = r#"{
        "chapter_id": 12,
        "old_type": "朋友",
        "new_type": "敌人",
        "reason": "夺宝反目"
    }"#;
    let rc: RelationshipChange = serde_json::from_str(json).unwrap();
    assert_eq!(rc.chapter_id.as_str(), "12");
    assert_eq!(rc.new_type, "敌人");
}

#[test]
fn test_state_transition_string_chapter_id_still_works() {
    // 新版数据 chapter_id 是字符串，必须继续兼容
    let json = r#"{
        "from": "a",
        "to": "b",
        "trigger": "t",
        "chapter_id": "15",
        "story_time": "s",
        "causality": "c"
    }"#;
    let st: StateTransition = serde_json::from_str(json).unwrap();
    assert_eq!(st.chapter_id.as_str(), "15");
}

// ── Character 知识系统 ──────────────────────────────────────────────

#[test]
fn test_knowledge_source_told_variant_round_trip() {
    let item = KnowledgeItem {
        fact_id: "fact-1".to_string(),
        content: "皇宫密道位置".to_string(),
        source: KnowledgeSource::Told {
            from: CharacterId::new("c-spy"),
        },
        reliability: 0.7,
    };
    let back = round_trip(&item);
    match back.source {
        KnowledgeSource::Told { from } => assert_eq!(from.as_str(), "c-spy"),
        other => panic!("期望 Told 变体，实际: {other:?}"),
    }
}

// ── Foreshadow 完整生命周期字段 ─────────────────────────────────────

#[test]
fn test_foreshadow_resolved_round_trip() {
    let fs = Foreshadow {
        id: ForeshadowId::new("fs-1"),
        name: "身世之谜".to_string(),
        description: "主角真实身份".to_string(),
        status: ForeshadowStatus::Resolved,
        planted_chapter: ChapterId::from_i64(2),
        expected_resolve_chapter: Some(ChapterId::from_i64(40)),
        actual_resolve_chapter: Some(ChapterId::from_i64(38)),
        related_characters: vec![CharacterId::new("c-1"), CharacterId::new("c-2")],
        related_items: vec!["玉佩".to_string()],
    };
    let back = round_trip(&fs);
    assert_eq!(back.status, ForeshadowStatus::Resolved);
    assert_eq!(
        back.actual_resolve_chapter.as_ref().unwrap().as_i64(),
        Some(38)
    );
    assert_eq!(back.related_characters.len(), 2);
}

// ── World 层 ────────────────────────────────────────────────────────

#[test]
fn test_world_layer_round_trip() {
    let world = WorldLayer {
        world_id: WorldId::new("w-1"),
        name: "九州大陆".to_string(),
        spatial_model: SpatialModel {
            locations: Vec::new(),
            hierarchy: Vec::new(),
        },
        timeline: Timeline {
            events: Vec::new(),
            epoch_markers: Vec::new(),
        },
        setting_rules: Vec::new(),
        glossary: Vec::new(),
        item_graph: Vec::new(),
    };
    let back = round_trip(&world);
    assert_eq!(back.name, "九州大陆");
    assert_eq!(back.world_id.as_str(), "w-1");
}
