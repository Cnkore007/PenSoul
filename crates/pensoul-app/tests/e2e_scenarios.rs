/// PenSoul v2 — 集成测试：端到端场景验收
///
/// 覆盖验收标准：
/// - #72: 端到端 — 章节创作流程
/// - #73: 端到端 — 联动传播
/// - #74: 端到端 — 崩溃恢复
/// - #75: 端到端 — 记忆注入
use pensoul_cda::edge::{EdgeRelation, ImpactEdge};
use pensoul_cda::graph::ImpactGraph;
use pensoul_cda::node::{ImpactNode, NodeType};
use pensoul_consistency::checker::IncrementalChecker;
use pensoul_consistency::entity_state::{EntityState, EntityType};
use pensoul_core::chapter::{Chapter, ChapterStatus};
use pensoul_core::id::{ChapterId, ProjectId, StageName, VolumeId};
use pensoul_core::ontology::NovelOntology;
use pensoul_harness::engine::HarnessEngine;
use pensoul_harness::stage::{GateType, RunnerType, Stage};
use pensoul_memory::cold::ColdMemory;
use pensoul_memory::hot::HotMemory;
use pensoul_memory::narrative::NarrativeMemory;
use pensoul_memory::packet::{ChapterSummary, NarrativeCategory, NarrativeDetail};
use pensoul_memory::warm::WarmMemory;
use std::collections::HashMap;

// ── 辅助函数 ──────────────────────────────────────────────────────────────

