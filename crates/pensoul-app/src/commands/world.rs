// world.rs — 世界管理 API（人物/地点/事件/伏笔/规则/核心概念）

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::*;
use pensoul_domain::entity::EntityRef;

#[derive(Deserialize)]
pub struct AddForeshadowParams {
    pub name: String,
    pub planted_chapter: i64,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub id: String,
}

#[derive(Deserialize)]
pub struct DeleteIndexParams {
    pub index: usize,
}

#[derive(Deserialize, Default)]
pub struct UpdateForeshadowParams {
    pub id: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub planted_chapter: Option<String>,
    pub expected_payoff: Option<String>,
    pub actual_payoff: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRuleParams {
    pub index: usize,
    pub content: String,
}

#[derive(Deserialize)]
pub struct AddRuleParams {
    pub content: String,
}

#[derive(Deserialize, Default)]
pub struct UpdateConceptParams {
    pub high_concept: Option<String>,
    pub premise: Option<String>,
    pub protagonist_hint: Option<String>,
    pub tone: Option<String>,
    pub central_conflict: Option<String>,
    pub inspiration: Option<String>,
}

/// 获取所有角色
pub async fn list_characters(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .characters
        .characters
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.to_string(),
                "name": c.name,
                "age": c.properties.age,
                "occupation": c.properties.occupation,
                "personality": c.properties.personality,
                "appearance": c.properties.appearance,
                "backstory": c.properties.backstory,
                "wants": c.properties.wants,
                "fears": c.properties.fears,
                "secret": c.properties.secret,
                "speech_style": c.properties.speech_style,
                // P0 档案化扩展
                "attire": c.properties.attire,
                "techniques": c.properties.techniques,
                "realm": c.properties.realm,
                "items": c.properties.items,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 获取所有地点
pub async fn list_locations(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .world
        .locations
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "name": s.name,
                "category": s.category,
                "rules": s.rules,
                "description": s.description,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 获取时间线
pub async fn list_timeline(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .world
        .timeline
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "name": e.name,
                "chapter_id": e.chapter_id,
                "story_time": e.story_time,
                "description": e.description,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 获取所有伏笔
pub async fn list_foreshadows(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    let result: Vec<serde_json::Value> = ontology
        .narrative
        .foreshadows
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.id.to_string(),
                "name": f.name,
                "description": f.description,
                "status": format!("{:?}", f.status),
                "planted_chapter": f.planted_chapter,
                "expected_payoff": f.expected_payoff,
                "actual_payoff": f.actual_payoff,
            })
        })
        .collect();

    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 添加伏笔（写入正典并落盘；埋设章号必须指向真实存在的章节）
pub async fn add_foreshadow(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddForeshadowParams>,
) -> Result<String, ApiError> {
    if params.planted_chapter <= 0 {
        return Err(ApiError::bad_request("埋设章节号必须 >= 1"));
    }
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("伏笔名不能为空"));
    }
    // 埋设章号必须存在，禁止静默挂到不存在的章（AGENTS.md 禁令 5）
    let chapter_exists = {
        let state = state.read().await;
        state
            .ontology
            .as_ref()
            .map(|o| o.chapters.iter().any(|c| c.chapter_no == params.planted_chapter))
            .unwrap_or(false)
    };
    if !chapter_exists {
        return Err(ApiError::bad_request(format!(
            "章节 {} 不存在，无法埋设伏笔（请选择已有章节）",
            params.planted_chapter
        )));
    }

    let foreshadow = Foreshadow::new(name, params.planted_chapter);
    let id = foreshadow.id.to_string();

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.narrative.foreshadows.push(foreshadow);
    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 获取世界观规则
pub async fn list_rules(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    serde_json::to_string(&ontology.world.rules).map_err(|e| ApiError::internal(e.to_string()))
}

/// 获取核心概念
pub async fn get_concept(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    serde_json::to_string(&ontology.core_concept).map_err(|e| ApiError::internal(e.to_string()))
}

