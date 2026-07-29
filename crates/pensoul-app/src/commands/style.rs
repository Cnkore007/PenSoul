/// 文风分析命令
use crate::state::AppState;

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