/// 创建测试用的章节
fn make_chapter(chapter_id: &str, title: &str) -> Chapter {
    Chapter {
        chapter_id: ChapterId::new(chapter_id),
        chapter_no: 1,
        volume_id: VolumeId::new("vol_1"),
        title: title.to_string(),
        summary: String::new(),
        content: format!("这是「{title}」的内容。"),
        word_count: 100,
        version: 1,
        status: ChapterStatus::Draft,
        consistency_score: 0.9,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

/// 创建测试用的创作阶段
fn make_writing_stage() -> Stage {
    Stage {
        name: StageName::new("chapter_write"),
        display_name: "章节写作".into(),
        tools_allowed: vec!["write_prose".into(), "read_outline".into()],
        tools_denied: vec!["modify_settings".into()],
        gate_type: GateType::Auto,
        next_stage: Some(StageName::new("chapter_review")),
        runner: RunnerType::Local,
        max_retries: 0,
        ..Stage::default()
    }
}

/// 创建测试用的审阅阶段
fn make_review_stage() -> Stage {
    Stage {
        name: StageName::new("chapter_review"),
        display_name: "一致性审阅".into(),
        gate_type: GateType::Conditional,
        gate_condition: Some("consistency_score >= 80".into()),
        on_fail: Some(StageName::new("chapter_write")),
        next_stage: Some(StageName::new("chapter_polish")),
        max_retries: 2,
        ..Stage::default()
    }
}

/// 创建测试用的润色阶段
fn make_polish_stage() -> Stage {
    Stage {
        name: StageName::new("chapter_polish"),
        display_name: "润色".into(),
        gate_type: GateType::Auto,
        next_stage: None,
        ..Stage::default()
    }
}

/// 创建章节摘要
fn make_chapter_summary(chapter_id: i64, title: &str, summary: &str) -> ChapterSummary {
    ChapterSummary {
        chapter_id: ChapterId::new(chapter_id.to_string()),
        title: title.to_string(),
        summary: summary.to_string(),
        key_events: vec![format!("{title}的关键事件")],
        character_states: HashMap::new(),
        word_count: 1000,
        consistency_score: 0.9,
        semantic_embedding: None,
    }
}

// ── 场景一：章节创作流程（验收标准 #72）──────────────────────────────────

#[test]
fn test_chapter_writing_flow() {
    // 1. 创建项目
    let project_id = ProjectId::new("proj_001");
    let mut ontology = NovelOntology::new(project_id, "测试小说".to_string());

    // 2. 添加章节到 ontology
    let chapter1 = make_chapter("ch_001", "第一章：开端");
    let chapter2 = make_chapter("ch_002", "第二章：发展");
    ontology.chapters.push(chapter1);
    ontology.chapters.push(chapter2);

    assert_eq!(ontology.chapters.len(), 2);
    assert_eq!(ontology.chapters[0].title, "第一章：开端");
    assert_eq!(ontology.chapters[1].title, "第二章：发展");

    // 3. 创建 HarnessEngine 并注册阶段
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());

    engine.register_stage(make_writing_stage());
    engine.register_stage(make_review_stage());
    engine.register_stage(make_polish_stage());

    assert_eq!(engine.stage_count(), 3);

    // 4. 设置起始阶段并启动
    engine
        .set_start_stage(StageName::new("chapter_write"))
        .unwrap();

    let stage_instance = engine.start_stage().unwrap();
    assert_eq!(
        stage_instance.status,
        pensoul_harness::stage::StageStatus::Running
    );

    // 5. 注入备忘录
    engine.inject_memo("current_chapter", "ch_001").unwrap();
    engine.inject_memo("chapter_title", "第一章：开端").unwrap();
    assert_eq!(engine.memo.get("current_chapter"), Some("ch_001"));

    // 6. 完成阶段
    let result = serde_json::json!({
        "output": "章节写作完成",
        "word_count": 1500,
        "consistency_score": 85
    });
    engine.complete_stage(result).unwrap();

    // 7. 验证状态流转：Auto 门控通过后应推进到 chapter_review
    assert_eq!(
        engine.current_stage().map(|n| n.as_str()),
        Some("chapter_review")
    );

    // 8. 使用 IncrementalChecker 做一致性检查
    let mut checker = IncrementalChecker::new();
    checker.register_state(EntityState {
        entity_id: "char_001".to_string(),
        entity_type: EntityType::Character,
        chapter_id: ChapterId::new("1"),
        state_data: serde_json::json!({"name": "主角", "location": "村庄"}),
        version: 1,
    });
    checker.register_state(EntityState {
        entity_id: "char_001".to_string(),
        entity_type: EntityType::Character,
        chapter_id: ChapterId::new("2"),
        state_data: serde_json::json!({"name": "主角", "location": "森林"}),
        version: 2,
    });

    let report = checker.check_incremental(ChapterId::new("2"), EntityType::Character);
    assert_eq!(report.total_entities_checked, 1);

    // 9. 验证流程状态
    let state = engine.build_state();
    assert_eq!(state.current_stage.as_deref(), Some("chapter_review"));
    assert!(state.memo.contains_key("current_chapter"));
    assert!(state.stages_status.contains_key("chapter_write"));
}

// ── 场景二：联动传播（验收标准 #73）──────────────────────────────────────

#[test]
fn test_change_propagation() {
    // 1. 创建影响图
    let mut graph = ImpactGraph::new();

    // 2. 添加节点（模拟章节依赖）
    // 第1章：角色引入
    graph.add_node(ImpactNode::new(
        "ch1_char_intro".to_string(),
        NodeType::Entity,
        1,
        "hash_char_intro".to_string(),
    ));

    // 第2章：角色发展
    graph.add_node(ImpactNode::new(
        "ch2_char_development".to_string(),
        NodeType::Entity,
        2,
        "hash_char_dev".to_string(),
    ));

    // 第3章：伏笔设置
    graph.add_node(ImpactNode::new(
        "ch3_foreshadow".to_string(),
        NodeType::Foreshadow,
        3,
        "hash_foreshadow".to_string(),
    ));

    // 第4章：伏笔引用
    graph.add_node(ImpactNode::new(
        "ch4_foreshadow_ref".to_string(),
        NodeType::Foreshadow,
        4,
        "hash_foreshadow_ref".to_string(),
    ));

    // 第5章：结局
    graph.add_node(ImpactNode::new(
        "ch5_ending".to_string(),
        NodeType::Event,
        5,
        "hash_ending".to_string(),
    ));

    assert_eq!(graph.node_count(), 5);

    // 3. 添加边（模拟依赖关系）
    // ch2 依赖 ch1
    graph
        .add_edge(ImpactEdge::new(
            "ch2_char_development".to_string(),
            "ch1_char_intro".to_string(),
            EdgeRelation::References,
            1.0,
        ))
        .unwrap();

    // ch3 依赖 ch2
    graph
        .add_edge(ImpactEdge::new(
            "ch3_foreshadow".to_string(),
            "ch2_char_development".to_string(),
            EdgeRelation::DependsOn,
            0.9,
        ))
        .unwrap();

    // ch4 引用 ch3
    graph
        .add_edge(ImpactEdge::new(
            "ch4_foreshadow_ref".to_string(),
            "ch3_foreshadow".to_string(),
            EdgeRelation::References,
            0.8,
        ))
        .unwrap();

    // ch5 依赖 ch3 和 ch4
    graph
        .add_edge(ImpactEdge::new(
            "ch5_ending".to_string(),
            "ch3_foreshadow".to_string(),
            EdgeRelation::Causes,
            1.0,
        ))
        .unwrap();

    graph
        .add_edge(ImpactEdge::new(
            "ch5_ending".to_string(),
            "ch4_foreshadow_ref".to_string(),
            EdgeRelation::DependsOn,
            0.7,
        ))
        .unwrap();

    assert_eq!(graph.edge_count(), 5);

    // 4. 修改第 2 章，查询受影响章节
    let affected = graph.find_affected(2, &["ch2_char_development".to_string()], 10);

    // 验证受影响节点：ch3, ch4, ch5 都应该受影响
    let affected_ids: Vec<&str> = affected.iter().map(|a| a.node_id.as_str()).collect();

    // ch3 直接依赖 ch2
    assert!(affected_ids.contains(&"ch3_foreshadow"), "ch3 应该受影响");
    // ch4 引用 ch3，应该被传播影响
    assert!(
        affected_ids.contains(&"ch4_foreshadow_ref"),
        "ch4 应该受影响"
    );
    // ch5 依赖 ch3 和 ch4，应该被传播影响
    assert!(affected_ids.contains(&"ch5_ending"), "ch5 应该受影响");

    // 5. 验证影响结果的严重程度
    for item in &affected {
        match item.node_id.as_str() {
            "ch3_foreshadow" => {
                // 深度1，章节距离1 (|2-3|=1 <= 2)，应该是 Direct
                assert_eq!(item.severity, pensoul_cda::node::ImpactSeverity::Direct);
            }
            "ch4_foreshadow_ref" => {
                // 深度2，章节距离2 (|2-4|=2 <= 2)，应该是 Direct
                assert_eq!(item.severity, pensoul_cda::node::ImpactSeverity::Direct);
            }
            "ch5_ending" => {
                // 深度2，章节距离3 (|2-5|=3 > 2)，应该是 Indirect
                assert_eq!(item.severity, pensoul_cda::node::ImpactSeverity::Indirect);
            }
            _ => {}
        }
    }

    // 6. 验证每个受影响项都有建议操作
    for item in &affected {
        assert!(
            !item.suggested_action.is_empty(),
            "受影响项 {} 应该有建议操作",
            item.node_id
        );
    }
}

// ── 场景三：崩溃恢复（验收标准 #74）──────────────────────────────────────

#[test]
fn test_crash_recovery() {
    let tmp = tempfile::tempdir().unwrap();

    // 1. 创建 HarnessEngine
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(make_writing_stage());
    engine.register_stage(make_review_stage());
    engine.register_stage(make_polish_stage());

    // 2. 设置起始阶段并执行操作
    engine
        .set_start_stage(StageName::new("chapter_write"))
        .unwrap();

    // 启动阶段
    let _ = engine.start_stage();

    // 注入备忘录
    engine.inject_memo("chapter_id", "ch_001").unwrap();
    engine
        .inject_memo("chapter_title", "崩溃恢复测试章节")
        .unwrap();
    engine.inject_memo("word_count", "1500").unwrap();

    // 3. 获取快照并保存状态
    let state_before = engine.build_state();
    assert_eq!(state_before.current_stage.as_deref(), Some("chapter_write"));
    assert_eq!(
        state_before.memo.get("chapter_id").map(|s| s.as_str()),
        Some("ch_001")
    );
    assert_eq!(
        state_before.memo.get("chapter_title").map(|s| s.as_str()),
        Some("崩溃恢复测试章节")
    );

    engine.save_state().unwrap();

    // 4. 模拟崩溃（丢弃引擎）
    drop(engine);

    // 5. 恢复引擎（从 WAL 重放）
    let mut engine2 = HarnessEngine::new(tmp.path());
    engine2.register_stage(make_writing_stage());
    engine2.register_stage(make_review_stage());
    engine2.register_stage(make_polish_stage());

    let recovered = engine2.recover_from_crash().unwrap();
    assert!(recovered, "崩溃恢复应该成功");

    // 6. 验证 state 和 memo 完整
    assert_eq!(
        engine2.current_stage().map(|n| n.as_str()),
        Some("chapter_write")
    );
    assert_eq!(engine2.memo.get("chapter_id"), Some("ch_001"));
    assert_eq!(engine2.memo.get("chapter_title"), Some("崩溃恢复测试章节"));
    assert_eq!(engine2.memo.get("word_count"), Some("1500"));

    // 7. 验证恢复后可以继续执行
    let state_after = engine2.build_state();
    assert_eq!(state_after.current_stage.as_deref(), Some("chapter_write"));
    assert!(state_after.stages_status.contains_key("chapter_write"));
}

// ── 场景四：记忆注入（验收标准 #75）──────────────────────────────────────

#[test]
fn test_memory_injection() {
    // 1. 创建四种记忆层
    let mut hot = HotMemory::new(2);
    let mut warm = WarmMemory::new();
    let mut cold = ColdMemory::new(2);
    let mut narrative = NarrativeMemory::new();

    // 2. 向热记忆添加章节
    hot.insert(
        1,
        "第一章：故事开始。主角在村庄中醒来，发现自己失去了记忆。".to_string(),
    );
    hot.insert(
        2,
        "第二章：探索森林。主角在森林中遇到一只神秘的白狼。".to_string(),
    );
    hot.insert(
        3,
        "第三章：白狼的指引。白狼带领主角来到一座古老的神庙。".to_string(),
    );

    // 3. 向温记忆添加章节摘要
    warm.insert_chapter(
        1,
        make_chapter_summary(1, "开端", "主角失去记忆，在村庄中醒来"),
    );
    warm.insert_chapter(2, make_chapter_summary(2, "探索", "主角探索森林，遇到白狼"));
    warm.insert_chapter(3, make_chapter_summary(3, "指引", "白狼带领主角来到神庙"));

    // 设置伏笔和角色状态
    warm.set_foreshadows(vec![
        "白狼的真实身份".to_string(),
        "神庙的秘密".to_string(),
        "主角失去的记忆".to_string(),
    ]);
    warm.set_character_states("主角：失忆状态，信任白狼；白狼：神秘，引导者".to_string());

    // 4. 向冷记忆添加较早的章节
    cold.insert_chapter(-2, make_chapter_summary(-2, "序章", "世界观介绍，古代传说"));
    cold.insert_chapter(-1, make_chapter_summary(-1, "前传", "主角的过去"));

    // 5. 向叙事记忆添加细节
    narrative.add_detail(NarrativeDetail {
        detail_id: "detail_001".to_string(),
        chapter_id: ChapterId::new("1"),
        category: NarrativeCategory::Habit,
        content: "主角习惯性地摸左手无名指上的戒指".to_string(),
        importance: 0.8,
        last_referenced: Some(ChapterId::new("2")),
    });

    narrative.add_detail(NarrativeDetail {
        detail_id: "detail_002".to_string(),
        chapter_id: ChapterId::new("2"),
        category: NarrativeCategory::Prop,
        content: "白狼脖子上挂着一枚古老的吊坠".to_string(),
        importance: 0.9,
        last_referenced: Some(ChapterId::new("3")),
    });

    narrative.add_detail(NarrativeDetail {
        detail_id: "detail_003".to_string(),
        chapter_id: ChapterId::new("1"),
        category: NarrativeCategory::Subplot,
        content: "村庄中有一个关于失落神庙的传说".to_string(),
        importance: 0.7,
        last_referenced: Some(ChapterId::new("3")),
    });

    // 6. 构建记忆包（模拟第 3 章写作）
    let budget = 10000;
    let current_chapter = 3;

    // 构建热记忆
    let hot_data = hot.build(current_chapter, budget / 2);

    // 构建温记忆
    let warm_data = warm.build(current_chapter, budget / 4);

    // 构建冷记忆
    let cold_data = cold.retrieve(current_chapter, budget / 4);

    // 构建叙事记忆
    let narrative_data = narrative.retrieve(current_chapter, budget / 8);

    // 7. 验证热记忆包含正确的章节
    assert!(!hot_data.is_empty(), "热记忆不应为空");
    assert!(
        hot_data.iter().any(|s| s.contains("第一章")),
        "热记忆应包含第一章"
    );
    assert!(
        hot_data.iter().any(|s| s.contains("第二章")),
        "热记忆应包含第二章"
    );
    assert!(
        hot_data.iter().any(|s| s.contains("第三章")),
        "热记忆应包含第三章"
    );

    // 验证热记忆标签正确
    assert!(
        hot_data.iter().any(|s| s.contains("[前前章]")),
        "应有前前章标签"
    );
    assert!(
        hot_data.iter().any(|s| s.contains("[前一章]")),
        "应有前一章标签"
    );
    assert!(
        hot_data.iter().any(|s| s.contains("[当前章]")),
        "应有当前章标签"
    );

    // 8. 验证温记忆包含正确内容
    assert!(
        warm_data.volume_summary.contains("第1章"),
        "温记忆应包含第1章摘要"
    );
    assert!(
        warm_data.volume_summary.contains("第2章"),
        "温记忆应包含第2章摘要"
    );
    assert!(
        warm_data.volume_summary.contains("第3章"),
        "温记忆应包含第3章摘要"
    );
    assert_eq!(warm_data.active_foreshadows.len(), 3);
    assert!(
        warm_data
            .active_foreshadows
            .contains(&"白狼的真实身份".to_string()),
        "应包含伏笔"
    );
    assert!(warm_data.character_states.is_some());

    // 9. 验证冷记忆排除了热记忆窗口内的章节
    // 当前第 3 章，窗口 ±2 = 1,2,3,4,5，所以冷记忆只包含 -2, -1
    assert!(
        cold_data.iter().any(|s| s.contains("序章")),
        "冷记忆应包含序章"
    );
    assert!(
        cold_data.iter().any(|s| s.contains("前传")),
        "冷记忆应包含前传"
    );

    // 10. 验证叙事记忆包含高重要性细节
    assert!(!narrative_data.is_empty(), "叙事记忆不应为空");
    assert!(
        narrative_data.iter().any(|d| d.detail_id == "detail_002"),
        "应包含高重要性细节"
    );

    // 11. 构建最终记忆包
    let packet = pensoul_memory::packet::MemoryPacket {
        hot: hot_data.clone(),
        warm: warm_data.clone(),
        cold: cold_data.clone(),
        narrative: narrative_data.clone(),
        total_tokens: hot_data.len() * 50 + warm_data.active_foreshadows.len() * 20,
        budget_used: pensoul_memory::packet::BudgetRatio {
            hot: 0.5,
            warm: 0.25,
            cold: 0.2,
            narrative: 0.05,
        },
    };

    // 12. 验证记忆包包含正确的记忆上下文
    assert!(!packet.hot.is_empty(), "记忆包热记忆不应为空");
    assert!(
        !packet.warm.volume_summary.is_empty(),
        "记忆包温记忆摘要不应为空"
    );
    assert!(
        !packet.warm.active_foreshadows.is_empty(),
        "记忆包温记忆伏笔不应为空"
    );
    assert!(!packet.narrative.is_empty(), "记忆包叙事记忆不应为空");

    // 验证完整上下文可用于 LLM prompt
    let prompt_context = format!(
        "热记忆：{}\n温记忆摘要：{}\n伏笔：{}\n冷记忆：{}\n叙事细节：{}",
        packet.hot.join("\n"),
        packet.warm.volume_summary,
        packet.warm.active_foreshadows.join(", "),
        packet.cold.join("\n"),
        packet
            .narrative
            .iter()
            .map(|d| d.content.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );

    assert!(
        prompt_context.contains("主角在村庄中醒来"),
        "prompt 应包含热记忆内容"
    );
    assert!(
        prompt_context.contains("白狼的真实身份"),
        "prompt 应包含伏笔"
    );
    assert!(prompt_context.contains("序章"), "prompt 应包含冷记忆内容");
    assert!(
        prompt_context.contains("白狼脖子上挂着一枚古老的吊坠"),
        "prompt 应包含叙事细节"
    );
}
