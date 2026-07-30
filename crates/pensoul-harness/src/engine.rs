/// 引擎核心 — 确定性流程引擎的中枢。
///
/// `HarnessEngine` 管理阶段注册、状态机推进、门控判定、
/// WAL 审计和崩溃恢复。AI 无权跳步，一切由引擎驱动。
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::gate::GateEvaluator;
use crate::memo::RollingMemo;
use crate::stage::{GateType, Stage, StageInstance, StageStatus};
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
    /// 已收到带外人工批准的阶段名称集合（Manual 门控用）。
    manual_approvals: HashSet<String>,
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
            manual_approvals: HashSet::new(),
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

        // WAL 是审计主干：写不进去就拒绝变更，而不是静默继续
        self.wal.write_mut(
            WalAction::Advance,
            Some(stage_name.as_str()),
            Some("设置起始阶段"),
        )?;

        Ok(())
    }

    /// 注入滚动备忘录条目。
    ///
    /// 注入后自动写入 WAL 记录；WAL 写失败时返回错误。
    pub fn inject_memo(&mut self, key: &str, value: &str) -> Result<()> {
        self.memo.inject(key, value);

        self.wal.write_mut(
            WalAction::MemoInject,
            None,
            Some(&serde_json::json!({"key": key, "value": value}).to_string()),
        )?;

        Ok(())
    }

    /// 人工批准指定阶段的 Manual 门控（带外确认通道）。
    ///
    /// Manual 门控只认这里登记的批准，不看阶段产出中的任何字段，
    /// 防止 AI 通过构造 `human_approved: true` 自我放行。
    pub fn approve_manual_gate(&mut self, stage_name: &StageName) -> Result<()> {
        if !self.stages.contains_key(stage_name) {
            return Err(PensoulError::StageNotFound(stage_name.to_string()));
        }
        self.manual_approvals.insert(stage_name.to_string());
        self.wal.write_mut(
            WalAction::GatePass,
            Some(stage_name.as_str()),
            Some("收到带外人工批准"),
        )?;
        Ok(())
    }

    /// 查询指定阶段是否已收到人工批准。
    pub fn is_manually_approved(&self, stage_name: &StageName) -> bool {
        self.manual_approvals.contains(stage_name.as_str())
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

    /// 启动当前阶段，返回阶段实例快照。
    ///
    /// # 逻辑
    /// 1. 检查当前阶段是否已注册
    /// 2. 标记实例为 Running
    /// 3. 写入 WAL StageStart
    /// 4. 返回实例的克隆快照（调用方随后通过 `complete_stage` 提交结果）
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

        self.wal
            .write_mut(WalAction::StageStart, Some(current.as_str()), None)?;

        Ok(inst.clone())
    }

    /// 完成当前阶段，执行门控判定并推进/回退。
    ///
    /// # 参数
    /// - `result`: 阶段产出结果（JSON 语义化数据）。
    ///
    /// # 返回值
    /// - `Ok(())` — 流程推进成功，或 Manual 门控进入等待人工确认。
    /// - `Err` — 阶段未注册、门控逻辑异常或 WAL 写入失败。
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
        self.wal
            .write_mut(WalAction::StageComplete, Some(current.as_str()), None)?;

        // 执行门控判定（Manual 门控只认带外人工批准）
        let manual_approved = self.manual_approvals.contains(current.as_str());
        let gate_result = GateEvaluator::evaluate(&stage, &result, manual_approved)?;

        // Manual 门控未获批准：进入等待人工确认，既不退进也不计失败
        if stage.gate_type == GateType::Manual && !gate_result.passed {
            self.wal.write_mut(
                WalAction::GateFail,
                Some(current.as_str()),
                Some("等待人工确认（带外批准尚未到达）"),
            )?;
            if let Some(inst) = self.stages_status.get_mut(current.as_str()) {
                inst.gate_result = Some(gate_result);
                inst.mark_waiting_human();
            }
            return Ok(());
        }

        // 门控通过：消费掉该阶段的人工批准
        if gate_result.passed {
            self.manual_approvals.remove(current.as_str());
        }

        if gate_result.passed {
            // 门控通过
            self.wal.write_mut(
                WalAction::GatePass,
                Some(current.as_str()),
                Some(&gate_result.reason),
            )?;

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
                self.wal
                    .write_mut(WalAction::HarnessComplete, None, Some("所有阶段完成"))?;
            }
        } else {
            // 门控未通过
            self.wal.write_mut(
                WalAction::GateFail,
                Some(current.as_str()),
                Some(&gate_result.reason),
            )?;

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

        self.wal
            .write_mut(WalAction::Advance, Some(target.as_str()), None)?;

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

        self.wal.write_mut(WalAction::StateSync, None, None)?;

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
