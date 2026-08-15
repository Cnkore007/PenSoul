// query.rs — 时间感知查询
// 支持"第X章时实体的状态"查询

use crate::graph::EntityGraph;
use pensoul_domain::entity::{Entity, EntityState};

impl EntityGraph {
    /// 查询实体在指定章节时的状态快照
    pub fn entity_state_at(
        &self,
        entity_id: &str,
        chapter_id: i64,
    ) -> Option<&EntityState> {
        let entity = self.get_entity(entity_id)?;
        match entity {
            Entity::Character(c) => c.state_at(chapter_id),
            _ => None, // 其他实体类型暂不支持状态查询
        }
    }

    /// 获取指定章节中出现的所有实体
    pub fn entities_in_chapter(&self, chapter_id: i64) -> Vec<&Entity> {
        self.all_entities()
            .filter(|e| match e {
                Entity::Character(c) => c.states.iter().any(|s| s.chapter_id == chapter_id),
                Entity::Event(ev) => ev.chapter_id == chapter_id,
                _ => false,
            })
            .collect()
    }

    /// 获取指定章节范围内的事件（按时间排序）
    pub fn events_in_range(
        &self,
        chapter_start: i64,
        chapter_end: i64,
    ) -> Vec<&Entity> {
        let mut events: Vec<&Entity> = self
            .all_entities()
            .filter(|e| match e {
                Entity::Event(ev) => {
                    ev.chapter_id >= chapter_start && ev.chapter_id <= chapter_end
                }
                _ => false,
            })
            .collect();

        events.sort_by(|a, b| {
            let a_ch = match a {
                Entity::Event(ev) => ev.chapter_id,
                _ => 0,
            };
            let b_ch = match b {
                Entity::Event(ev) => ev.chapter_id,
                _ => 0,
            };
            a_ch.cmp(&b_ch)
        });

        events
    }
}
