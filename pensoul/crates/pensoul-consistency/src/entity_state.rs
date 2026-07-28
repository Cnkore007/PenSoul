/// 实体状态管理模块
use std::collections::HashMap;

/// 实体类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    /// 角色
    Character,
    /// 世界观设定
    Setting,
    /// 时间线
    Timeline,
    /// 事件
    Event,
    /// 情节
    Plot,
    /// 伏笔
    Foreshadow,
}

/// 实体状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityState {
    /// 实体 ID
    pub entity_id: String,
    /// 实体类型
    pub entity_type: EntityType,
    /// 所属章节 ID
    pub chapter_id: i64,
    /// 状态数据
    pub state_data: serde_json::Value,
    /// 版本号
    pub version: i32,
}

/// 实体状态管理器
#[derive(Debug, Default)]
pub struct EntityStateManager {
    /// entity_id -> 状态列表（按章节）
    states: HashMap<String, Vec<EntityState>>,
}

impl EntityStateManager {
    /// 创建新的状态管理器
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// 注册一个状态
    pub fn register_state(&mut self, state: EntityState) {
        self.states
            .entry(state.entity_id.clone())
            .or_default()
            .push(state);
    }

    /// 获取实体的所有状态
    pub fn get_state(&self, entity_id: &str) -> Option<&Vec<EntityState>> {
        self.states.get(entity_id)
    }

    /// 获取实体在特定章节的状态
    pub fn get_state_in_chapter(&self, entity_id: &str, chapter_id: i64) -> Option<&EntityState> {
        self.states
            .get(entity_id)?
            .iter()
            .find(|s| s.chapter_id == chapter_id)
    }

    /// 获取所有指定类型的实体 ID
    pub fn get_all_entity_ids_by_type(&self, entity_type: &EntityType) -> Vec<String> {
        self.states
            .iter()
            .filter(|(_, states)| states.iter().any(|s| s.entity_type == *entity_type))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 获取指定实体在指定章节范围内的所有状态
    pub fn get_states_in_chapter_range(
        &self,
        entity_id: &str,
        start_chapter: i64,
        end_chapter: i64,
    ) -> Vec<&EntityState> {
        self.states
            .get(entity_id)
            .map(|states| {
                states
                    .iter()
                    .filter(|s| s.chapter_id >= start_chapter && s.chapter_id <= end_chapter)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取实体状态总数
    pub fn total_states(&self) -> usize {
        self.states.values().map(|v| v.len()).sum()
    }

    /// 获取实体总数
    pub fn total_entities(&self) -> usize {
        self.states.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_state(entity_id: &str, entity_type: EntityType, chapter_id: i64, version: i32) -> EntityState {
        EntityState {
            entity_id: entity_id.to_string(),
            entity_type,
            chapter_id,
            state_data: json!({"name": "Test"}),
            version,
        }
    }

    #[test]
    fn test_register_and_get_state() {
        let mut manager = EntityStateManager::new();
        let state = make_state("char_1", EntityType::Character, 1, 1);
        manager.register_state(state);

        let states = manager.get_state("char_1").unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].chapter_id, 1);
    }

    #[test]
    fn test_get_state_in_chapter() {
        let mut manager = EntityStateManager::new();
        manager.register_state(make_state("char_1", EntityType::Character, 1, 1));
        manager.register_state(make_state("char_1", EntityType::Character, 2, 1));

        let state = manager.get_state_in_chapter("char_1", 2).unwrap();
        assert_eq!(state.chapter_id, 2);
    }

    #[test]
    fn test_get_all_entity_ids_by_type() {
        let mut manager = EntityStateManager::new();
        manager.register_state(make_state("char_1", EntityType::Character, 1, 1));
        manager.register_state(make_state("char_2", EntityType::Character, 1, 1));
        manager.register_state(make_state("event_1", EntityType::Event, 1, 1));

        let chars = manager.get_all_entity_ids_by_type(&EntityType::Character);
        assert_eq!(chars.len(), 2);
    }

    #[test]
    fn test_get_states_in_chapter_range() {
        let mut manager = EntityStateManager::new();
        manager.register_state(make_state("char_1", EntityType::Character, 1, 1));
        manager.register_state(make_state("char_1", EntityType::Character, 3, 1));
        manager.register_state(make_state("char_1", EntityType::Character, 5, 1));

        let states = manager.get_states_in_chapter_range("char_1", 2, 4);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].chapter_id, 3);
    }

    #[test]
    fn test_total_counts() {
        let mut manager = EntityStateManager::new();
        manager.register_state(make_state("char_1", EntityType::Character, 1, 1));
        manager.register_state(make_state("char_1", EntityType::Character, 2, 1));
        manager.register_state(make_state("char_2", EntityType::Character, 1, 1));

        assert_eq!(manager.total_entities(), 2);
        assert_eq!(manager.total_states(), 3);
    }
}
