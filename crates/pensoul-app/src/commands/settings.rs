/// 创作设定 IPC 命令
use crate::state::AppState;
use pensoul_core::ProjectSettings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Emitter;

/// 蒸馏阶段事件 —— 实时推送给前端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillPhaseEvent {
    pub phase: String,
    pub status: String,
    pub message: String,
    pub detail: String,
}

/// 扫描女娲蒸馏技能目录，返回可导入的专家列表
#[tauri::command]
pub async fn scan_nuwa_skills() -> Result<Vec<pensoul_core::Expert>, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    let mut experts = Vec::new();

    // 扫描两个可能的技能目录
    let search_dirs = [
        home.join(".codex").join("skills"),
        home.join(".agents").join("skills"),
    ];

    for base in &search_dirs {
        if !base.exists() {
            continue;
        }
        let entries = std::fs::read_dir(base).map_err(|e| format!("读取目录失败: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // 只处理 -perspective 结尾的目录
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !dir_name.ends_with("-perspective") {
                continue;
            }
            let skill_file = path.join("SKILL.md");
            if !skill_file.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&skill_file)
                .map_err(|e| format!("读取 SKILL.md 失败: {e}"))?;

            // 解析 YAML frontmatter 和 markdown body
            let (frontmatter, body) = parse_skill_md(&content);

            // 从 frontmatter 提取字段
            let fm_name = frontmatter
                .get("name")
                .cloned()
                .unwrap_or_else(|| dir_name.to_string());
            let fm_desc = frontmatter.get("description").cloned().unwrap_or_default();

            // 人名：去掉 -perspective 后缀
            let persona = fm_name
                .strip_suffix("-perspective")
                .unwrap_or(&fm_name)
                .to_string();

            // 从 markdown body 提取关键章节
            let identity = extract_section(&body, "身份卡");
            let perspective_text = extract_section(&body, "核心心智模型");
            let decision = extract_section(&body, "决策启发式");
            let expression = extract_section(&body, "表达DNA");

            // 组合 description：身份卡 + 英文 description
            let description = if identity.is_empty() {
                fm_desc.clone()
            } else {
                format!("{}\n\n{}", identity.trim(), fm_desc.trim())
            };

            // perspective：核心心智模型
            let perspective = if perspective_text.is_empty() {
                persona.clone()
            } else {
                perspective_text.trim().to_string()
            };

            // default_prompt：决策启发式 + 表达DNA
            let mut prompt_parts = Vec::new();
            if !decision.is_empty() {
                prompt_parts.push(format!("## 决策启发式\n{}", decision.trim()));
            }
            if !expression.is_empty() {
                prompt_parts.push(format!("## 表达DNA\n{}", expression.trim()));
            }
            let default_prompt = if prompt_parts.is_empty() {
                fm_desc.clone()
            } else {
                prompt_parts.join("\n\n")
            };

            let expert = pensoul_core::Expert {
                id: format!("nuwa-{}", uuid::Uuid::new_v4()),
                name: persona.clone(),
                description,
                source_persona: persona,
                model_id: "gpt-4o".to_string(),
                perspective,
                default_prompt,
                created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                skill_path: Some(skill_file.to_string_lossy().to_string()),
                skill_summary: Some(fm_desc),
            };
            experts.push(expert);
        }
    }
    Ok(experts)
}

