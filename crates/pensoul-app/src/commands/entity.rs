// entity.rs — 实体管理 API

use axum::extract::{Form, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::*;
use pensoul_domain::entity::EntityRef;

#[derive(Deserialize)]
pub struct AddCharacterParams {
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddEventParams {
    pub name: String,
    pub chapter_id: i64,
}

#[derive(Deserialize)]
pub struct AddSettingParams {
    pub name: String,
    pub category: String,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub id: String,
}

#[derive(Deserialize, Default)]
pub struct UpdateCharacterParams {
    pub id: String,
    pub name: Option<String>,
    pub age: Option<String>,
    pub occupation: Option<String>,
    pub appearance: Option<String>,
    pub backstory: Option<String>,
    pub wants: Option<String>,
    pub fears: Option<String>,
    pub secret: Option<String>,
    // P0 档案化扩展（人物档案）
    pub attire: Option<String>,
    /// 逗号分隔的功法列表
    pub techniques: Option<String>,
    pub realm: Option<String>,
    /// 逗号分隔的法宝列表
    pub items: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct UpdateEventParams {
    pub id: String,
    pub name: Option<String>,
    pub chapter_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct UpdateSettingParams {
    pub id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}

/// 添加角色（写入正典并落盘）
pub async fn add_character(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddCharacterParams>,
) -> Result<String, ApiError> {
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("角色名不能为空"));
    }

    let character = Character::new(name);
    let id = character.id.to_string();

    let mut state = state.write().await;
    let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.characters.characters.push(character);
    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 添加事件（校验章号存在，写入正典并落盘）
pub async fn add_event(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddEventParams>,
) -> Result<String, ApiError> {
    if params.chapter_id <= 0 {
        return Err(ApiError::bad_request("章节号必须 >= 1"));
    }
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("事件名不能为空"));
    }
    // 章号必须指向真实存在的章节，禁止静默挂到不存在的章（AGENTS.md 禁令 5）
    let chapter_exists = {
        let state = state.read().await;
        state
            .ontology
            .as_ref()
            .map(|o| o.chapters.iter().any(|c| c.chapter_no == params.chapter_id))
            .unwrap_or(false)
    };
    if !chapter_exists {
        return Err(ApiError::bad_request(format!(
            "章节 {} 不存在，无法添加事件（请选择已有章节）",
            params.chapter_id
        )));
    }

    let event = Event::new(name, params.chapter_id);
    let id = event.id.to_string();

    let mut state = state.write().await;
    let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.world.timeline.push(event);
    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 添加设定（写入正典并落盘）
pub async fn add_setting(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddSettingParams>,
) -> Result<String, ApiError> {
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("设定名不能为空"));
    }

    let setting = Setting::new(name, params.category.trim());
    let id = setting.id.to_string();

    let mut state = state.write().await;
    let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.world.locations.push(setting);
    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;

    Ok(id)
}

/// 获取所有实体（来自派生图谱，保证与正典一致）
pub async fn list_entities(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let entities: Vec<serde_json::Value> = state
        .graph
        .all_entities()
        .map(|e| {
            serde_json::json!({
                "id": e.entity_id(),
                "type": format!("{:?}", e.entity_type()),
                "name": e.name(),
            })
        })
        .collect();
    serde_json::to_string(&entities).map_err(|e| ApiError::internal(e.to_string()))
}

