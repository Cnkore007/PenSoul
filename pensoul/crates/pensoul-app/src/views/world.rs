/// 世界观视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldViewState {
    /// 地点列表
    pub locations: Vec<LocationItem>,
    /// 时间线事件
    pub timeline_events: Vec<TimelineEvent>,
    /// 世界设定规则
    pub setting_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationItem {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub name: String,
    pub chapter: i64,
    pub description: String,
}

impl WorldViewState {
    pub fn new() -> Self {
        Self {
            locations: Vec::new(),
            timeline_events: Vec::new(),
            setting_rules: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.locations.clear();
        self.timeline_events.clear();
        self.setting_rules.clear();
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for WorldViewState {
    fn default() -> Self {
        Self::new()
    }
}
