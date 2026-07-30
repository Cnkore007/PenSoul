//! Harness 引擎流程集成测试（原 engine.rs 内置测试迁移而来，
//! 以满足 AGENTS.md 单文件不超过 500 行的约束）。
use pensoul_core::{PensoulError, StageName};
use pensoul_harness::stage::{GateType, RunnerType, StageStatus};
use pensoul_harness::{HarnessEngine, Stage};

fn writing_stage() -> Stage {
    Stage {
        name: StageName::new("writing"),
        display_name: "章节写作".into(),
        tools_allowed: vec!["write_prose".into(), "read_outline".into()],
        tools_denied: vec!["modify_settings".into()],
        gate_type: GateType::Auto,
        next_stage: Some(StageName::new("review")),
        runner: RunnerType::Local,
        max_retries: 0,
        ..Stage::default()
    }
}

fn review_stage() -> Stage {
    Stage {
        name: StageName::new("review"),
        display_name: "一致性审查".into(),
        gate_type: GateType::Conditional,
        gate_condition: Some("consistency_score >= 80".into()),
        on_fail: Some(StageName::new("writing")),
        next_stage: Some(StageName::new("polish")),
        max_retries: 2,
        ..Stage::default()
    }
}

fn polish_stage() -> Stage {
    Stage {
        name: StageName::new("polish"),
        display_name: "润色".into(),
        gate_type: GateType::Auto,
        next_stage: None,
        ..Stage::default()
    }
}

#[test]
fn test_engine_creation_and_stage_registration() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());

    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    assert_eq!(engine.stage_count(), 2);
}

#[test]
fn test_set_start_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());

    engine.set_start_stage(StageName::new("writing")).unwrap();
    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("writing"));
}

#[test]
fn test_set_start_stage_not_registered() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());

    let result = engine.set_start_stage(StageName::new("nonexistent"));
    assert!(result.is_err());
}

#[test]
fn test_start_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.set_start_stage(StageName::new("writing")).unwrap();

    let inst = engine.start_stage().unwrap();
    assert_eq!(inst.status, StageStatus::Running);
}

#[test]
fn test_complete_stage_auto_gate_advances() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    engine.set_start_stage(StageName::new("writing")).unwrap();
    let _ = engine.start_stage();

    let result = serde_json::json!({"output": "章节正文"});
    engine.complete_stage(result).unwrap();

    // Auto gate 应该直接推进到 review
    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("review"));
}

#[test]
fn test_complete_stage_conditional_gate_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    engine.register_stage(polish_stage());
    engine.set_start_stage(StageName::new("review")).unwrap();
    let _ = engine.start_stage();

    let result = serde_json::json!({"consistency_score": 85});
    engine.complete_stage(result).unwrap();

    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("polish"));
}

#[test]
fn test_complete_stage_conditional_gate_fail_goes_back() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    engine.set_start_stage(StageName::new("review")).unwrap();
    let _ = engine.start_stage();

    let result = serde_json::json!({"consistency_score": 60});
    engine.complete_stage(result).unwrap();

    // 条件不满足，应退回到 writing
    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("writing"));
}

#[test]
fn test_tool_access_check() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.set_start_stage(StageName::new("writing")).unwrap();

    assert!(engine.check_tool_access("write_prose"));
    assert!(!engine.check_tool_access("modify_settings"));
    assert!(!engine.check_tool_access("unknown_tool"));
}

#[test]
fn test_inject_memo() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.inject_memo("conflict", "主角对决反派").unwrap();
    assert_eq!(engine.memo.get("conflict"), Some("主角对决反派"));
}

#[test]
fn test_build_state() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.set_start_stage(StageName::new("writing")).unwrap();
    engine.inject_memo("key", "value").unwrap();

    let state = engine.build_state();
    assert_eq!(state.current_stage.as_deref(), Some("writing"));
    assert_eq!(state.memo.get("key"), Some(&"value".to_string()));
}

