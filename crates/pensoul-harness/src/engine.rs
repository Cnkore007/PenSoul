/// 引擎核心 — 确定性流程引擎的中枢。
///
/// `HarnessEngine` 管理阶段注册、状态机推进、门控判定、
/// WAL 审计和崩溃恢复。AI 无权跳步，一切由引擎驱动。
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::gate::GateEvaluator;
use crate::memo::RollingMemo;
use crate::stage::{Stage, StageInstance, StageStatus};
use crate::tools::ToolWhitelist;
use crate::wal::{WalAction, WalManager};
use pensoul_core::{PensoulError, Result, StageName};

// ── 引擎状态快照 ──────────────────────────────────────────────────────────

/// 引擎状态快照，用于持久化和崩溃恢复。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EngineState {
    /// 当前阶段名称。
    pub current_stage: Option<String>,
    /// 滚动备忘录内容。
    pub memo: HashMap<String, String>,
    /// 所有阶段的状态记录。
    pub stages_status: HashMap<String, StageInstance>,
}

// ── Harness 引擎 ──────────────────────────────────────────────────────────

/// 确定性流程引擎 — 小说创作的调度中枢。
///
/// 核心职责：
/// - 管理阶段注册与拓扑
/// - 驱动阶段状态机的确定性流转
/// - 执行门控判定（Auto/Manual/Conditional）
/// - 管理工具白名单
/// - 维护 WAL 审计轨迹
/// - 支持崩溃恢复
pub struct HarnessEngine {
    /// 项目目录路径。
    project_dir: PathBuf,
    /// 已注册的阶段定义。
    stages: HashMap<StageName, Stage>,
    /// 当前活跃阶段的名称。
    current_stage: Option<StageName>,
    /// 滚动备忘录。
    pub memo: RollingMemo,
    /// 所有阶段的运行实例。
    pub stages_status: HashMap<String, StageInstance>,
    /// WAL 管理器。
    pub wal: WalManager,
}