/// 扫描本地 Experts 文件夹中预制的女娲蒸馏专家
#[tauri::command]
pub async fn scan_experts_folder(path: String) -> Result<Vec<pensoul_core::Expert>, String> {
    let experts_path = std::path::PathBuf::from(&path);

    if !experts_path.exists() || !experts_path.is_dir() {
        return Err(format!("目录不存在或不可读: {}", path));
    }

    let mut experts = Vec::new();
    let entries = std::fs::read_dir(&experts_path).map_err(|e| format!("读取目录失败: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !dir_name.ends_with("-perspective") {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&skill_file)
            .map_err(|e| format!("读取 SKILL.md 失败: {e}"))?;

        let (frontmatter, body) = parse_skill_md(&content);
        let fm_name = frontmatter.get("name").cloned().unwrap_or_else(|| dir_name.to_string());
        let fm_desc = frontmatter.get("description").cloned().unwrap_or_default();
        let persona = fm_name.strip_suffix("-perspective").unwrap_or(&fm_name).to_string();

        let identity = extract_section(&body, "身份卡");
        let perspective_text = extract_section(&body, "核心心智模型");
        let decision = extract_section(&body, "决策启发式");
        let expression = extract_section(&body, "表达DNA");

        let description = if identity.is_empty() { fm_desc.clone() } else { format!("{}\n\n{}", identity.trim(), fm_desc.trim()) };
        let perspective = if perspective_text.is_empty() { persona.clone() } else { perspective_text.trim().to_string() };

        let mut prompt_parts = Vec::new();
        if !decision.is_empty() { prompt_parts.push(format!("## 决策启发式\n{}", decision.trim())); }
        if !expression.is_empty() { prompt_parts.push(format!("## 表达DNA\n{}", expression.trim())); }
        let default_prompt = if prompt_parts.is_empty() { fm_desc.clone() } else { prompt_parts.join("\n\n") };

        let expert = pensoul_core::Expert {
            id: format!("nuwa-{}", uuid::Uuid::new_v4()),
            name: persona.clone(),
            description,
            source_persona: persona,
            model_id: "gpt-4o".to_string(),
            perspective,
            default_prompt,
            created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            skill_path: Some(skill_file.to_string_lossy().to_string()),
            skill_summary: Some(fm_desc),
        };
        experts.push(expert);
    }
    Ok(experts)
}

/// 解析 SKILL.md 的 YAML frontmatter（不需要 serde_yaml）
/// 返回 (frontmatter键值对, markdown body)
fn parse_skill_md(content: &str) -> (std::collections::HashMap<String, String>, String) {
    let mut fm = std::collections::HashMap::new();
    let body = if let Some(rest) = content.strip_prefix("---") {
        // 找到第二个 ---
        if let Some(end_idx) = rest.find("\n---") {
            let yaml_block = &rest[..end_idx];
            let markdown_body = &rest[end_idx + 4..]; // 跳过 "\n---"
            // 简单解析 key: value
            for line in yaml_block.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_string();
                    let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !key.is_empty() {
                        fm.insert(key, value);
                    }
                }
            }
            markdown_body.to_string()
        } else {
            content.to_string()
        }
    } else {
        content.to_string()
    };
    (fm, body)
}

