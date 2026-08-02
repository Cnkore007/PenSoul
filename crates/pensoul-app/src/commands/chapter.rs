//! 章节管理命令
use crate::state::AppState;
use pensoul_concurrency::{Operation, OperationType};
use pensoul_core::{ChapterAnnotation, ChapterId, ChapterStatus, Volume, VolumeId};

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
    annotations: Option<Vec<ChapterAnnotation>>,
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
                    if let Some(anno) = annotations {
                        // 回填批注定位串，保证批注中心/标注集可统一寻址
                        let mut anno = anno;
                        for a in anno.iter_mut() {
                            if a.target.is_none() {
                                a.target = Some(format!("chapter:{}:body", id));
                            }
                        }
                        chapter.annotations = anno;
                    }
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

/// 新建或更新章节（插入式保存）
///
/// `save_chapter` 只更新已存在章节的内容；前端在大纲中新建的章节
/// （含讨论成果导入的章节）必须走这里才能落盘。
/// 梗概属于大纲层信息，与正文 content 分开存储。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn upsert_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    volume_id: String,
    title: String,
    content: String,
    summary: String,
    status: String,
) -> Result<(), String> {
    if chapter_id.trim().is_empty() {
        return Err("章节 ID 不能为空".to_string());
    }
    let id = ChapterId::new(chapter_id);
    let vid = VolumeId::new(volume_id);
    let status: ChapterStatus = serde_json::from_value(serde_json::Value::String(status))
        .map_err(|e| format!("非法章节状态: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    let edit_sample = {
        let mut ontology = state.ontology.write();
        // 卷不存在时自动补建（标题由 save_volumes 后续覆盖）
        if !ontology.volumes.iter().any(|v| v.volume_id == vid) {
            ontology.volumes.push(Volume {
                volume_id: vid.clone(),
                title: vid.as_str().to_string(),
                chapter_ids: Vec::new(),
                summary: String::new(),
                expanded: true,
            });
        }
        let edit_result = match ontology.chapters.iter_mut().find(|ch| ch.chapter_id == id) {
            Some(ch) => {
                let old = ch.clone();
                ch.title = title;
                ch.volume_id = vid;
                ch.summary = summary;
                ch.status = status;
                if ch.content != content {
                    ch.content = content;
                    ch.version += 1;
                }
                ch.word_count = ch.content.chars().count() as u32;
                ch.updated_at = now;
                Some(crate::edits::chapter_diff_samples(
                    &old,
                    &ch.title,
                    &ch.summary,
                    &ch.content,
                ))
            }
            None => {
                let word_count = content.chars().count() as u32;
                // 新章节分配序号：现有最大 chapter_no + 1（忽略未分配的 0）
                let chapter_no = ontology
                    .chapters
                    .iter()
                    .map(|c| c.chapter_no)
                    .max()
                    .unwrap_or(0)
                    + 1;
                ontology.chapters.push(pensoul_core::Chapter {
                    chapter_id: id.clone(),
                    chapter_no,
                    volume_id: vid,
                    title,
                    summary,
                    content,
                    word_count,
                    version: 1,
                    status,
                    consistency_score: 1.0,
                    created_at: now.clone(),
                    updated_at: now,
                    annotations: Vec::new(),
                    revisions: Vec::new(),
                });
                None
            }
        };
        // 同步卷的章节列表（先分组再写回，避免经由锁守卫的交叉借用）
        let mut by_volume: std::collections::HashMap<String, Vec<ChapterId>> =
            std::collections::HashMap::new();
        for ch in &ontology.chapters {
            by_volume
                .entry(ch.volume_id.as_str().to_string())
                .or_default()
                .push(ch.chapter_id.clone());
        }
        for vol in ontology.volumes.iter_mut() {
            if let Some(ids) = by_volume.get(vol.volume_id.as_str()) {
                vol.chapter_ids = ids.clone();
            }
        }
        edit_result
    };
    if let Some(samples) = edit_sample {
        crate::edits::record_edit_samples(&state, samples);
    }

    // 注册并发版本，后续 save_chapter 走乐观锁才不会误报冲突
    {
        let concurrency = state.concurrency.read();
        if concurrency.get_version(id.as_str()) == -1 {
            let ontology = state.ontology.read();
            if let Some(ch) = ontology.get_chapter(&id) {
                concurrency.restore_chapter(id.as_str(), &ch.content, ch.version);
            }
        }
    }

    state.save().map_err(|e| e.to_string())
}

/// 保存卷列表（卷名等元数据）
///
/// 卷的归属关系以章节的 volume_id 为准，这里只持久化标题与摘要；
/// chapter_ids 由后端按章节归属自动同步。
#[tauri::command]
pub async fn save_volumes(
    state: tauri::State<'_, AppState>,
    volumes: Vec<serde_json::Value>,
) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct VolumeInput {
        volume_id: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        summary: String,
        #[serde(default = "default_volume_expanded")]
        expanded: bool,
    }

    fn default_volume_expanded() -> bool {
        true
    }

    let mut parsed = Vec::with_capacity(volumes.len());
    for v in volumes {
        let input: VolumeInput = serde_json::from_value(v).map_err(|e| e.to_string())?;
        if input.volume_id.trim().is_empty() {
            return Err("卷 ID 不能为空".to_string());
        }
        parsed.push(input);
    }

    {
        let mut ontology = state.ontology.write();
        let mut volumes = Vec::with_capacity(parsed.len());
        for input in parsed {
            let vid = VolumeId::new(input.volume_id);
            let title = if input.title.trim().is_empty() {
                vid.as_str().to_string()
            } else {
                input.title
            };
            let chapter_ids = ontology
                .chapters
                .iter()
                .filter(|ch| ch.volume_id == vid)
                .map(|ch| ch.chapter_id.clone())
                .collect();
            volumes.push(Volume {
                volume_id: vid,
                title,
                chapter_ids,
                summary: input.summary,
                expanded: input.expanded,
            });
        }
        ontology.volumes = volumes;
    }

    state.save().map_err(|e| e.to_string())
}

