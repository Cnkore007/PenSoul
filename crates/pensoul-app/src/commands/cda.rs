/// CDA 影响图命令
use crate::state::AppState;
use pensoul_core::ChapterId;

/// 查找受影响的章节
#[tauri::command]
pub async fn find_affected_chapters(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    changed_entities: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let graph = state.impact_graph.read();

    // 解析 chapter_id 为 u32
    let chapter_num: u32 = chapter_id
        .parse()
        .map_err(|_| format!("无效的章节 ID: {}", chapter_id))?;

    let affected = graph.find_affected(chapter_num, &changed_entities, 5);

    let results: Vec<serde_json::Value> = affected
        .into_iter()
        .filter_map(|item| serde_json::to_value(&item).ok())
        .collect();

    Ok(results)
}

/// 获取影响图
#[tauri::command]
pub async fn get_impact_graph(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let graph = state.impact_graph.read();
    let stats = graph.stats();

    serde_json::to_value(&stats).map_err(|e| e.to_string())
}

/// 章节修改后的影响分析（供笔耕保存后展示）：
/// 解析章号 → 以本章涉及的实体为变更种子跑 CDA → 过滤本章相关的一致性违规。
#[tauri::command]
pub async fn analyze_chapter_impact(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<serde_json::Value, String> {
    let id = ChapterId::new(chapter_id.clone());
    let chapter_no: u32 = {
        let onto = state.ontology.read();
        onto.get_chapter(&id)
            .map(|c| c.chapter_no as u32)
            .ok_or_else(|| format!("章节不存在: {chapter_id}"))?
    };

    // 本章涉及的实体（角色/伏笔/世界观/术语）作为变更种子
    let entity_ids: Vec<String> = {
        let onto = state.ontology.read();
        let version = onto.get_chapter(&id).map(|c| c.version).unwrap_or(0);
        crate::integration::entity_states_for_chapter(&onto, &id, version)
            .into_iter()
            .map(|s| s.entity_id)
            .collect()
    };

    // CDA：从变更实体反向传播，标记受影响章节（Direct / Indirect / Cascading）
    let affected: Vec<serde_json::Value> = {
        let graph = state.impact_graph.read();
        graph
            .find_affected(chapter_no, &entity_ids, 5)
            .into_iter()
            .filter_map(|item| serde_json::to_value(&item).ok())
            .collect()
    };

    // 一致性：全书检查后过滤与本章相关的违规
    let violations: Vec<serde_json::Value> = {
        let checker = state.consistency_checker.read();
        let report = checker.check_all();
        report
            .violations
            .iter()
            .filter(|v| v.chapter_a.as_str() == id.as_str() || v.chapter_b.as_str() == id.as_str())
            .filter_map(|v| serde_json::to_value(v).ok())
            .collect()
    };

    Ok(serde_json::json!({
        "chapter_no": chapter_no,
        "affected": affected,
        "consistency": violations,
    }))
}
