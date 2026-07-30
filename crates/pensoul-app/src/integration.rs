//! 集成层 — 把各引擎子系统接到本体的真实数据流上。
//!
//! 此前的架构缺陷：记忆、影响图、一致性检查器、并发控制器全部
//! 以空状态存在于 AppState 中，没有任何代码路径向它们注入数据，
//! 导致相关 IPC 永远返回空结果、章节保存永远版本冲突。
//!
//! 本模块提供两条数据通路：
//! - `rebuild_derived_state`：项目加载/切换时，从本体全量重建派生状态
//!   （记忆管道、影响图、一致性实体状态、并发版本）。
//! - `on_chapter_saved`：章节保存成功后，增量更新派生状态。
//!
//! 派生状态全部可以由本体重算，因此不单独持久化，避免双写不一致。
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pensoul_cda::{EdgeRelation, ImpactEdge, ImpactGraph, ImpactNode, NodeType};
use pensoul_consistency::{EntityState, EntityType};
use pensoul_core::{ChapterId, NovelOntology};

use crate::state::AppState;

/// 项目加载/切换后，从本体全量重建所有派生状态。
pub fn rebuild_derived_state(state: &AppState) {
    let ontology = state.ontology.read().clone();

    // ── 记忆管道：按章节顺序回放，重建热/温/冷/叙事记忆 ──
    {
        let mut memory = state.memory.write();
        for chapter in chapters_in_order(&ontology) {
            if let Some(num) = chapter.chapter_id.as_i64() {
                // 记忆管道的提取是原型级启发式，失败不阻断主流程
                let _ = memory.update(num, &chapter.content);
            }
        }
    }

    // ── 影响图：从角色/伏笔/章节引用关系重建 ──
    {
        let mut graph = state.impact_graph.write();
        *graph = build_impact_graph(&ontology);
    }

    // ── 一致性实体状态 ──
    {
        let mut checker = state.consistency_checker.write();
        for entity_state in collect_entity_states(&ontology) {
            checker.register_state(entity_state);
        }
    }

    // ── 并发版本：与本体中持久化的 chapter.version 同步 ──
    {
        let concurrency = state.concurrency.read();
        for chapter in &ontology.chapters {
            concurrency.restore_chapter(
                chapter.chapter_id.as_str(),
                &chapter.content,
                chapter.version,
            );
        }
    }
}

/// 章节保存成功后的增量更新。
///
/// 注意调用方必须已完成本体写入，本函数读取的是最新本体。
pub fn on_chapter_saved(state: &AppState, chapter_id: &ChapterId) {
    let Some(num) = chapter_id.as_i64() else {
        // 非数字章节 ID 无法进入按章节索引的派生状态，跳过
        return;
    };

    let (content, version) = {
        let ontology = state.ontology.read();
        match ontology.get_chapter(chapter_id) {
            Some(ch) => (ch.content.clone(), ch.version),
            None => return,
        }
    };

    // 记忆管道：更新该章的热/温/冷/叙事记忆
    {
        let mut memory = state.memory.write();
        let _ = memory.update(num, &content);
    }

    // 并发版本：同步最新版本号
    {
        let concurrency = state.concurrency.read();
        concurrency.restore_chapter(chapter_id.as_str(), &content, version);
    }

    // 一致性：upsert 该章的实体状态（重复保存不产生重复状态）
    {
        let ontology = state.ontology.read();
        let mut checker = state.consistency_checker.write();
        for entity_state in entity_states_for_chapter(&ontology, chapter_id, num, version) {
            checker.upsert_state(entity_state);
        }
    }

    // 影响图：章节内容会影响引用边，全量重建（规模有限，重建比分片更新更可靠）
    {
        let ontology = state.ontology.read();
        let mut graph = state.impact_graph.write();
        *graph = build_impact_graph(&ontology);
    }
}

// ── 影响图构建 ──────────────────────────────────────────────────────────

