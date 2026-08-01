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
//!
//! 顺序语义一律使用 `Chapter.chapter_no`（加载时已回填），
//! 不再依赖 `chapter_id` 可解析为数字——前端生成的 `ch-<ts>-<rand>`
//! 字符串 ID 也能正常进入记忆/影响图/一致性。
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
            if chapter.chapter_no > 0 {
                // 记忆管道的提取是原型级启发式，失败不阻断主流程
                let _ = memory.update(chapter.chapter_no, &chapter.content);
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
    let (num, content, version) = {
        let ontology = state.ontology.read();
        match ontology.get_chapter(chapter_id) {
            // 未分配序号的章节不进入按章节索引的派生状态
            Some(ch) if ch.chapter_no > 0 => (ch.chapter_no, ch.content.clone(), ch.version),
            _ => return,
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
        for entity_state in entity_states_for_chapter(&ontology, chapter_id, version) {
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
        let num = chapter.chapter_no;
        if num <= 0 || num > u32::MAX as i64 {
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
        let planted = chapter_no_of(ontology, &foreshadow.planted_chapter)
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
        if chapter.chapter_no <= 0 {
            continue;
        }
        states.extend(entity_states_for_chapter(
            ontology,
            &chapter.chapter_id,
            chapter.version,
        ));
    }

    states
}

/// 采集单个章节的一致性实体状态。
fn entity_states_for_chapter(
    ontology: &NovelOntology,
    chapter_id: &ChapterId,
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
                    .and_then(|c| chapter_no_of(ontology, c)),
                "actual_resolve_chapter": foreshadow
                    .actual_resolve_chapter
                    .as_ref()
                    .and_then(|c| chapter_no_of(ontology, c)),
                "related_characters": foreshadow
                    .related_characters
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>(),
            }),
            version,
        });
    }

    states
}

// ── 内部辅助 ────────────────────────────────────────────────────────────

/// 按章节序号升序返回章节引用；未分配序号（chapter_no <= 0）的章节排在最后。
fn chapters_in_order(ontology: &NovelOntology) -> Vec<&pensoul_core::Chapter> {
    let mut chapters: Vec<&pensoul_core::Chapter> = ontology.chapters.iter().collect();
    chapters.sort_by_key(|ch| {
        if ch.chapter_no > 0 {
            ch.chapter_no
        } else {
            i64::MAX
        }
    });
    chapters
}

/// 由章节 ID 查章节序号；章节不存在或未分配序号时返回 None。
fn chapter_no_of(ontology: &NovelOntology, chapter_id: &ChapterId) -> Option<i64> {
    ontology
        .get_chapter(chapter_id)
        .map(|ch| ch.chapter_no)
        .filter(|n| *n > 0)
}

/// 查找角色首次出现的章节号（u32），未出现返回 None。
fn first_appearance_chapter(ontology: &NovelOntology, name: &str) -> Option<u32> {
    chapters_in_order(ontology)
        .iter()
        .filter(|ch| ch.content.contains(name))
        .filter_map(|ch| {
            if ch.chapter_no > 0 && ch.chapter_no <= u32::MAX as i64 {
                Some(ch.chapter_no as u32)
            } else {
                None
            }
        })
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
    use pensoul_core::{Chapter, ChapterStatus, ProjectId, VolumeId};

    fn make_chapter(id: &str, chapter_no: i64, content: &str) -> Chapter {
        Chapter {
            chapter_id: ChapterId::new(id),
            chapter_no,
            volume_id: VolumeId::new("vol-1"),
            title: format!("第{id}章"),
            summary: String::new(),
            content: content.to_string(),
            word_count: content.chars().count() as u32,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 1.0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_short_hash_stable() {
        assert_eq!(short_hash("abc"), short_hash("abc"));
        assert_ne!(short_hash("abc"), short_hash("abd"));
    }

    #[test]
    fn test_chapters_in_order_sorts_by_chapter_no_zero_last() {
        let mut onto = NovelOntology::new(ProjectId::new("p"), String::new());
        // 故意乱序放入：序号 3、未分配、序号 1
        onto.chapters.push(make_chapter("ch-c", 3, ""));
        onto.chapters.push(make_chapter("ch-x", 0, ""));
        onto.chapters.push(make_chapter("ch-a", 1, ""));

        let ordered = chapters_in_order(&onto);
        let ids: Vec<&str> = ordered.iter().map(|c| c.chapter_id.as_str()).collect();
        assert_eq!(ids, vec!["ch-a", "ch-c", "ch-x"]);
    }

    #[test]
    fn test_chapter_no_of_resolves_string_id() {
        let mut onto = NovelOntology::new(ProjectId::new("p"), String::new());
        onto.chapters.push(make_chapter("ch-123-abc", 2, ""));

        assert_eq!(chapter_no_of(&onto, &ChapterId::new("ch-123-abc")), Some(2));
        assert_eq!(chapter_no_of(&onto, &ChapterId::new("missing")), None);
    }

    #[test]
    fn test_first_appearance_uses_chapter_no_not_id() {
        let mut onto = NovelOntology::new(ProjectId::new("p"), String::new());
        // 字符串 ID 的章节也能被正确索引
        onto.chapters
            .push(make_chapter("ch-ts-1", 1, "路人甲路过。"));
        onto.chapters.push(make_chapter("ch-ts-2", 2, "林晚登场。"));

        assert_eq!(first_appearance_chapter(&onto, "林晚"), Some(2));
        assert_eq!(first_appearance_chapter(&onto, "不存在"), None);
    }
}