/// 更新伏笔（状态机由约束引擎前置检查把关）
pub async fn update_foreshadow(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateForeshadowParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let fs_id = params.id.clone();
    let entity_ref = EntityRef::new(EntityType::Foreshadow, &fs_id);
    let proposed = serde_json::json!({
        "name": params.name,
        "status": params.status,
        "description": params.description,
        "planted_chapter": params.planted_chapter,
        "expected_payoff": params.expected_payoff,
        "actual_payoff": params.actual_payoff,
    });
    let pre_check = state.constraints.pre_edit_check(&entity_ref, &proposed);
    if let Some(violation) = pre_check
        .violations
        .iter()
        .find(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Error)
    {
        return Err(ApiError::bad_request(&violation.message));
    }

    let before = state.ontology.clone();
    {
        let ontology = state
            .ontology
            .as_mut()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let Some(foreshadow) = ontology.narrative.foreshadows.iter_mut().find(|f| f.id.to_string() == fs_id) else {
            return Err(ApiError::not_found("伏笔不存在"));
        };
        if let Some(name) = params.name {
            if name.trim().is_empty() {
                return Err(ApiError::bad_request("伏笔名不能为空"));
            }
            foreshadow.name = name;
        }
        if let Some(status_str) = params.status {
            let next_status = match status_str.as_str() {
                "Planned" => ForeshadowStatus::Planned,
                "Planted" => ForeshadowStatus::Planted,
                "Progressing" => ForeshadowStatus::Progressing,
                "Resolved" => ForeshadowStatus::Resolved,
                "Abandoned" => ForeshadowStatus::Abandoned,
                "Overdue" => ForeshadowStatus::Overdue,
                _ => return Err(ApiError::bad_request(format!("未知伏笔状态: {status_str}"))),
            };
            foreshadow.status = next_status;
        }
        if let Some(description) = crate::commands::params::parse_optional_string(params.description) {
            foreshadow.description = description.unwrap_or_default();
        }
        if let Some(chapter) = crate::commands::params::parse_optional_i64(
            params.planted_chapter.clone(),
            "planted_chapter",
        )? {
            if chapter <= 0 {
                return Err(ApiError::bad_request("埋设章节号必须 >= 1"));
            }
            foreshadow.planted_chapter = chapter;
        }
        match crate::commands::params::parse_clearable_i64(
            params.expected_payoff.clone(),
            "expected_payoff",
        )? {
            crate::commands::params::Clearable::Keep => {}
            crate::commands::params::Clearable::Clear => foreshadow.expected_payoff = None,
            crate::commands::params::Clearable::Set(value) => {
                foreshadow.expected_payoff = Some(value)
            }
        }
        match crate::commands::params::parse_clearable_i64(
            params.actual_payoff.clone(),
            "actual_payoff",
        )? {
            crate::commands::params::Clearable::Keep => {}
            crate::commands::params::Clearable::Clear => foreshadow.actual_payoff = None,
            crate::commands::params::Clearable::Set(value) => foreshadow.actual_payoff = Some(value),
        }
    }

    state.rebuild_derived();
    let post_check = state.constraints.post_edit_validate(&entity_ref);
    if let Some(violation) = post_check
        .violations
        .iter()
        .find(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Error)
    {
        state.ontology = before;
        state.rebuild_derived();
        return Err(ApiError::bad_request(format!(
            "伏笔修改被约束引擎拒绝：{}",
            violation.message
        )));
    }

    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除伏笔
pub async fn delete_foreshadow(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let id = params.id.clone();
    let len_before;
    {
        let ontology = state
            .ontology
            .as_mut()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        len_before = ontology.narrative.foreshadows.len();
        ontology.narrative.foreshadows.retain(|f| f.id.to_string() != id);
    }
    if state.ontology.as_ref().map(|o| o.narrative.foreshadows.len()) == Some(len_before) {
        return Err(ApiError::not_found("伏笔不存在"));
    }

    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 更新规则
pub async fn update_rule(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateRuleParams>,
) -> Result<String, ApiError> {
    if params.content.trim().is_empty() {
        return Err(ApiError::bad_request("规则内容不能为空"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    if params.index >= ontology.world.rules.len() {
        return Err(ApiError::not_found("规则索引越界"));
    }
    ontology.world.rules[params.index] = params.content;
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 添加规则
pub async fn add_rule(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddRuleParams>,
) -> Result<String, ApiError> {
    if params.content.trim().is_empty() {
        return Err(ApiError::bad_request("规则内容不能为空"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.world.rules.push(params.content);
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除规则
pub async fn delete_rule(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteIndexParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    if params.index >= ontology.world.rules.len() {
        return Err(ApiError::not_found("规则索引越界"));
    }
    ontology.world.rules.remove(params.index);
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 更新核心概念（至少提供一项修改）
pub async fn update_concept(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateConceptParams>,
) -> Result<String, ApiError> {
    if params.high_concept.is_none()
        && params.premise.is_none()
        && params.protagonist_hint.is_none()
        && params.tone.is_none()
        && params.central_conflict.is_none()
        && params.inspiration.is_none()
    {
        return Err(ApiError::bad_request("没有提供任何要更新的字段"));
    }

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;

    if let Some(v) = params.high_concept {
        ontology.core_concept.high_concept = v;
    }
    if let Some(v) = params.premise {
        ontology.core_concept.premise = v;
    }
    if let Some(v) = params.protagonist_hint {
        ontology.core_concept.protagonist_hint = v;
    }
    if let Some(v) = params.tone {
        ontology.core_concept.tone = v;
    }
    if let Some(v) = params.central_conflict {
        ontology.core_concept.central_conflict = v;
    }
    if let Some(v) = params.inspiration {
        ontology.core_concept.inspiration = v;
    }

    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

// ---- 写作风格笔记（正典 AestheticLayer，F13） ----

#[derive(Deserialize, Default)]
pub struct UpdateStyleParams {
    pub style_notes: Option<String>,
    pub pacing_notes: Option<String>,
}

/// 读取写作风格笔记
pub async fn get_style(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    serde_json::to_string(&serde_json::json!({
        "style_notes": ontology.aesthetic.style_notes,
        "pacing_notes": ontology.aesthetic.pacing_notes,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 更新写作风格笔记（落盘正典 AestheticLayer）
pub async fn update_style(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateStyleParams>,
) -> Result<String, ApiError> {
    if params.style_notes.is_none() && params.pacing_notes.is_none() {
        return Err(ApiError::bad_request("没有提供任何要更新的字段"));
    }
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    if let Some(v) = params.style_notes {
        ontology.aesthetic.style_notes = v;
    }
    if let Some(v) = params.pacing_notes {
        ontology.aesthetic.pacing_notes = v;
    }
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}
