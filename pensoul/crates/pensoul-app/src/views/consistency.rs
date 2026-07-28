/// 一致性检查视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyViewState {
    /// 检查报告
    pub report: Option<ConsistencyReportView>,
    /// 违反记录
    pub violations: Vec<ViolationView>,
    /// 检查进度
    pub is_checking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReportView {
    pub total_entities_checked: usize,
    pub total_violations: usize,
    pub check_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationView {
    pub entity_id: String,
    pub severity: String,
    pub description: String,
    pub chapter_a: i64,
    pub chapter_b: i64,
}

impl ConsistencyViewState {
    pub fn new() -> Self {
        Self {
            report: None,
            violations: Vec::new(),
            is_checking: false,
        }
    }

    pub fn reset(&mut self) {
        self.report = None;
        self.violations.clear();
        self.is_checking = false;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for ConsistencyViewState {
    fn default() -> Self {
        Self::new()
    }
}
