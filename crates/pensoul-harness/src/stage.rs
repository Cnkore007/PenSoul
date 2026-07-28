/// 阶段定义与实例 — 确定性流程引擎的基础类型。
///
/// 每个 `Stage` 是一张声明式"任务卡"，描述创作流程中的一个步骤。
/// `StageInstance` 则记录该步骤在运行时的状态快照。
use pensoul_core::StageName;

// ── 类型定义 ──────────────────────────────────────────────────────────────

/// 门控类型：决定阶段完成后如何推进。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateType {
    /// 自动放行 — 完成后立即进下一站（高速公路 ETC）。
    Auto,
    /// 人工放行 — 必须等用户确认（收费站窗口）。
    Manual,
    /// 条件放行 — 根据检查结果决定（检查站）。
    Conditional,
}

/// 执行者类型：决定阶段由谁来执行。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunnerType {
    /// 本机执行 — 在本地进程中运行。
    Local,
    /// 委托执行 — 委托给独立 Agent 或外部服务。
    Delegated,
}

/// 阶段运行状态：状态机的当前节点。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StageStatus {
    /// 等待开始。
    Pending,
    /// 正在执行中。
    Running,
    /// 等待门控判定。
    WaitingGate,
    /// 等待人工确认。
    WaitingHuman,
    /// 已完成。
    Completed,
    /// 执行失败。
    Failed,
    /// 被阻塞（依赖未满足或永久性错误）。
    Blocked,
}

// ── 阶段定义 ──────────────────────────────────────────────────────────────

/// 创作阶段的声明式定义。
///
/// 包含四要素：工作手册、工具白名单、门控配置、流转路径。
/// 所有字段可通过 `Default` trait 获取合理默认值，再按需覆盖。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stage {
    /// 阶段内部名称（唯一标识）。
    pub name: StageName,
    /// 阶段显示名称（面向用户）。
    pub display_name: String,
    /// 工作手册：描述该阶段的目标和方法。
    pub manual: String,
    /// 允许使用的工具列表（白名单）。
    pub tools_allowed: Vec<String>,
    /// 禁止使用的工具列表（黑名单，优先级高于白名单）。
    pub tools_denied: Vec<String>,
    /// 门控类型。
    pub gate_type: GateType,
    /// 条件放行时的条件表达式（如 `consistency_score >= 80`）。
    pub gate_condition: Option<String>,
    /// 下一阶段名称（None 表示流程结束）。
    pub next_stage: Option<StageName>,
    /// 门控失败时的回退阶段（None 表示直接标记失败）。
    pub on_fail: Option<StageName>,
    /// 执行者类型。
    pub runner: RunnerType,
    /// 最大重试次数（0 表示不重试）。
    pub max_retries: u32,
    /// 阶段超时时间（秒），None 表示不限时。
    pub timeout_secs: Option<u64>,
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            name: StageName::default(),
            display_name: String::new(),
            manual: String::new(),
            tools_allowed: Vec::new(),
            tools_denied: Vec::new(),
            gate_type: GateType::Auto,
            gate_condition: None,
            next_stage: None,
            on_fail: None,
            runner: RunnerType::Local,
            max_retries: 0,
            timeout_secs: None,
        }
    }
}

// ── 阶段实例 ──────────────────────────────────────────────────────────────

/// 阶段的运行时实例，记录单次执行的完整生命周期。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StageInstance {
    /// 所属阶段的名称。
    pub stage_name: StageName,
    /// 当前状态。
    pub status: StageStatus,
    /// 当前尝试次数（从 1 开始）。
    pub attempt: u32,
    /// 开始时间（Unix 时间戳秒数）。
    pub started_at: Option<f64>,
    /// 完成时间（Unix 时间戳秒数）。
    pub completed_at: Option<f64>,
    /// 阶段产出结果（JSON 语义化数据）。
    pub result: Option<serde_json::Value>,
    /// 门控判定结果。
    pub gate_result: Option<GateResult>,
    /// 错误信息。
    pub error: Option<String>,
}

/// 门控判定的结构化结果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateResult {
    /// 是否通过门控。
    pub passed: bool,
    /// 门控分数（仅条件放行时有值）。
    pub score: Option<f64>,
    /// 门控判定的原因描述。
    pub reason: String,
}

impl StageInstance {
    /// 创建新的阶段实例。
    ///
    /// 初始状态为 `Pending`，尝试次数为 1。
    pub fn new(stage_name: StageName) -> Self {
        Self {
            stage_name,
            status: StageStatus::Pending,
            attempt: 1,
            started_at: None,
            completed_at: None,
            result: None,
            gate_result: None,
            error: None,
        }
    }

    /// 标记阶段为运行中。
    pub fn mark_running(&mut self) {
        self.status = StageStatus::Running;
        self.started_at = Some(now_timestamp());
    }

    /// 标记阶段为等待门控。
    pub fn mark_waiting_gate(&mut self) {
        self.status = StageStatus::WaitingGate;
    }

    /// 标记阶段为等待人工确认。
    pub fn mark_waiting_human(&mut self) {
        self.status = StageStatus::WaitingHuman;
    }

    /// 标记阶段完成。
    pub fn mark_completed(&mut self) {
        self.status = StageStatus::Completed;
        self.completed_at = Some(now_timestamp());
    }

    /// 标记阶段失败。
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = StageStatus::Failed;
        self.completed_at = Some(now_timestamp());
        self.error = Some(error.into());
    }

    /// 标记阶段被阻塞。
    pub fn mark_blocked(&mut self, error: impl Into<String>) {
        self.status = StageStatus::Blocked;
        self.completed_at = Some(now_timestamp());
        self.error = Some(error.into());
    }

    /// 是否可以重试（尚未超过最大重试次数）。
    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.attempt <= max_retries
    }

    /// 递增尝试次数。
    pub fn increment_attempt(&mut self) {
        self.attempt += 1;
    }
}

/// 获取当前 Unix 时间戳（秒，浮点数）。
fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_default() {
        let stage = Stage::default();
        assert_eq!(stage.gate_type, GateType::Auto);
        assert_eq!(stage.runner, RunnerType::Local);
        assert_eq!(stage.max_retries, 0);
        assert!(stage.tools_allowed.is_empty());
        assert!(stage.tools_denied.is_empty());
    }

    #[test]
    fn test_stage_instance_lifecycle() {
        let name = StageName::new("writing");
        let mut inst = StageInstance::new(name.clone());
        assert_eq!(inst.status, StageStatus::Pending);
        assert_eq!(inst.attempt, 1);
        assert!(inst.started_at.is_none());

        inst.mark_running();
        assert_eq!(inst.status, StageStatus::Running);
        assert!(inst.started_at.is_some());

        inst.mark_waiting_gate();
        assert_eq!(inst.status, StageStatus::WaitingGate);

        inst.mark_completed();
        assert_eq!(inst.status, StageStatus::Completed);
        assert!(inst.completed_at.is_some());
    }

    #[test]
    fn test_can_retry() {
        let mut inst = StageInstance::new(StageName::new("test"));
        assert!(inst.can_retry(3));
        inst.increment_attempt();
        assert!(inst.can_retry(3));
        inst.increment_attempt();
        inst.increment_attempt();
        assert!(!inst.can_retry(3));
    }

    #[test]
    fn test_mark_failed_sets_error() {
        let mut inst = StageInstance::new(StageName::new("test"));
        inst.mark_failed("something went wrong");
        assert_eq!(inst.status, StageStatus::Failed);
        assert_eq!(inst.error.as_deref(), Some("something went wrong"));
    }
}