/// 从本体构建影响图。
///
/// 节点：章节（Event）、角色（Entity）、伏笔（Foreshadow）。
/// 边：章节 → 实体 的 References 边，表示该章节引用了该实体；
/// 当实体定义变更时，BFS 沿反向边找到所有引用它的章节。
pub fn build_impact_graph(ontology: &NovelOntology) -> ImpactGraph {
    let mut graph = ImpactGraph::new();

    // 章节节点
    for chapter in &ontology.chapters {
        let Some(num) = chapter.chapter_id.as_i64() else {
            continue;
        };
        if num < 0 || num > u32::MAX as i64 {
            continue;
        }
        let node = ImpactNode::new(
            format!("chapter:{}", chapter.chapter_id),
            NodeType::Event,
            num as u32,
            short_hash(&chapter.content),
        )
        .with_metadata("kind", "chapter")
        .with_metadata("title", chapter.title.clone());
        graph.add_node(node);
    }

    // 角色节点：所在章节 = 首次出现的章节
    for character in &ontology.characters.characters {
        if character.name.is_empty() {
            continue;
        }
        let first_chapter = first_appearance_chapter(ontology, &character.name).unwrap_or(1);
        let node = ImpactNode::new(
            format!("character:{}", character.id),
            NodeType::Entity,
            first_chapter,
            short_hash(&character.name),
        )
        .with_metadata("kind", "character")
        .with_metadata("name", character.name.clone());
        graph.add_node(node);
    }

    // 伏笔节点：所在章节 = 埋设章节
    for foreshadow in &ontology.narrative.foreshadows {
        let planted = foreshadow
            .planted_chapter
            .as_i64()
            .filter(|n| *n >= 0 && *n <= u32::MAX as i64)
            .map(|n| n as u32)
            .unwrap_or(1);
        let node = ImpactNode::new(
            format!("foreshadow:{}", foreshadow.id),
            NodeType::Foreshadow,
            planted,
            short_hash(&foreshadow.description),
        )
        .with_metadata("kind", "foreshadow")
        .with_metadata("name", foreshadow.name.clone());
        graph.add_node(node);
    }

    // 边：章节引用了哪些实体（按内容中出现名称判定）
    for chapter in &ontology.chapters {
        let chapter_node = format!("chapter:{}", chapter.chapter_id);
        if !graph.has_node(&chapter_node) {
            continue;
        }

        for character in &ontology.characters.characters {
            if character.name.is_empty() {
                continue;
            }
            let entity_node = format!("character:{}", character.id);
            if graph.has_node(&entity_node) && chapter.content.contains(&character.name) {
                let _ = graph.add_edge(ImpactEdge::new(
                    &chapter_node,
                    &entity_node,
                    EdgeRelation::References,
                    1.0,
                ));
            }
        }

        for foreshadow in &ontology.narrative.foreshadows {
            let entity_node = format!("foreshadow:{}", foreshadow.id);
            if !graph.has_node(&entity_node) {
                continue;
            }
            let mentioned =
                !foreshadow.name.is_empty() && chapter.content.contains(&foreshadow.name);
            let is_planted = foreshadow.planted_chapter == chapter.chapter_id;
            if mentioned || is_planted {
                let _ = graph.add_edge(ImpactEdge::new(
                    &chapter_node,
                    &entity_node,
                    EdgeRelation::References,
                    1.0,
                ));
            }
        }
    }

    graph
}

// ── 一致性实体状态采集 ──────────────────────────────────────────────────

/// 从本体全量采集一致性实体状态（项目加载时用）。
fn collect_entity_states(ontology: &NovelOntology) -> Vec<EntityState> {
    let mut states = Vec::new();

    for chapter in &ontology.chapters {
        let Some(num) = chapter.chapter_id.as_i64() else {
            continue;
        };
        states.extend(entity_states_for_chapter(
            ontology,
            &chapter.chapter_id,
            num,
            chapter.version,
        ));
    }

    states
}

/// 采集单个章节的一致性实体状态。
fn entity_states_for_chapter(
    ontology: &NovelOntology,
    chapter_id: &ChapterId,
    chapter_num: i64,
    version: i32,
) -> Vec<EntityState> {
    let mut states = Vec::new();

    let content = ontology
        .get_chapter(chapter_id)
        .map(|ch| ch.content.as_str())
        .unwrap_or("");

    // 角色状态：该章出现的角色，记录名称与当前位置快照
    for character in &ontology.characters.characters {
        if character.name.is_empty() || !content.contains(&character.name) {
            continue;
        }
        states.push(EntityState {
            entity_id: character.id.to_string(),
            entity_type: EntityType::Character,
            chapter_id: chapter_id.clone(),
            state_data: serde_json::json!({
                "name": character.name,
                "location": character.current_location,
            }),
            version,
        });
    }

    // 伏笔状态：埋设于该章或在该章被提及的伏笔
    for foreshadow in &ontology.narrative.foreshadows {
        let is_planted = foreshadow.planted_chapter == *chapter_id;
        let mentioned = !foreshadow.name.is_empty() && content.contains(&foreshadow.name);
        if !is_planted && !mentioned {
            continue;
        }
        states.push(EntityState {
            entity_id: foreshadow.id.to_string(),
            entity_type: EntityType::Foreshadow,
            chapter_id: chapter_id.clone(),
            state_data: serde_json::json!({
                "name": foreshadow.name,
                "status": format!("{:?}", foreshadow.status),
                "expected_resolve_chapter": foreshadow
                    .expected_resolve_chapter
                    .as_ref()
                    .and_then(|c| c.as_i64()),
                "actual_resolve_chapter": foreshadow
                    .actual_resolve_chapter
                    .as_ref()
                    .and_then(|c| c.as_i64()),
                "related_characters": foreshadow
                    .related_characters
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>(),
            }),
            version,
        });
    }

    let _ = chapter_num;
    states
}

// ── 内部辅助 ────────────────────────────────────────────────────────────

/// 按数字章节 ID 升序返回章节引用；非数字 ID 的章节排在最后。
fn chapters_in_order(ontology: &NovelOntology) -> Vec<&pensoul_core::Chapter> {
    let mut chapters: Vec<&pensoul_core::Chapter> = ontology.chapters.iter().collect();
    chapters.sort_by_key(|ch| ch.chapter_id.as_i64().unwrap_or(i64::MAX));
    chapters
}

/// 查找角色首次出现的章节号（u32），未出现返回 None。
fn first_appearance_chapter(ontology: &NovelOntology, name: &str) -> Option<u32> {
    chapters_in_order(ontology)
        .iter()
        .filter(|ch| ch.content.contains(name))
        .filter_map(|ch| ch.chapter_id.as_i64())
        .filter(|n| *n >= 0 && *n <= u32::MAX as i64)
        .map(|n| n as u32)
        .next()
}

/// 内容短哈希（仅用于变更检测，非加密用途）。
fn short_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_hash_stable() {
        assert_eq!(short_hash("abc"), short_hash("abc"));
        assert_ne!(short_hash("abc"), short_hash("abd"));
    }
}
