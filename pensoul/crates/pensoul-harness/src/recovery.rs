/// 崩溃恢复 — 基于 WAL 重放还原引擎状态。
///
/// 恢复流程：
/// 1. 读取 WAL 文件中的所有条目
/// 2. 校验每个条目的 blake3 校验和
/// 3. 按时间顺序重放条目，还原引擎的阶段状态和备忘录
use crate::engine::{EngineState, HarnessEngine};
use crate::wal::{WalAction, WalManager};
use pensoul_core::{PensoulError, Result};

/// 崩溃恢复管理器。
///
/// 负责在引擎启动时检测异常退出并恢复状态。
#[derive(Debug, Clone)]
pub struct CrashRecovery;

impl CrashRecovery {
    /// 执行崩溃恢复流程。
    ///
    /// # 流程
    /// 1. 加载 WAL 条目
    /// 2. 校验所有条目的完整性（checksum）
    /// 3. 重放条目，还原引擎状态
    ///
    /// # 返回值
    /// - `Ok(true)` — 恢复成功。
    /// - `Ok(false)` — 无可恢复的数据（首次启动或 WAL 为空）。
    /// - `Err` — WAL 损坏，无法恢复。
    pub fn recover(engine: &mut HarnessEngine) -> Result<bool> {
        let entries = engine.wal.load_entries()?;

        if entries.is_empty() {
            return Ok(false);
        }

        // 校验完整性
        WalManager::verify_integrity(&entries)?;

        // 重放条目还原状态
        Self::replay_entries(engine, &entries)?;

        // 写入恢复完成标记
        engine
            .wal
            .write_mut(WalAction::EngineInit, None, Some("crash recovery completed"))?;

        Ok(true)
    }

    /// 重放 WAL 条目以还原引擎状态。
    fn replay_entries(engine: &mut HarnessEngine, entries: &[crate::wal::WalEntry]) -> Result<()> {
        for entry in entries {
            match entry.action {
                WalAction::EngineInit => {
                    // 引擎初始化事件，记录日志即可
                    tracing::info!(
                        "WAL 重放: EngineInit at {}",
                        entry.timestamp
                    );
                }

                WalAction::MemoInject => {
                    // 还原备忘录条目
                    if let Some(ref data) = entry.data {
                        // data 格式: JSON {"key":"...", "value":"..."}
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                            && let (Some(key), Some(value)) = (
                                json.get("key").and_then(|v| v.as_str()),
                                json.get("value").and_then(|v| v.as_str()),
                            ) {
                                engine.memo.inject(key, value);
                                tracing::info!(
                                    "WAL 重放: MemoInject key={key}"
                                );
                            }
                    }
                }

                WalAction::StageStart => {
                    // 标记阶段为运行中
                    if let Some(ref stage_name) = entry.stage
                        && let Some(inst) = engine.stages_status.get_mut(stage_name) {
                            inst.mark_running();
                            tracing::info!(
                                "WAL 重放: StageStart stage={stage_name}"
                            );
                        }
                }

                WalAction::StageComplete => {
                    if let Some(ref stage_name) = entry.stage
                        && let Some(inst) = engine.stages_status.get_mut(stage_name) {
                            inst.mark_completed();
                            tracing::info!(
                                "WAL 重放: StageComplete stage={stage_name}"
                            );
                        }
                }

                WalAction::GatePass => {
                    tracing::info!(
                        "WAL 重放: GatePass stage={}",
                        entry.stage.as_deref().unwrap_or("unknown")
                    );
                }

                WalAction::GateFail => {
                    tracing::info!(
                        "WAL 重放: GateFail stage={}",
                        entry.stage.as_deref().unwrap_or("unknown")
                    );
                }

                WalAction::Advance => {
                    // 还原当前阶段
                    if let Some(ref stage_name) = entry.stage {
                        engine.set_current_stage(pensoul_core::StageName::new(stage_name.clone()));
                        // 重置目标阶段实例为 Pending
                        if let Some(inst) = engine.stages_status.get_mut(stage_name) {
                            inst.status = crate::stage::StageStatus::Pending;
                        }
                    }
                    tracing::info!(
                        "WAL 重放: Advance to stage={}",
                        entry.stage.as_deref().unwrap_or("unknown")
                    );
                }

                WalAction::ToolBlocked => {
                    tracing::warn!(
                        "WAL 重放: ToolBlocked stage={} data={}",
                        entry.stage.as_deref().unwrap_or("unknown"),
                        entry.data.as_deref().unwrap_or("")
                    );
                }

                WalAction::HarnessComplete => {
                    tracing::info!("WAL 重放: HarnessComplete");
                }

                WalAction::StateSync => {
                    tracing::info!(
                        "WAL 重放: StateSync at {}",
                        entry.timestamp
                    );
                }
            }
        }

        Ok(())
    }

    /// 尝试从磁盘加载引擎状态快照。
    ///
    /// 如果存在有效的状态快照，直接还原；否则依赖 WAL 重放。
    pub fn load_state_snapshot(wal: &WalManager) -> Option<EngineState> {
        let state_path = wal.state_path();
        if !state_path.exists() {
            return None;
        }

        let data = std::fs::read_to_string(state_path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// 保存引擎状态快照到磁盘。
    pub fn save_state_snapshot(wal: &WalManager, state: &EngineState) -> Result<()> {
        let value = serde_json::to_value(state)
            .map_err(|e| PensoulError::SerializationError(format!("序列化引擎状态失败: {e}")))?;
        wal.save_state(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recover_empty_wal() {
        let tmp = tempfile::tempdir().unwrap();
        // 直接创建 WALManager（不经过 HarnessEngine，避免自动写入 EngineInit）
        let wal = WalManager::new(tmp.path());
        // WAL 文件不存在时 load_entries 返回空
        let entries = wal.load_entries().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_recover_with_valid_wal() {
        let tmp = tempfile::tempdir().unwrap();
        let mut engine = HarnessEngine::new(tmp.path());

        // 写入一些 WAL 条目
        engine
            .wal
            .write_mut(WalAction::EngineInit, None, Some("init"))
            .unwrap();
        engine
            .wal
            .write_mut(
                WalAction::MemoInject,
                None,
                Some(r#"{"key":"key", "value":"value"}"#),
            )
            .unwrap();

        // 恢复
        let result = CrashRecovery::recover(&mut engine).unwrap();
        assert!(result);
        // 备忘录应该被还原
        assert_eq!(engine.memo.get("key"), Some("value"));
    }

    #[test]
    fn test_state_snapshot_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = WalManager::new(tmp.path());
        let state = EngineState::default();

        CrashRecovery::save_state_snapshot(&wal, &state).unwrap();
        let loaded = CrashRecovery::load_state_snapshot(&wal);
        assert!(loaded.is_some());
    }
}
