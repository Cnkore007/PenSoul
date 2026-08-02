//! 本体数据读写命令 —— 世界观 / 人物志 / 文风 / 一致性 / 创作设定 / 核心概念 / 萌芽 / 工作流配置
use crate::state::AppState;
use pensoul_core::ProjectSettings;

/// 获取世界观数据
#[tauri::command]
pub async fn get_world(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    serde_json::to_value(&ontology.world).map_err(|e| e.to_string())
}

/// 保存世界观数据
#[tauri::command]
pub async fn save_world(
    state: tauri::State<'_, AppState>,
    world: serde_json::Value,
) -> Result<(), String> {
    let layer: pensoul_core::WorldLayer =
        serde_json::from_value(world).map_err(|e| e.to_string())?;
    let samples = {
        let onto = state.ontology.read();
        crate::edits::world_diff_samples(&onto.world, &layer)
    };
    {
        let mut ontology = state.ontology.write();
        ontology.world = layer;
    }
    crate::edits::record_edit_samples(&state, samples);
    state.save().map_err(|e| e.to_string())
}

/// 获取所有角色
#[tauri::command]
pub async fn get_characters(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    serde_json::to_value(&ontology.characters).map_err(|e| e.to_string())
}

/// 保存角色列表
#[tauri::command]
pub async fn save_characters(
    state: tauri::State<'_, AppState>,
    characters: serde_json::Value,
) -> Result<(), String> {
    let mut layer: pensoul_core::CharacterLayer =
        serde_json::from_value(characters).map_err(|e| e.to_string())?;
    // 关系去重防线：同 from+to+relation_type 只保留第一条，防止前端/历史 bug 成倍膨胀
    let mut seen = std::collections::HashSet::new();
    layer.relationships.retain(|r| {
        let key = format!("{}|{}|{}", r.from, r.to, r.relation_type);
        seen.insert(key)
    });
    let samples = {
        let onto = state.ontology.read();
        crate::edits::characters_diff_samples(&onto.characters, &layer)
    };
    {
        let mut ontology = state.ontology.write();
        ontology.characters = layer;
    }
    crate::edits::record_edit_samples(&state, samples);
    state.save().map_err(|e| e.to_string())
}

/// 获取文风指标
#[tauri::command]
pub async fn get_style_metrics(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    let aesthetic = &ontology.aesthetic;
    let fp = &aesthetic.style_fingerprint;
    let pm = &aesthetic.pacing_model;

    // 从反 AI 规则数计算 ai_pattern_score（规则数 / 最大可能值）
    let rule_count = aesthetic.anti_ai_rules.len() as f32;
    let ai_pattern_score = (rule_count / 10.0).min(1.0);

    let metrics = serde_json::json!({
        "avg_sentence_length": fp.sentence_length_avg,
        "vocabulary_richness": fp.vocabulary_richness,
        "dialogue_ratio": fp.dialogue_ratio,
        "pace_score": pm.action_ratio,
        "ai_pattern_score": ai_pattern_score,
    });
    Ok(metrics)
}

/// 全书一致性检查
#[tauri::command]
pub async fn check_consistency(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let checker = state.consistency_checker.read();
    let report = checker.check_all();
    let violations: Vec<serde_json::Value> = report
        .violations
        .iter()
        .filter_map(|v| serde_json::to_value(v).ok())
        .collect();
    Ok(violations)
}

/// 保存创作设定到后端
#[tauri::command]
pub async fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: ProjectSettings,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.settings = settings;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取创作设定
#[tauri::command]
pub async fn load_settings(state: tauri::State<'_, AppState>) -> Result<ProjectSettings, String> {
    let ontology = state.ontology.read();
    Ok(ontology.settings.clone())
}

/// 保存核心概念到后端
#[tauri::command]
pub async fn save_concept(
    state: tauri::State<'_, AppState>,
    concept: pensoul_core::CoreConcept,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.core_concept = concept;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取核心概念
#[tauri::command]
pub async fn load_concept(
    state: tauri::State<'_, AppState>,
) -> Result<pensoul_core::CoreConcept, String> {
    let ontology = state.ontology.read();
    Ok(ontology.core_concept.clone())
}

/// 保存萌芽数据到后端
///
/// 讨论结果（last_discussion）由讨论命令在后台持久化；前端常规保存
/// 可能携带过期的 None（例如讨论在后台完成时用户正在别的页面编辑），
/// 此时保留后端已有的讨论结果，避免被旧副本覆盖。
#[tauri::command]
pub async fn save_sprout(
    state: tauri::State<'_, AppState>,
    sprout: pensoul_core::SproutData,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        let mut sprout = sprout;
        if sprout.last_discussion.is_none() {
            sprout.last_discussion = ontology.sprout.last_discussion.clone();
        }
        ontology.sprout = sprout;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取萌芽数据
#[tauri::command]
pub async fn load_sprout(
    state: tauri::State<'_, AppState>,
) -> Result<pensoul_core::SproutData, String> {
    let ontology = state.ontology.read();
    Ok(ontology.sprout.clone())
}

/// 保存工作流技能配置（环节 → 模型 + 技法卡绑定，结构由前端定义，透明存储）
#[tauri::command]
pub async fn save_workflow_skills(
    state: tauri::State<'_, AppState>,
    config: serde_json::Value,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
        ontology.workflow_skills = config;
    }
    state.save().map_err(|e| e.to_string())
}

/// 从后端读取工作流技能配置（未配置过返回 null）
#[tauri::command]
pub async fn load_workflow_skills(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let ontology = state.ontology.read();
    Ok(ontology.workflow_skills.clone())
}
