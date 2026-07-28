/// 角色视图状态
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterViewState {
    /// 当前选中的角色 ID
    pub selected_character: Option<String>,
    /// 角色列表
    pub characters: Vec<CharacterItem>,
    /// 关系图数据
    pub relationships: Vec<RelationshipItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterItem {
    pub id: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipItem {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

impl CharacterViewState {
    pub fn new() -> Self {
        Self {
            selected_character: None,
            characters: Vec::new(),
            relationships: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.selected_character = None;
        self.characters.clear();
        self.relationships.clear();
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

impl Default for CharacterViewState {
    fn default() -> Self {
        Self::new()
    }
}
