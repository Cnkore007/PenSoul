//! 专家蒸馏 IPC 命令 —— 调用 LLM 将人物思维提炼为技能卡
//!
//! 从 experts.rs 拆分而来（单文件 500 行上限约束）。
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::experts::experts_base_dir;

/// 蒸馏阶段事件 —— 实时推送给前端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillPhaseEvent {
    pub phase: String,
    pub status: String,
    pub message: String,
    pub detail: String,
}

#[tauri::command]
pub async fn distill_expert(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    persona: String,
) -> Result<pensoul_core::Expert, String> {
    // 使用共享辅助模块
    use super::llm_helper as lh;
    lh::ensure_api_keys_loaded(&state);

    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);
    let api_keys = { state.api_keys.read().clone() };

    // 找第一个有 API Key 的供应商
    let (_provider_id, api_key, api_base) =
        lh::find_any_available_provider(&saved_providers, &api_keys)
            .ok_or_else(|| "未配置任何 LLM API Key，请在模型设置中配置".to_string())?;

    // 从已保存的模型中取第一个该供应商可用的模型
    let model_id = saved_models
        .iter()
        .find(|m| {
            m.get("provider_id").and_then(|v| v.as_str()) == Some(&_provider_id)
                && m.get("is_available")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .and_then(|m| m.get("model_id").and_then(|v| v.as_str()))
        .unwrap_or("gpt-4o");

    // Phase 1: 人物研究
    emit_phase(
        &app_handle,
        "人物研究",
        "running",
        &format!("正在搜集「{}」的背景与思维特征...", persona),
        "",
    )
    .ok();

    let research_prompt = format!(
        "你是一位人物分析师。请用简短的文字为「{}」提炼：
         1. 人物简介（一句话）
         2. 核心理念（一句话）
         3. 思维特征（一句话）
         4. 表达风格（一句话）
         5. 经典名言（1-2条）
请用中文。",
        persona
    );
    let research = lh::call_llm(&lh::ProviderAuth { provider_id: &_provider_id, api_key: &api_key, api_base: &api_base }, model_id, "你是一个专业的认知框架分析师。你的任务是提炼人物的思维方式和决策逻辑。回答简洁、有深度、直击本质。", &research_prompt, 0.7, 2048).await?;
    emit_phase(&app_handle, "人物研究", "done", "研究完成", &research).ok();

    // Phase 2: 生成技能卡并保存到 Experts 文件夹
    emit_phase(
        &app_handle,
        "技能生成",
        "running",
        &format!("正在为「{}」生成技能卡...", persona),
        "",
    )
    .ok();

    let skill_gen_prompt = format!(
        "基于以下关于「{}」的研究，生成一份结构化的创作思维技能。

{}\n
         请按以下格式输出（不要 JSON，用纯文本按章节输出）：
         ---
         【名称】
         【描述】
         【评审维度】
         【身份卡】以「我是谁」开头，第一人称，100字以内
         【心智模型】3-5句话描述
         【决策原则】3-5条，每条一句话
         【表达DNA】几句话描述
         【评审提示词】写给 AI 扮演的规则，第二人称「你」，约150字",
        persona, research
    );

    let skill_content = lh::call_llm(&lh::ProviderAuth { provider_id: &_provider_id, api_key: &api_key, api_base: &api_base }, model_id, "你是一个专业的认知框架分析师。你的任务是提炼人物的思维方式和决策逻辑。回答简洁、有深度、直击本质。", &skill_gen_prompt, 0.7, 2048).await?;

    // 从生成的文本中提取各个部分
    let name = extract_field(&skill_content, "【名称】");
    let description = extract_field(&skill_content, "【描述】");
    let perspective = extract_field(&skill_content, "【评审维度】");
    let identity_card = extract_field(&skill_content, "【身份卡】");
    let focus_dims = extract_field(&skill_content, "【核心关注维度】");
    let criteria = extract_field(&skill_content, "【判断标准】");
    let questions = extract_field(&skill_content, "【追问习惯】");
    let decision = extract_field(&skill_content, "【决策原则】");
    let expression = extract_field(&skill_content, "【表达DNA】");
    let default_prompt = extract_field(&skill_content, "【评审提示词】");
    let boundaries = extract_field(&skill_content, "【诚实边界】");

    let expert_name = if name.is_empty() { &persona } else { &name };

    // 保存到 Experts 文件夹
    let safe_name: String = expert_name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            _ => c,
        })
        .collect();
    let dir_name = format!("{}-perspective", safe_name);

    let experts_base = experts_base_dir(&state);
    let skill_dir = experts_base.join(&dir_name);
    let _ = std::fs::create_dir_all(&skill_dir);

    let review_framework = format!(
        "### 核心关注维度\n\n{}\n\n### 判断标准\n\n{}\n\n### 追问习惯\n\n{}",
        focus_dims.trim(),
        criteria.trim(),
        questions.trim()
    );
    let skill_md = format!(
        "---
name: {}
description: {}
---

# {} · PenSoul 创作思维

> {}

## 身份卡

{}

## 评审框架

{}

## 决策启发式

{}

## 表达DNA

{}

## 评审提示词

{}

## 诚实边界

{}",
        dir_name,
        description.trim(),
        expert_name,
        description.trim(),
        identity_card.trim(),
        review_framework,
        decision.trim(),
        expression.trim(),
        default_prompt.trim(),
        boundaries.trim()
    );

    let skill_file = skill_dir.join("SKILL.md");
    let _ = std::fs::write(&skill_file, &skill_md);

    let desc_combined = format!("【PenSoul技能】{} - {}", persona, description.trim());

    let expert = pensoul_core::Expert {
        id: format!("distilled-{}", uuid::Uuid::new_v4()),
        name: expert_name.to_string(),
        description: desc_combined,
        source_persona: persona.clone(),
        model_id: "gpt-4o".to_string(),
        perspective: perspective.trim().to_string(),
        default_prompt: default_prompt.trim().to_string(),
        created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        skill_path: Some(skill_file.to_string_lossy().to_string()),
        skill_summary: Some(format!("PenSoul技能 · {}", persona)),
    };

    emit_phase(
        &app_handle,
        "技能生成",
        "done",
        "技能生成完成！",
        &format!("已生成「{}」并保存到 Experts/{}", expert_name, dir_name),
    )
    .ok();
    Ok(expert)
}

/// 从 LLM 输出的纯文本中提取字段值
fn extract_field(text: &str, field_name: &str) -> String {
    let mut result = String::new();
    let mut capturing = false;
    for line in text.lines() {
        if line.trim().starts_with(field_name) {
            capturing = true;
            // 提取冒号后的内容
            if let Some((_, content)) = line.split_once('】') {
                let content = content.trim();
                if !content.is_empty() {
                    result.push_str(content);
                    result.push('\n');
                }
            }
            continue;
        }
        if capturing {
            if line.trim().starts_with("【") {
                break;
            }
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim().to_string()
}

/// 向 Tauri 前端发射蒸馏阶段事件
fn emit_phase(
    app_handle: &tauri::AppHandle,
    phase: &str,
    status: &str,
    message: &str,
    detail: &str,
) -> Result<(), String> {
    let event = DistillPhaseEvent {
        phase: phase.to_string(),
        status: status.to_string(),
        message: message.to_string(),
        detail: detail.to_string(),
    };
    let _ = app_handle.emit("distill-phase", event);
    Ok(())
}
