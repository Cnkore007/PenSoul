/// 一致性检查范围模块
use crate::entity_state::EntityType;

/// 一致性检查范围
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConsistencyCheckScope {
    /// 仅当前章节
    ChapterOnly,
    /// 当前章节及相邻章节
    ChapterPlusNeighbors,
    /// 全书
    FullBook,
}

/// 根据实体类型确定检查范围
pub fn determine_scope(entity_type: &EntityType) -> ConsistencyCheckScope {
    match entity_type {
        EntityType::Character => ConsistencyCheckScope::ChapterOnly,
        EntityType::Setting => ConsistencyCheckScope::FullBook,
        EntityType::Timeline => ConsistencyCheckScope::ChapterPlusNeighbors,
        EntityType::Foreshadow => ConsistencyCheckScope::ChapterPlusNeighbors,
        EntityType::Event => ConsistencyCheckScope::ChapterOnly,
        EntityType::Plot => ConsistencyCheckScope::ChapterOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_scope() {
        assert_eq!(determine_scope(&EntityType::Character), ConsistencyCheckScope::ChapterOnly);
    }

    #[test]
    fn test_setting_scope() {
        assert_eq!(determine_scope(&EntityType::Setting), ConsistencyCheckScope::FullBook);
    }

    #[test]
    fn test_timeline_scope() {
        assert_eq!(determine_scope(&EntityType::Timeline), ConsistencyCheckScope::ChapterPlusNeighbors);
    }

    #[test]
    fn test_foreshadow_scope() {
        assert_eq!(determine_scope(&EntityType::Foreshadow), ConsistencyCheckScope::ChapterPlusNeighbors);
    }

    #[test]
    fn test_event_scope() {
        assert_eq!(determine_scope(&EntityType::Event), ConsistencyCheckScope::ChapterOnly);
    }

    #[test]
    fn test_plot_scope() {
        assert_eq!(determine_scope(&EntityType::Plot), ConsistencyCheckScope::ChapterOnly);
    }
}
