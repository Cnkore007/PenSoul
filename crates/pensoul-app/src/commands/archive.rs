// archive.rs — 归档压缩 + 操作日志 / 全局回滚 / 成本档位（P6）
// ① 操作日志查询：operation-log.json（P1 事实提取 + 级联同步共用的审计轨迹）
// ② 全局回滚：逆应用最近 N 条操作的结构化事实（删实体 / 恢复旧值 / 回退伏笔状态）
// ③ 档案压缩归档：老章节（按章号）事实下沉为「卷摘要」，控制档案膨胀与检索噪音
// ④ 成本档位：按 Agent 绑定模型 + 日志操作数给出成本提示

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::extract::AppliedFact;
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::ForeshadowStatus;

// ---- 操作日志 ----

#[derive(Deserialize)]
pub struct LogParams {
    /// 返回条数（默认 50，最多 500）
    pub limit: Option<String>,
}

#[derive(Deserialize)]
pub struct RollbackParams {
    /// 回滚最近 N 条操作（默认 1）
    pub last_n: Option<String>,
}

fn log_path(base_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(base_dir).join("_config").join("operation-log.json")
}

fn load_log(base_dir: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(log_path(base_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// 操作日志查询（倒序返回）
pub async fn list_operations(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<LogParams>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let limit = params
        .limit
        .as_deref()
        .map(|s| s.parse::<usize>().unwrap_or(50).min(500))
        .unwrap_or(50);
    let entries = load_log(&base_dir);
    let total = entries.len();
    let recent: Vec<&serde_json::Value> = entries.iter().rev().take(limit).collect();
    serde_json::to_string(&serde_json::json!({
        "total": total,
        "returned": recent.len(),
        "entries": recent,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 全局回滚：逆应用最近 N 条操作（删实体 / 恢复旧值 / 回退伏笔）
pub async fn rollback(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<RollbackParams>,
) -> Result<String, ApiError> {
    let last_n = params
        .last_n
        .as_deref()
        .map(|s| s.parse::<usize>().unwrap_or(1))
        .unwrap_or(1)
        .max(1);
    if last_n > 100 {
        return Err(ApiError::bad_request("单次最多回滚 100 条操作"));
    }

    let mut state = state.write().await;
    let base_dir = state.base_dir.clone();
    let mut entries = load_log(&base_dir);
    if entries.is_empty() {
        return Err(ApiError::bad_request("没有可回滚的操作日志"));
    }
    let n = last_n.min(entries.len());
    let target: Vec<serde_json::Value> = entries.split_off(entries.len() - n);

    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let mut undone: Vec<String> = Vec::new();

    // 逆序逆应用（后做先撤）
    for entry in target.iter().rev() {
        let facts: Vec<AppliedFact> = entry
            .get("facts")
            .and_then(|f| serde_json::from_value(f.clone()).ok())
            .unwrap_or_default();
        for fact in facts.iter().rev() {
            rollback_fact(ontology, fact, &mut undone);
        }
        if let Some(t) = entry["time"].as_str() {
            undone.push(format!("[{}] 回滚操作", t));
        }
    }

    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;
    // 已回滚条目从日志移除：写失败必须显式告知，防止用户重复回滚已回滚的条目
    let mut log_warning: Option<String> = None;
    let trimmed = serde_json::to_string_pretty(&entries).map_err(|e| ApiError::internal(e.to_string()))?;
    if let Err(e) = std::fs::write(log_path(&base_dir), &trimmed) {
        log_warning = Some(format!(
            "操作日志截断写入失败（{e}），本次已回滚的条目仍保留在日志中，请勿重复回滚"
        ));
    }

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "rolled_back": n,
        "remaining": entries.len(),
        "undone": undone,
        "log_warning": log_warning,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 逆应用单条事实
fn rollback_fact(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &AppliedFact,
    undone: &mut Vec<String>,
) {
    match fact.kind.as_str() {
        "new_character" => {
            ontology
                .characters
                .characters
                .retain(|c| c.name != fact.name);
            undone.push(format!("删除角色: {}", fact.name));
        }
        "new_organization" => {
            ontology
                .world
                .organizations
                .retain(|o| o.name != fact.name);
            undone.push(format!("删除组织: {}", fact.name));
        }
        "new_setting" => {
            ontology.world.locations.retain(|s| s.name != fact.name);
            undone.push(format!("删除设定: {}", fact.name));
        }
        "new_event" => {
            ontology.world.timeline.retain(|e| e.name != fact.name);
            undone.push(format!("删除事件: {}", fact.name));
        }
        "character_update" => {
            let Some(character) = ontology
                .characters
                .characters
                .iter_mut()
                .find(|c| c.name == fact.name)
            else {
                undone.push(format!("回滚失败（角色不存在）: {}", fact.name));
                return;
            };
            let attr = fact.attribute.as_deref().unwrap_or("");
            let p = &mut character.properties;
            match attr {
                "realm" => p.realm = fact.old_value.clone(),
                "appearance" => p.appearance = fact.old_value.clone(),
                "attire" => p.attire = fact.old_value.clone(),
                "occupation" => p.occupation = fact.old_value.clone(),
                "wants" => p.wants = fact.old_value.clone(),
                "fears" => p.fears = fact.old_value.clone(),
                "secret" => p.secret = fact.old_value.clone(),
                "backstory" => p.backstory = fact.old_value.clone(),
                "techniques" => {
                    // 追加型：移除本次新增项
                    if let Some(new) = &fact.new_value {
                        for t in new.split(['，', ',']) {
                            let t = t.trim();
                            p.techniques.retain(|x| x != t);
                        }
                    }
                }
                "items" => {
                    if let Some(new) = &fact.new_value {
                        for it in new.split(['，', ',']) {
                            let it = it.trim();
                            p.items.retain(|x| x != it);
                        }
                    }
                }
                _ => {
                    undone.push(format!("回滚跳过未知属性 {attr}: {}", fact.name));
                    return;
                }
            }
            undone.push(format!("角色 {}({attr}) 恢复: {:?}", fact.name, fact.old_value));
        }
        "foreshadow_progress" => {
            let Some(foreshadow) = ontology
                .narrative
                .foreshadows
                .iter_mut()
                .find(|f| f.name == fact.name)
            else {
                undone.push(format!("回滚失败（伏笔不存在）: {}", fact.name));
                return;
            };
            if let Some(old) = fact.old_value.as_deref() {
                let restored = match old {
                    "Planned" => ForeshadowStatus::Planned,
                    "Planted" => ForeshadowStatus::Planted,
                    "Progressing" => ForeshadowStatus::Progressing,
                    "Resolved" => ForeshadowStatus::Resolved,
                    "Abandoned" => ForeshadowStatus::Abandoned,
                    _ => ForeshadowStatus::Overdue,
                };
                foreshadow.status = restored;
                undone.push(format!("伏笔 {} 状态恢复: {}", fact.name, old));
            }
        }
        other => undone.push(format!("回滚跳过未知类型 {other}: {}", fact.name)),
    }
}

// ---- 档案压缩归档 ----

#[derive(Deserialize)]
pub struct CompressParams {
    /// 保留最近 N 章明细（早于 N 的章节下沉为卷摘要；默认 20）
    pub keep_recent: Option<String>,
}

fn archive_path(base_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(base_dir).join("_config").join("archive").join("volumes.json")
}

/// 归档压缩：老章节（按章号）事实下沉为卷摘要，控制档案膨胀与检索噪音
pub async fn compress(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CompressParams>,
) -> Result<String, ApiError> {
    let keep_recent = match params.keep_recent.as_deref() {
        None | Some("") => 20,
        Some(s) => s
            .parse::<i64>()
            .map_err(|_| ApiError::bad_request("keep_recent 必须是正整数"))?
            .max(1),
    };

    let (base_dir, archived) = {
        let state = state.read().await;
        let base_dir = state.base_dir.clone();
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let max_no = ontology
            .chapters
            .iter()
            .map(|c| c.chapter_no)
            .max()
            .unwrap_or(0);
        let cutoff = max_no - keep_recent;
        if cutoff < 1 {
            return Err(ApiError::bad_request(format!(
                "章节数不足（当前 {max_no} 章），无需归档。"
            )));
        }
        // 老章节（≤ cutoff）：汇总为卷摘要
        let archived: Vec<serde_json::Value> = ontology
            .chapters_in_order()
            .into_iter()
            .filter(|c| c.chapter_no <= cutoff)
            .map(|c| {
                let chars = c.content.chars().count();
                serde_json::json!({
                    "chapter_no": c.chapter_no,
                    "title": c.title,
                    "summary": c.summary,
                    "word_count": chars,
                    "archived_at": chrono::Utc::now().to_rfc3339(),
                })
            })
            .collect();
        (base_dir, archived)
    };

    if archived.is_empty() {
        return Err(ApiError::bad_request("没有可归档的老章节"));
    }
    let dir = std::path::Path::new(&base_dir)
        .join("_config")
        .join("archive");
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::internal(e.to_string()))?;
    let volumes: serde_json::Value = serde_json::json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "keep_recent": keep_recent,
        "volumes": archived,
    });
    std::fs::write(
        archive_path(&base_dir),
        serde_json::to_string_pretty(&volumes).map_err(|e| ApiError::internal(e.to_string()))?,
    )
    .map_err(|e| ApiError::internal(format!("归档写入失败: {e}")))?;

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "archived": archived.len(),
        "keep_recent": keep_recent,
        "note": format!(
            "已归档 {} 个老章节为卷摘要（保留最近 {} 章明细）。图谱与约束不受影响，正文仍可编辑。",
            archived.len(),
            keep_recent
        ),
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 查看归档卷摘要
pub async fn list_archive(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let base_dir = state.read().await.base_dir.clone();
    let volumes = std::fs::read_to_string(archive_path(&base_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or(serde_json::json!({ "volumes": [] }));
    serde_json::to_string(&volumes).map_err(|e| ApiError::internal(e.to_string()))
}

// ---- 成本档位 ----

/// 成本提示：按 Agent 绑定模型 + 日志操作数给出档位
pub async fn cost_report(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let base_dir = state.base_dir.clone();
    let project_id = state
        .ontology
        .as_ref()
        .map(|o| o.project_id.as_str().to_string())
        .unwrap_or_default();
    let entries = load_log(&base_dir);
    let cascade_entries: Vec<serde_json::Value> = std::fs::read_to_string(
        std::path::Path::new(&base_dir).join("_config").join("cascade-log.json"),
    )
    .ok()
    .and_then(|t| serde_json::from_str(&t).ok())
    .unwrap_or_default();
    let recipe = crate::commands::distill::load_style_recipe(&base_dir, &project_id);
    let agents = crate::commands::agent::AgentConfigStore::new(&base_dir)
        .load()
        .agents
        .clone();

    // 各角色绑定模型（未绑定 = 全局默认）
    let agent_bindings: Vec<serde_json::Value> = agents
        .iter()
        .map(|a| {
            serde_json::json!({
                "role": a.role_id,
                "display_name": a.display_name,
                "llm_config_id": a.llm_config_id,
            })
        })
        .collect();

    // 成本档位估算（按操作量分级）
    let op_count = entries.len() + cascade_entries.len();
    let tier = if op_count == 0 {
        "空闲"
    } else if op_count <= 50 {
        "低（日常创作）"
    } else if op_count <= 200 {
        "中（密集创作）"
    } else {
        "高（建议关注 token 消耗）"
    };

    serde_json::to_string(&serde_json::json!({
        "operation_count": op_count,
        "fact_extract_count": entries.len(),
        "cascade_count": cascade_entries.len(),
        "distilled_books": recipe.as_ref().map(|r| r.books.len()).unwrap_or(0),
        "tier": tier,
        "agent_bindings": agent_bindings,
        "note": "成本主要由 LLM 调用产生：生成/审校/提取/蒸馏均为按次调用。写作用高质量模型、审校/提取用低成本模型可显著降本。",
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

// ---- 单元测试 ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::extract::AppliedFact;

    #[test]
    fn rollback_new_entity_removes_it() {
        let mut ontology = pensoul_domain::ontology::NovelOntology::new(
            pensoul_domain::id::ProjectId::default(),
            "测试",
        );
        ontology.characters.characters.push(pensoul_domain::entity::Character::new("林默"));
        let fact = AppliedFact {
            kind: "new_character".into(),
            name: "林默".into(),
            attribute: None,
            old_value: None,
            new_value: None,
        };
        let mut undone = Vec::new();
        rollback_fact(&mut ontology, &fact, &mut undone);
        assert!(ontology.characters.characters.is_empty(), "新建角色应被删除");
        assert!(!undone.is_empty());
    }

    #[test]
    fn rollback_character_update_restores_old() {
        let mut ontology = pensoul_domain::ontology::NovelOntology::new(
            pensoul_domain::id::ProjectId::default(),
            "测试",
        );
        let mut c = pensoul_domain::entity::Character::new("林默");
        c.properties.realm = Some("金丹".into());
        ontology.characters.characters.push(c);
        let fact = AppliedFact {
            kind: "character_update".into(),
            name: "林默".into(),
            attribute: Some("realm".into()),
            old_value: Some("筑基".into()),
            new_value: Some("金丹".into()),
        };
        let mut undone = Vec::new();
        rollback_fact(&mut ontology, &fact, &mut undone);
        assert_eq!(
            ontology.characters.characters[0].properties.realm.as_deref(),
            Some("筑基"),
            "境界应恢复为旧值"
        );
    }

    #[test]
    fn rollback_foreshadow_restores_status() {
        let mut ontology = pensoul_domain::ontology::NovelOntology::new(
            pensoul_domain::id::ProjectId::default(),
            "测试",
        );
        let mut f = pensoul_domain::entity::Foreshadow::new("身世之谜", 5);
        f.status = ForeshadowStatus::Progressing;
        ontology.narrative.foreshadows.push(f);
        let fact = AppliedFact {
            kind: "foreshadow_progress".into(),
            name: "身世之谜".into(),
            attribute: Some("status".into()),
            old_value: Some("Planted".into()),
            new_value: Some("Progressing".into()),
        };
        let mut undone = Vec::new();
        rollback_fact(&mut ontology, &fact, &mut undone);
        assert_eq!(
            ontology.narrative.foreshadows[0].status,
            ForeshadowStatus::Planted,
            "伏笔状态应回退"
        );
    }
}