/// 从 markdown body 中提取某个 ## 标题下的内容，直到下一个 ## 标题
fn extract_section(body: &str, heading: &str) -> String {
    let marker = format!("## {}", heading);
    let lines: Vec<&str> = body.lines().collect();
    let mut capturing = false;
    let mut result = Vec::new();

    for line in &lines {
        if line.trim().starts_with(&marker) {
            capturing = true;
            // 不包含标题行本身
            continue;
        }
        if capturing && line.trim().starts_with("## ") {
            break; // 遇到下一个二级标题，停止
        }
        if capturing {
            result.push(*line);
        }
    }
    result.join("\n").trim().to_string()
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
#[tauri::command]
pub async fn save_sprout(
    state: tauri::State<'_, AppState>,
    sprout: pensoul_core::SproutData,
) -> Result<(), String> {
    {
        let mut ontology = state.ontology.write();
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

/// 保存专家列表到后端（全局存储，不绑定项目）
#[tauri::command]
pub async fn save_experts(
    state: tauri::State<'_, AppState>,
    experts: Vec<pensoul_core::Expert>,
) -> Result<(), String> {
    let config_dir = state.config_dir();
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    let path = config_dir.join("experts.json");
    let list = pensoul_core::ExpertList { experts };
    let json =
        serde_json::to_string_pretty(&list).map_err(|e| format!("序列化专家列表失败: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入专家文件失败: {e}"))?;
    Ok(())
}

/// 从后端读取专家列表（全局存储）
#[tauri::command]
pub async fn load_experts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<pensoul_core::Expert>, String> {
    let path = state.config_dir().join("experts.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("读取专家文件失败: {e}"))?;
    let list: pensoul_core::ExpertList =
        serde_json::from_str(&json).map_err(|e| format!("反序列化专家列表失败: {e}"))?;
    Ok(list.experts)
}

/// 删除 Experts 文件夹中的技能目录（路径由 skill_path 指定）
#[tauri::command]
pub async fn delete_expert_skill(skill_path: String) -> Result<(), String> {
    let p = std::path::Path::new(&skill_path);
    if !p.exists() {
        return Ok(()); // 文件已不存在，视为成功
    }
    // 删除 SKILL.md 所在的目录（即 <name>-perspective/）
    if let Some(parent) = p.parent() {
        if parent.exists() {
            std::fs::remove_dir_all(parent).map_err(|e| format!("删除技能目录失败: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_experts_folder(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(experts_base_dir(&state).to_string_lossy().to_string())
}

/// 计算 Experts 文件夹路径
fn experts_base_dir(state: &AppState) -> std::path::PathBuf {
    state.base_dir.parent()
        .map(|p| p.join("Experts"))
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_default().join("Experts")
        })
}
#[tauri::command]
pub async fn distill_expert(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    persona: String,
) -> Result<pensoul_core::Expert, String> {
    let api_keys: HashMap<String, String> = { let keys = state.api_keys.read(); keys.clone() };

    // 按 providers.json 中注册的供应商顺序，找第一个有 API Key 的
    let config_dir = state.config_dir();
    let providers_path = config_dir.join("providers.json");
    let models_path = config_dir.join("models.json");
    let saved_providers: Vec<serde_json::Value> = std::fs::read_to_string(&providers_path)
        .ok().and_then(|d| serde_json::from_str(&d).ok()).unwrap_or_default();
    let saved_models: Vec<serde_json::Value> = std::fs::read_to_string(&models_path)
        .ok().and_then(|d| serde_json::from_str(&d).ok()).unwrap_or_default();

    let (_provider_id, api_key, api_base) = saved_providers.iter()
        .filter_map(|p| {
            let pid = p.get("provider_id").and_then(|v| v.as_str())?;
            let key = api_keys.get(pid)?;
            let base = p.get("api_base").and_then(|v| v.as_str())?;
            Some((pid.to_string(), key.clone(), base.to_string()))
        })
        .next()
        .ok_or_else(|| "未配置任何 LLM API Key，请在模型设置中配置".to_string())?;

    // 从已保存的模型中取第一个该供应商可用的模型，不硬编码 gpt-4o
    let model_id = saved_models.iter()
        .find(|m| {
            m.get("provider_id").and_then(|v| v.as_str()) == Some(&_provider_id)
                && m.get("is_available").and_then(|v| v.as_bool()).unwrap_or(false)
        })
        .and_then(|m| m.get("model_id").and_then(|v| v.as_str()))
        .unwrap_or("gpt-4o");

    // Phase 1: 人物研究
    emit_phase(&app_handle, "人物研究", "running",
        &format!("正在搜集「{}」的背景与思维特征...", persona), "").ok();

    let research_prompt = format!(
        "你是一位人物分析师。请用简短的文字为「{}」提炼：
         1. 人物简介（一句话）
         2. 核心理念（一句话）
         3. 思维特征（一句话）
         4. 表达风格（一句话）
         5. 经典名言（1-2条）
请用中文。", persona
    );
    let research = call_llm(&api_key, &api_base, model_id, &research_prompt).await?;
    emit_phase(&app_handle, "人物研究", "done", "研究完成", &research).ok();

    // Phase 2: 生成技能卡并保存到 Experts 文件夹
    emit_phase(&app_handle, "技能生成", "running",
        &format!("正在为「{}」生成技能卡...", persona), "").ok();

    // 先让 LLM 以纯文本形式生成技能内容，避免 JSON 嵌套问题
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
         【评审提示词】写给 AI 扮演的规则，第二人称「你」，约150字", persona, research
    );

    let skill_content = call_llm(&api_key, &api_base, model_id, &skill_gen_prompt).await?;

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
    let safe_name: String = expert_name.chars()
        .map(|c| match c { '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_', _ => c })
        .collect();
    let dir_name = format!("{}-perspective", safe_name);

    let experts_base = experts_base_dir(&state);
    let skill_dir = experts_base.join(&dir_name);
    let _ = std::fs::create_dir_all(&skill_dir);

    let review_framework = format!(
        "### 核心关注维度\n\n{}\n\n### 判断标准\n\n{}\n\n### 追问习惯\n\n{}",
        focus_dims.trim(), criteria.trim(), questions.trim()
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
        dir_name, description.trim(), expert_name, description.trim(),
        identity_card.trim(), review_framework, decision.trim(),
        expression.trim(), default_prompt.trim(), boundaries.trim()
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

    emit_phase(&app_handle, "技能生成", "done",
        "技能生成完成！", &format!("已生成「{}」并保存到 Experts/{}", expert_name, dir_name)).ok();
    Ok(expert)
}

/// 从 LLM 输出的纯文本中提取字段值
fn extract_field<'a>(text: &'a str, field_name: &str) -> String {
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
fn emit_phase(app_handle: &tauri::AppHandle, phase: &str, status: &str, message: &str, detail: &str) -> Result<(), String> {
    let event = DistillPhaseEvent {
        phase: phase.to_string(),
        status: status.to_string(),
        message: message.to_string(),
        detail: detail.to_string(),
    };
    let _ = app_handle.emit("distill-phase", event);
    Ok(())
}

/// 调用 LLM API
async fn call_llm(api_key: &str, api_base: &str, model_id: &str, prompt: &str) -> Result<String, String> {
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model_id,
        "messages": [
            { "role": "system", "content": "你是一个专业的认知框架分析师。你的任务是提炼人物的思维方式和决策逻辑。回答简洁、有深度、直击本质。" },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.7,
        "max_tokens": 2048
    });
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120)).build().map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let response = client.post(&url).header("Authorization", format!("Bearer {}", api_key)).header("Content-Type", "application/json").json(&body).send().await.map_err(|e| format!("LLM 请求失败: {}", e))?;
    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();
    if !status.is_success() { return Err(format!("LLM API 错误 ({}): {}", status, body_text)); }
    let json: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| format!("解析 LLM 响应失败: {}", e))?;
    Ok(json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}