impl HarnessEngine {
    /// 创建新的 Harness 引擎。
    ///
    /// 会创建 `{project_dir}/.harness/` 目录用于存储 WAL 和状态文件。
    pub fn new(project_dir: &Path) -> Self {
        let harness_dir = project_dir.join(".harness");
        std::fs::create_dir_all(&harness_dir).ok();

        let wal = WalManager::new(project_dir);

        let mut engine = Self {
            project_dir: project_dir.to_path_buf(),
            stages: HashMap::new(),
            current_stage: None,
            memo: RollingMemo::new(),
            stages_status: HashMap::new(),
            wal,
        };

        // 写入引擎初始化 WAL
        if let Err(e) = engine.wal.write_mut(
            WalAction::EngineInit,
            None,
            Some(&format!("引擎创建: {}", project_dir.display())),
        ) {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        engine
    }

    /// 注册一个创作阶段。
    ///
    /// 重复注册同名阶段会覆盖旧定义。
    pub fn register_stage(&mut self, stage: Stage) {
        let name = stage.name.clone();
        self.stages.insert(name.clone(), stage);

        // 如果还没有初始实例，创建一个
        if !self.stages_status.contains_key(name.as_str()) {
            self.stages_status
                .insert(name.to_string(), StageInstance::new(name));
        }
    }

    /// 设置起始阶段。
    ///
    /// 必须在 `start_stage()` 之前调用。设置后会写入 WAL。
    pub fn set_start_stage(&mut self, stage_name: StageName) -> Result<()> {
        if !self.stages.contains_key(&stage_name) {
            return Err(PensoulError::StageNotFound(stage_name.to_string()));
        }

        self.current_stage = Some(stage_name.clone());

        if let Err(e) = self.wal.write_mut(
            WalAction::Advance,
            Some(stage_name.as_str()),
            Some("设置起始阶段"),
        ) {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        Ok(())
    }

    /// 注入滚动备忘录条目。
    ///
    /// 注入后自动写入 WAL 记录。
    pub fn inject_memo(&mut self, key: &str, value: &str) {
        self.memo.inject(key, value);

        if let Err(e) = self.wal.write_mut(
            WalAction::MemoInject,
            None,
            Some(&serde_json::json!({"key": key, "value": value}).to_string()),
        ) {
            eprintln!("[警告] WAL 写入失败: {e}");
        }
    }

    /// 检查当前阶段是否允许使用指定工具。
    ///
    /// 如果当前没有活跃阶段，返回 `false`。
    pub fn check_tool_access(&self, tool_name: &str) -> bool {
        let current = match &self.current_stage {
            Some(name) => name,
            None => return false,
        };

        let stage = match self.stages.get(current) {
            Some(s) => s,
            None => return false,
        };

        ToolWhitelist::check_access(stage, tool_name, &self.wal, current.as_str())
    }

    /// 启动当前阶段，返回阶段实例。
    ///
    /// # 逻辑
    /// 1. 检查当前阶段是否已注册
    /// 2. 标记实例为 Running
    /// 3. 写入 WAL StageStart
    /// 4. 返回可变引用供调用方填充 result
    pub fn start_stage(&mut self) -> Result<StageInstance> {
        let current = self
            .current_stage
            .clone()
            .ok_or_else(|| PensoulError::Internal("未设置起始阶段".into()))?;

        if !self.stages.contains_key(&current) {
            return Err(PensoulError::StageNotFound(current.to_string()));
        }

        let inst = self
            .stages_status
            .get_mut(current.as_str())
            .ok_or_else(|| PensoulError::StageNotFound(current.to_string()))?;

        inst.mark_running();

        if let Err(e) = self
            .wal
            .write_mut(WalAction::StageStart, Some(current.as_str()), None)
        {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        Ok(inst.clone())
    }

    /// 完成当前阶段，执行门控判定并推进/回退。
    ///
    /// # 参数
    /// - `result`: 阶段产出结果（JSON 语义化数据）。
    ///
    /// # 返回值
    /// - `Ok(())` — 流程推进成功。
    /// - `Err` — 阶段未注册或门控逻辑异常。
    pub fn complete_stage(&mut self, result: serde_json::Value) -> Result<()> {
        let current = self
            .current_stage
            .clone()
            .ok_or_else(|| PensoulError::Internal("未设置当前阶段".into()))?;

        let stage = self
            .stages
            .get(&current)
            .ok_or_else(|| PensoulError::StageNotFound(current.to_string()))?
            .clone();

        // 更新实例状态为 WaitingGate
        if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
            inst.mark_waiting_gate();
            inst.result = Some(result.clone());
        }

        // 写入 StageComplete WAL
        if let Err(e) = self
            .wal
            .write_mut(WalAction::StageComplete, Some(current.as_str()), None)
        {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        // 执行门控判定
        let gate_result = GateEvaluator::evaluate(&stage, &result)?;

        if gate_result.passed {
            // 门控通过
            if let Err(e) = self.wal.write_mut(
                WalAction::GatePass,
                Some(current.as_str()),
                Some(&gate_result.reason),
            ) {
                eprintln!("[警告] WAL 写入失败: {e}");
            }

            // 更新实例门控结果
            if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                inst.gate_result = Some(gate_result.clone());
                inst.mark_completed();
            }

            // 推进到下一阶段
            if let Some(ref next_name) = stage.next_stage {
                let next_name = next_name.clone();
                self.advance_to_stage(&next_name)?;
            } else {
                // 流程结束
                if let Err(e) =
                    self.wal
                        .write_mut(WalAction::HarnessComplete, None, Some("所有阶段完成"))
                {
                    eprintln!("[警告] WAL 写入失败: {e}");
                }
            }
        } else {
            // 门控未通过
            if let Err(e) = self.wal.write_mut(
                WalAction::GateFail,
                Some(current.as_str()),
                Some(&gate_result.reason),
            ) {
                eprintln!("[警告] WAL 写入失败: {e}");
            }

            if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                inst.gate_result = Some(gate_result.clone());
            }

            // 检查是否需要回退
            if let Some(ref on_fail) = stage.on_fail {
                let on_fail = on_fail.clone();

                // 检查最大重试次数
                let (max_retries, attempt) =
                    if let Some(inst) = self.stages_status.get(current.as_str()) {
                        (stage.max_retries, inst.attempt)
                    } else {
                        (0, 1)
                    };

                if attempt <= max_retries {
                    // 可重试：回到失败阶段并递增尝试次数
                    if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                        inst.increment_attempt();
                        inst.status = StageStatus::Pending;
                    }
                    self.advance_to_stage(&on_fail)?;
                } else {
                    // 超过重试次数，标记失败
                    if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                        inst.mark_failed(format!(
                            "超过最大重试次数 ({max_retries}): {}",
                            gate_result.reason
                        ));
                    }
                }
            } else {
                // 无回退目标，标记失败
                if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                    inst.mark_failed(format!("门控未通过且无回退目标: {}", gate_result.reason));
                }
            }
        }

        Ok(())
    }

    /// 推进到指定阶段。
    fn advance_to_stage(&mut self, target: &StageName) -> Result<()> {
        if !self.stages.contains_key(target) {
            return Err(PensoulError::StageNotFound(target.to_string()));
        }

        if let Err(e) = self
            .wal
            .write_mut(WalAction::Advance, Some(target.as_str()), None)
        {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        self.current_stage = Some(target.clone());

        // 重置目标阶段实例为 Pending
        if let Some(inst) = self.stages_status.get_mut(target.as_str()) {
            inst.status = StageStatus::Pending;
        }

        Ok(())
    }

    /// 构建当前引擎状态快照。
    pub fn build_state(&self) -> EngineState {
        EngineState {
            current_stage: self.current_stage.as_ref().map(|n| n.to_string()),
            memo: self.memo.entries().clone(),
            stages_status: self.stages_status.clone(),
        }
    }

    /// 持久化引擎状态快照。
    pub fn save_state(&mut self) -> Result<()> {
        let state = self.build_state();
        let value = serde_json::to_value(&state)
            .map_err(|e| PensoulError::SerializationError(format!("序列化状态失败: {e}")))?;
        self.wal.save_state(&value)?;

        if let Err(e) = self.wal.write_mut(WalAction::StateSync, None, None) {
            eprintln!("[警告] WAL 写入失败: {e}");
        }

        Ok(())
    }

    /// 尝试从崩溃中恢复。
    ///
    /// 调用 `CrashRecovery` 执行 WAL 重放。
    pub fn recover_from_crash(&mut self) -> Result<bool> {
        crate::recovery::CrashRecovery::recover(self)
    }

    /// 获取当前阶段名称的引用。
    pub fn current_stage(&self) -> Option<&StageName> {
        self.current_stage.as_ref()
    }

    /// 获取阶段定义的引用。
    pub fn get_stage(&self, name: &StageName) -> Option<&Stage> {
        self.stages.get(name)
    }

    /// 设置当前阶段（仅用于崩溃恢复）。
    pub fn set_current_stage(&mut self, stage_name: pensoul_core::StageName) {
        self.current_stage = Some(stage_name);
    }

    /// 获取项目目录路径。
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// 获取已注册阶段的数量。
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::{GateType, RunnerType};

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
        assert_eq!(inst.status, crate::stage::StageStatus::Running);
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
        engine.inject_memo("conflict", "主角对决反派");
        assert_eq!(engine.memo.get("conflict"), Some("主角对决反派"));
    }

    #[test]
    fn test_build_state() {
        let tmp = tempfile::tempdir().unwrap();
        let mut engine = HarnessEngine::new(tmp.path());
        engine.register_stage(writing_stage());
        engine.set_start_stage(StageName::new("writing")).unwrap();
        engine.inject_memo("key", "value");

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
        engine.inject_memo("conflict", "核心冲突");

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

        // review 的 max_retries=2，需要 gate_fail 3 次才能 Failed
        // 但 complete_stage 会 advance 到 writing（auto pass）再回到 review，
        // 所以我们直接操作 review 实例来模拟多次失败。

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
}
