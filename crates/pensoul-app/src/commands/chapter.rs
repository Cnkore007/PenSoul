//! 章节管理命令
use crate::state::AppState;
use pensoul_concurrency::{Operation, OperationType};
use pensoul_core::ChapterId;

/// 获取章节
#[tauri::command]
pub async fn get_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    let id = ChapterId::new(chapter_id);

    match ontology.get_chapter(&id) {
        Some(chapter) => serde_json::to_value(chapter).map_err(|e| e.to_string()),
        None => Err(format!("章节 {} 不存在", id)),
    }
}

/// 保存章节（乐观锁）
///
/// 修复历史缺陷：此前章节的并发版本从未注册过，
/// `submit_operation` 必然走 Rejected 分支导致保存永远失败。
/// 现在会在首次保存时从本体同步版本，并在成功后更新派生状态
/// （记忆管道 / 影响图 / 一致性状态 / 并发版本）。
#[tauri::command]
pub async fn save_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    content: String,
    expected_version: i32,
) -> Result<i32, String> {
    let id = ChapterId::new(chapter_id);

    // 首次保存该章时，从本体恢复版本号，避免必然冲突
    {
        let concurrency = state.concurrency.read();
        if concurrency.get_version(id.as_str()) == -1 {
            let ontology = state.ontology.read();
            let chapter = ontology
                .get_chapter(&id)
                .ok_or_else(|| format!("章节 {} 不存在", id))?;
            concurrency.restore_chapter(id.as_str(), &chapter.content, chapter.version);
        }
    }

    let op = Operation {
        op_id: uuid::Uuid::new_v4().to_string(),
        op_type: OperationType::UserEdit,
        chapter_id: id.to_string(),
        content: content.clone(),
        expected_version,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        status: pensoul_concurrency::OperationStatus::Pending,
        actual_version: None,
    };

    let result = {
        let concurrency = state.concurrency.read();
        concurrency.submit_operation(op)
    };

    match result.status {
        pensoul_concurrency::OperationStatus::Applied => {
            let new_version = result.actual_version.unwrap_or(expected_version + 1);

            // 更新本体中的章节内容与版本
            {
                let mut ontology = state.ontology.write();
                if let Some(chapter) = ontology.chapters.iter_mut().find(|ch| ch.chapter_id == id) {
                    chapter.content = content;
                    chapter.version = new_version;
                    chapter.word_count = chapter.content.chars().count() as u32;
                }
            }

            // 增量更新派生状态（记忆管道/影响图/一致性状态/并发版本）
            crate::integration::on_chapter_saved(&state, &id);

            Ok(new_version)
        }
        pensoul_concurrency::OperationStatus::Conflict => Err(format!(
            "版本冲突: 期望版本 {}，实际版本 {}",
            expected_version,
            result.actual_version.unwrap_or(-1)
        )),
        _ => Err("操作被拒绝".to_string()),
    }
}

/// 列出所有章节
#[tauri::command]
pub async fn list_chapters(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let ontology = state.ontology.read();

    let chapters: Vec<serde_json::Value> = ontology
        .chapters
        .iter()
        .filter_map(|ch| serde_json::to_value(ch).ok())
        .collect();

    Ok(chapters)
}
