pub mod engine;
pub mod gate;
pub mod memo;
pub mod recovery;
pub mod runner;
/// PenSoul Harness — 确定性流程引擎。
///
/// 引擎管流程，模型管创作。AI 无权跳步，一切由引擎驱动。
///
/// # 核心概念
///
/// | 概念 | 说明 |
/// |------|------|
/// | `Stage` | 创作阶段的声明式定义（任务卡） |
/// | `StageInstance` | 阶段的运行时实例（状态快照） |
/// | `GateEvaluator` | 门控三模式评估器 |
/// | `ToolWhitelist` | 工具访问白名单 |
/// | `RollingMemo` | 跨阶段注入的滚动备忘录 |
/// | `WalManager` | Write-Ahead Log 管理器 |
/// | `RunnerMatrix` | 执行者矩阵 |
/// | `HarnessEngine` | 引擎核心，驱动整个创作流程 |
/// | `CrashRecovery` | 基于 WAL 的崩溃恢复 |
pub mod stage;
pub mod wal;

// ── 重导出公有类型 ────────────────────────────────────────────────────────

// stage.rs
pub use stage::{GateResult, GateType, RunnerType, Stage, StageInstance, StageStatus};

// gate.rs
pub use gate::GateEvaluator;

// wal.rs
pub use wal::{WalAction, WalEntry, WalManager};

// memo.rs
pub use memo::{RollingMemo, ToolWhitelist};

// engine.rs
pub use engine::{EngineState, HarnessEngine};

// runner.rs
pub use runner::{RunnerEntry, RunnerMatrix};

// recovery.rs
pub use recovery::CrashRecovery;