#[test]
fn test_save_and_recover_state() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    engine.set_start_stage(StageName::new("writing")).unwrap();
    engine.inject_memo("conflict", "核心冲突").unwrap();

    // 模拟阶段执行
    let _ = engine.start_stage();
    let result = serde_json::json!({});
    engine.complete_stage(result).unwrap();

    // 保存状态
    engine.save_state().unwrap();

    // 创建新引擎并恢复
    let mut engine2 = HarnessEngine::new(tmp.path());
    engine2.register_stage(writing_stage());
    engine2.register_stage(review_stage());
    let recovered = engine2.recover_from_crash().unwrap();
    assert!(recovered);
    assert_eq!(engine2.memo.get("conflict"), Some("核心冲突"));
}

#[test]
fn test_max_retries_exceeded() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(writing_stage());
    engine.register_stage(review_stage());
    engine.set_start_stage(StageName::new("review")).unwrap();
    let _ = engine.start_stage();

    // 第一次失败：attempt 1 -> 2，advance to writing
    let result = serde_json::json!({"consistency_score": 50});
    engine.complete_stage(result).unwrap();
    // 现在 current_stage = writing，auto pass -> review
    let result = serde_json::json!({"output": "rewrite"});
    engine.complete_stage(result).unwrap();

    // 第二次失败：attempt 2 -> 3，advance to writing
    let result = serde_json::json!({"consistency_score": 50});
    engine.complete_stage(result).unwrap();
    // auto pass -> review
    let result = serde_json::json!({"output": "rewrite2"});
    engine.complete_stage(result).unwrap();

    // 第三次失败：attempt 3，3 > 2，应该标记 Failed
    let result = serde_json::json!({"consistency_score": 50});
    engine.complete_stage(result).unwrap();

    let inst = engine.stages_status.get("review").unwrap();
    assert_eq!(inst.status, StageStatus::Failed);
}

// ── Manual 门控（带外人工批准）测试 ─────────────────────────────────────

fn manual_stage() -> Stage {
    Stage {
        name: StageName::new("approve"),
        display_name: "人工审批".into(),
        gate_type: GateType::Manual,
        next_stage: Some(StageName::new("polish")),
        ..Stage::default()
    }
}

#[test]
fn test_manual_gate_waits_for_out_of_band_approval() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(manual_stage());
    engine.register_stage(polish_stage());
    engine.set_start_stage(StageName::new("approve")).unwrap();
    let _ = engine.start_stage();

    // AI 试图通过 result 伪造人工批准 —— 不应放行
    let result = serde_json::json!({"human_approved": true});
    engine.complete_stage(result).unwrap();

    // 引擎应停留在 approve 阶段，实例状态为 WaitingHuman
    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("approve"));
    let inst = engine.stages_status.get("approve").unwrap();
    assert_eq!(inst.status, StageStatus::WaitingHuman);
}

#[test]
fn test_manual_gate_passes_after_approval() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());
    engine.register_stage(manual_stage());
    engine.register_stage(polish_stage());
    engine.set_start_stage(StageName::new("approve")).unwrap();
    let _ = engine.start_stage();

    // 带外人工批准
    engine
        .approve_manual_gate(&StageName::new("approve"))
        .unwrap();
    assert!(engine.is_manually_approved(&StageName::new("approve")));

    let result = serde_json::json!({});
    engine.complete_stage(result).unwrap();

    // 批准后放行，推进到 polish，且批准被消费
    assert_eq!(engine.current_stage().map(|n| n.as_str()), Some("polish"));
    assert!(!engine.is_manually_approved(&StageName::new("approve")));
}

#[test]
fn test_approve_manual_gate_unknown_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let mut engine = HarnessEngine::new(tmp.path());

    let result = engine.approve_manual_gate(&StageName::new("nonexistent"));
    assert!(matches!(result, Err(PensoulError::StageNotFound(_))));
}