/// 更新角色（约束前置检查 → 应用 → 验证 → 落盘）
pub async fn update_character(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateCharacterParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let character_id = params.id.clone();

    // 前置检查（实体存在性 + 变更合法性）
    let entity_ref = EntityRef::new(EntityType::Character, &character_id);
    let proposed = serde_json::json!({
        "name": params.name,
        "age": params.age,
        "occupation": params.occupation,
        "appearance": params.appearance,
        "backstory": params.backstory,
        "wants": params.wants,
        "fears": params.fears,
        "secret": params.secret,
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
    let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
    let Some(character) = ontology.characters.characters.iter_mut().find(|c| c.id.to_string() == character_id) else {
        return Err(ApiError::not_found("角色不存在"));
    };

    if let Some(name) = params.name {
        if name.trim().is_empty() {
            return Err(ApiError::bad_request("角色名不能为空"));
        }
        character.name = name;
    }
    if let Some(age) = crate::commands::params::parse_optional_i64(params.age.clone(), "age")? {
        character.properties.age = Some(age as i32);
    }
    if let Some(occupation) = crate::commands::params::parse_optional_string(params.occupation) {
        character.properties.occupation = occupation;
    }
    if let Some(appearance) = crate::commands::params::parse_optional_string(params.appearance) {
        character.properties.appearance = appearance;
    }
    if let Some(backstory) = crate::commands::params::parse_optional_string(params.backstory) {
        character.properties.backstory = backstory;
    }
    if let Some(wants) = crate::commands::params::parse_optional_string(params.wants) {
        character.properties.wants = wants;
    }
    if let Some(fears) = crate::commands::params::parse_optional_string(params.fears) {
        character.properties.fears = fears;
    }
    if let Some(secret) = crate::commands::params::parse_optional_string(params.secret) {
        character.properties.secret = secret;
    }
    if let Some(attire) = crate::commands::params::parse_optional_string(params.attire) {
        character.properties.attire = attire;
    }
    if let Some(techniques) = params.techniques {
        character.properties.techniques = split_csv(&techniques);
    }
    if let Some(realm) = crate::commands::params::parse_optional_string(params.realm) {
        character.properties.realm = realm;
    }
    if let Some(items) = params.items {
        character.properties.items = split_csv(&items);
    }

    state.rebuild_derived();
    validate_after_edit(&mut state, &entity_ref, before, "角色")?;
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除角色
pub async fn delete_character(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let id = params.id.clone();
    let len_before;
    {
        let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
        len_before = ontology.characters.characters.len();
        ontology.characters.characters.retain(|c| c.id.to_string() != id);
    }
    if state.ontology.as_ref().map(|o| o.characters.characters.len()) == Some(len_before) {
        return Err(ApiError::not_found("角色不存在"));
    }

    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 更新事件
pub async fn update_event(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateEventParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let event_id = params.id.clone();
    let entity_ref = EntityRef::new(EntityType::Event, &event_id);
    let proposed = serde_json::json!({
        "name": params.name,
        "chapter_id": params.chapter_id,
        "description": params.description,
    });
    reject_error_violations(&state, &entity_ref, &proposed)?;

    let before = state.ontology.clone();
    {
        let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
        let Some(event) = ontology.world.timeline.iter_mut().find(|e| e.id.to_string() == event_id) else {
            return Err(ApiError::not_found("事件不存在"));
        };
        if let Some(name) = params.name {
            if name.trim().is_empty() {
                return Err(ApiError::bad_request("事件名不能为空"));
            }
            event.name = name;
        }
        if let Some(chapter_id) =
            crate::commands::params::parse_optional_i64(params.chapter_id.clone(), "chapter_id")?
        {
            if chapter_id <= 0 {
                return Err(ApiError::bad_request("章节号必须 >= 1"));
            }
            event.chapter_id = chapter_id;
        }
        if let Some(description) = crate::commands::params::parse_optional_string(params.description) {
            event.description = description.unwrap_or_default();
        }
    }

    state.rebuild_derived();
    validate_after_edit(&mut state, &entity_ref, before, "事件")?;
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除事件
pub async fn delete_event(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let id = params.id.clone();
    let len_before;
    {
        let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
        len_before = ontology.world.timeline.len();
        ontology.world.timeline.retain(|e| e.id.to_string() != id);
    }
    if state.ontology.as_ref().map(|o| o.world.timeline.len()) == Some(len_before) {
        return Err(ApiError::not_found("事件不存在"));
    }

    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 更新设定
pub async fn update_setting(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateSettingParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let setting_id = params.id.clone();
    let entity_ref = EntityRef::new(EntityType::Setting, &setting_id);
    let proposed = serde_json::json!({
        "name": params.name,
        "category": params.category,
        "description": params.description,
    });
    reject_error_violations(&state, &entity_ref, &proposed)?;

    let before = state.ontology.clone();
    {
        let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
        let Some(setting) = ontology.world.locations.iter_mut().find(|s| s.id.to_string() == setting_id) else {
            return Err(ApiError::not_found("地点不存在"));
        };
        if let Some(name) = params.name {
            if name.trim().is_empty() {
                return Err(ApiError::bad_request("设定名不能为空"));
            }
            setting.name = name;
        }
        if let Some(category) = crate::commands::params::parse_optional_string(params.category) {
            setting.category = category.unwrap_or_default();
        }
        if let Some(description) = crate::commands::params::parse_optional_string(params.description) {
            setting.description = description.unwrap_or_default();
        }
    }

    state.rebuild_derived();
    validate_after_edit(&mut state, &entity_ref, before, "地点")?;
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除设定
pub async fn delete_setting(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let id = params.id.clone();
    let len_before;
    {
        let ontology = state.ontology.as_mut().ok_or(ApiError::bad_request("没有打开的项目"))?;
        len_before = ontology.world.locations.len();
        ontology.world.locations.retain(|s| s.id.to_string() != id);
    }
    if state.ontology.as_ref().map(|o| o.world.locations.len()) == Some(len_before) {
        return Err(ApiError::not_found("地点不存在"));
    }

    state.rebuild_derived();
    state
        .save_project()
        .map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 前置检查：存在 Error 级违规则拒绝修改
fn reject_error_violations(
    state: &AppState,
    entity_ref: &EntityRef,
    proposed: &serde_json::Value,
) -> Result<(), ApiError> {
    let check = state.constraints.pre_edit_check(entity_ref, proposed);
    if let Some(violation) = check
        .violations
        .iter()
        .find(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Error)
    {
        return Err(ApiError::bad_request(&violation.message));
    }
    Ok(())
}

/// 修改后验证：有 Error 级违规则回滚到修改前
fn validate_after_edit(
    state: &mut AppState,
    entity_ref: &EntityRef,
    before: Option<pensoul_domain::ontology::NovelOntology>,
    label: &str,
) -> Result<(), ApiError> {
    let check = state.constraints.post_edit_validate(entity_ref);
    if let Some(violation) = check
        .violations
        .iter()
        .find(|v| v.severity == pensoul_domain::constraint::ViolationSeverity::Error)
    {
        state.ontology = before;
        state.rebuild_derived();
        return Err(ApiError::bad_request(format!(
            "{label} 修改被约束引擎拒绝：{}",
            violation.message
        )));
    }
    Ok(())
}

// ---- 组织档案 CRUD（P0） ----

#[derive(Deserialize)]
pub struct AddOrganizationParams {
    pub name: String,
    pub category: String,
}

#[derive(Deserialize, Default)]
pub struct UpdateOrganizationParams {
    pub id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub structure: Option<String>,
    pub goals: Option<String>,
    pub rules: Option<String>,
    pub description: Option<String>,
}

/// 列出全部组织档案
pub async fn list_organizations(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let state = state.read().await;
    let ontology = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    serde_json::to_string(&ontology.world.organizations)
        .map_err(|e| ApiError::internal(e.to_string()))
}

/// 添加组织档案（写入正典并落盘）
pub async fn add_organization(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AddOrganizationParams>,
) -> Result<String, ApiError> {
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("组织名不能为空"));
    }
    let organization = Organization::new(name, params.category.trim());
    let id = organization.id.to_string();

    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    ontology.world.organizations.push(organization);
    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;
    Ok(id)
}

/// 更新组织档案
pub async fn update_organization(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<UpdateOrganizationParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let org_id = params.id.clone();
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let Some(org) = ontology
        .world
        .organizations
        .iter_mut()
        .find(|o| o.id.to_string() == org_id)
    else {
        return Err(ApiError::not_found("组织不存在"));
    };

    if let Some(name) = params.name {
        if name.trim().is_empty() {
            return Err(ApiError::bad_request("组织名不能为空"));
        }
        org.name = name;
    }
    if let Some(category) = crate::commands::params::parse_optional_string(params.category) {
        org.category = category.unwrap_or_default();
    }
    if let Some(structure) = crate::commands::params::parse_optional_string(params.structure) {
        org.structure = structure.unwrap_or_default();
    }
    if let Some(goals) = crate::commands::params::parse_optional_string(params.goals) {
        org.goals = goals.unwrap_or_default();
    }
    if let Some(rules) = params.rules {
        org.rules = split_csv(&rules);
    }
    if let Some(description) = crate::commands::params::parse_optional_string(params.description) {
        org.description = description.unwrap_or_default();
    }

    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 删除组织档案
pub async fn delete_organization(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<DeleteParams>,
) -> Result<String, ApiError> {
    let mut state = state.write().await;
    let ontology = state
        .ontology
        .as_mut()
        .ok_or(ApiError::bad_request("没有打开的项目"))?;
    let before = ontology.world.organizations.len();
    ontology
        .world
        .organizations
        .retain(|o| o.id.to_string() != params.id);
    if ontology.world.organizations.len() == before {
        return Err(ApiError::not_found("组织不存在"));
    }
    state.rebuild_derived();
    state.save_project().map_err(ApiError::internal)?;
    Ok("ok".to_string())
}

/// 逗号分隔 → 字符串列表（去空白、去空项）
fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