/// 列出所有卷
#[tauri::command]
pub async fn get_volumes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let ontology = state.ontology.read();
    let volumes: Vec<serde_json::Value> = ontology
        .volumes
        .iter()
        .filter_map(|v| serde_json::to_value(v).ok())
        .collect();
    Ok(volumes)
}

/// 智能分卷：按基础设定中的卷数（target_volumes）把全部章节均分到各卷，
/// 卷标题为「第一卷 / 第二卷 …」，每卷可折叠、内含章节细纲。
/// 设定未填卷数（0）时不自动分卷，保留导入/手动的卷名。
#[tauri::command]
pub async fn smart_volume_split(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let target_volumes = {
        let ontology = state.ontology.read();
        ontology.settings.target_volumes
    };
    if target_volumes == 0 {
        return Err("基础设定未填写「预计卷数」，保持导入的卷名，不执行智能分卷".to_string());
    }

    let (numbered_chapters, total) = {
        let ontology = state.ontology.read();
        // 只按有章号的章节分配（_default 隐式卷也参与，章号是全局序）
        let mut chapters: Vec<_> = ontology
            .chapters
            .iter()
            .filter(|c| c.chapter_no > 0)
            .map(|c| (c.chapter_id.clone(), c.chapter_no))
            .collect();
        chapters.sort_by_key(|(_, no)| *no);
        let total = chapters.len();
        (chapters, total)
    };
    if numbered_chapters.is_empty() {
        return Err("还没有可分卷的章节（需要有章节序号），请先展开细纲或添加章节".to_string());
    }

    // 卷数不超过章节数；每卷至少一章，按序均分
    let n = (target_volumes as usize).min(numbered_chapters.len());
    let per = numbered_chapters.len().div_ceil(n);
    let mut volumes = Vec::with_capacity(n);
    for i in 0..n {
        volumes.push(Volume {
            volume_id: VolumeId::new(format!("smart-vol-{}", i + 1)),
            title: format!("第{}卷", chinese_num(i + 1)),
            chapter_ids: Vec::new(),
            summary: String::new(),
            expanded: true,
        });
    }

    {
        let mut ontology = state.ontology.write();
        // 分配章节到各卷
        for (idx, (cid, _)) in numbered_chapters.iter().enumerate() {
            let vol_idx = (idx / per).min(n - 1);
            let vid = volumes[vol_idx].volume_id.clone();
            if let Some(c) = ontology
                .chapters
                .iter_mut()
                .find(|c| c.chapter_id == *cid)
            {
                c.volume_id = vid.clone();
                volumes[vol_idx].chapter_ids.push(cid.clone());
            }
        }
        ontology.volumes = volumes;
        // 同步其它章节（无章号）归入默认卷
        let mut by_volume: std::collections::HashMap<String, Vec<ChapterId>> =
            std::collections::HashMap::new();
        for ch in &ontology.chapters {
            by_volume
                .entry(ch.volume_id.as_str().to_string())
                .or_default()
                .push(ch.chapter_id.clone());
        }
        for vol in ontology.volumes.iter_mut() {
            if let Some(ids) = by_volume.get(vol.volume_id.as_str()) {
                vol.chapter_ids = ids.clone();
            }
        }
    }
    state.save().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "volumes": n,
        "chapters_per_volume": per,
        "total_chapters": total,
        "message": format!(
            "已按基础设定卷数（{} 卷）智能分卷：共 {} 章，每卷约 {} 章",
            n, total, per
        ),
    }))
}

/// 中文数字（1-99，智能分卷标题用）
fn chinese_num(n: usize) -> String {
    const DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    if n == 0 {
        return "零".to_string();
    }
    if n < 10 {
        return DIGITS[n].to_string();
    }
    if n == 10 {
        return "十".to_string();
    }
    if n < 20 {
        return format!("十{}", DIGITS[n % 10]);
    }
    let tens = DIGITS[n / 10];
    let ones = n % 10;
    if ones == 0 {
        format!("{tens}十")
    } else {
        format!("{tens}十{}", DIGITS[ones])
    }
}

/// 删除章节（同时从所属卷的章节列表移除）
#[tauri::command]
pub async fn delete_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<(), String> {
    let id = ChapterId::new(chapter_id);
    {
        let mut ontology = state.ontology.write();
        let before = ontology.chapters.len();
        ontology.chapters.retain(|ch| ch.chapter_id != id);
        if ontology.chapters.len() == before {
            return Err(format!("章节 {} 不存在", id));
        }
        for vol in ontology.volumes.iter_mut() {
            vol.chapter_ids.retain(|cid| cid != &id);
        }
    }
    state.save().map_err(|e| e.to_string())
}

/// 删除卷（卷下的章节一并删除）
#[tauri::command]
pub async fn delete_volume(
    state: tauri::State<'_, AppState>,
    volume_id: String,
) -> Result<(), String> {
    let vid = VolumeId::new(volume_id);
    {
        let mut ontology = state.ontology.write();
        let before = ontology.volumes.len();
        ontology.volumes.retain(|v| v.volume_id != vid);
        if ontology.volumes.len() == before {
            return Err(format!("卷 {} 不存在", vid));
        }
        ontology.chapters.retain(|ch| ch.volume_id != vid);
    }
    state.save().map_err(|e| e.to_string())
}
