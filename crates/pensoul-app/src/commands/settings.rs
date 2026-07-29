/// 创作设定 IPC 命令
use crate::state::AppState;
use pensoul_core::ProjectSettings;

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
