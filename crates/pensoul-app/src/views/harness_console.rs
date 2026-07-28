/// Harness 控制台视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConsoleState {
    /// 当前阶段
    pub current_stage: Option<String>,
    /// 阶段状态列表
    pub stages: Vec<StageStatusView>,
    /// 滚动备忘录
    pub memo: Vec<MemoEntryView>,
    /// WAL 日志
    pub wal_log: Vec<WalEntryView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageStatusView {
    pub name: String,
    pub status: String,
    pub attempt: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoEntryView {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntryView {
    pub action: String,
    pub stage: Option<String>,
    pub detail: Option<String>,
}

impl HarnessConsoleState {
    pub fn new() -> Self {
        Self {
            current_stage: None,
            stages: Vec::new(),
            memo: Vec::new(),
            wal_log: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.current_stage = None;
        self.stages.clear();
        self.memo.clear();
        self.wal_log.clear();
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for HarnessConsoleState {
    fn default() -> Self {
        Self::new()
    }
}
