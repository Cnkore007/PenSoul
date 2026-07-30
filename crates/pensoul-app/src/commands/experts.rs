//! 专家（女娲蒸馏）相关 IPC 命令
//!
//! 从 settings.rs 拆分而来，包含专家扫描、导入、删除等功能；
//! 蒸馏流程见 `expert_distill.rs`。
use crate::state::AppState;

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
        let found = scan_perspective_dirs(base)?;
        experts.extend(found);
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

    scan_perspective_dirs(&experts_path)
}

/// 扫描目录下所有 `*-perspective/SKILL.md`，解析为专家列表。
fn scan_perspective_dirs(base: &std::path::Path) -> Result<Vec<pensoul_core::Expert>, String> {
    let mut experts = Vec::new();
    if !base.exists() {
        return Ok(experts);
    }

    let entries = std::fs::read_dir(base).map_err(|e| format!("读取目录失败: {e}"))?;
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
        let content =
            std::fs::read_to_string(&skill_file).map_err(|e| format!("读取 SKILL.md 失败: {e}"))?;

        let (frontmatter, body) = parse_skill_md(&content);
        let fm_name = frontmatter
            .get("name")
            .cloned()
            .unwrap_or_else(|| dir_name.to_string());
        let fm_desc = frontmatter.get("description").cloned().unwrap_or_default();
        let persona = fm_name
            .strip_suffix("-perspective")
            .unwrap_or(&fm_name)
            .to_string();

        let identity = extract_section(&body, "身份卡");
        let perspective_text = extract_section(&body, "核心心智模型");
        let decision = extract_section(&body, "决策启发式");
        let expression = extract_section(&body, "表达DNA");

        let description = if identity.is_empty() {
            fm_desc.clone()
        } else {
            format!("{}\n\n{}", identity.trim(), fm_desc.trim())
        };
        let perspective = if perspective_text.is_empty() {
            persona.clone()
        } else {
            perspective_text.trim().to_string()
        };

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

        experts.push(pensoul_core::Expert {
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
        });
    }
    Ok(experts)
}

/// 解析 SKILL.md 的 YAML frontmatter（不需要 serde_yaml）
/// 返回 (frontmatter键值对, markdown body)
fn parse_skill_md(content: &str) -> (std::collections::HashMap<String, String>, String) {
    let mut fm = std::collections::HashMap::new();
    let body = if let Some(rest) = content.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let yaml_block = &rest[..end_idx];
            let markdown_body = &rest[end_idx + 4..]; // 跳过 "\n---"
            for line in yaml_block.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_string();
                    let value = value
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
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
            continue;
        }
        if capturing && line.trim().starts_with("## ") {
            break;
        }
        if capturing {
            result.push(*line);
        }
    }
    result.join("\n").trim().to_string()
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

/// 删除 Experts 文件夹中的技能目录（路径由 skill_path 指定）。
///
/// # 安全校验
/// 这是不可逆的递归删除，必须防止前端传入任意路径：
/// 1. 目标必须是名为 `SKILL.md` 的文件；
/// 2. 其父目录必须以 `-perspective` 结尾；
/// 3. 规范化（canonicalize）后的父目录必须位于受信任的根目录之一：
///    应用 Experts 目录、`~/.codex/skills`、`~/.agents/skills`。
#[tauri::command]
pub async fn delete_expert_skill(
    state: tauri::State<'_, AppState>,
    skill_path: String,
) -> Result<(), String> {
    let p = std::path::Path::new(&skill_path);
    if !p.exists() {
        return Ok(()); // 文件已不存在，视为成功
    }

    // 1. 必须是 SKILL.md 文件
    if p.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
        return Err("仅允许删除技能目录中的 SKILL.md 对应目录".to_string());
    }

    let parent = p
        .parent()
        .ok_or_else(|| "无效的技能路径".to_string())?
        .to_path_buf();

    // 2. 父目录必须是 <name>-perspective/
    let dir_name = parent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !dir_name.ends_with("-perspective") {
        return Err(format!(
            "目标目录不是技能目录（须以 -perspective 结尾）: {dir_name}"
        ));
    }

    // 3. 规范化后必须位于受信任的根目录内（防 `..` / 符号链接逃逸）
    let canonical = parent
        .canonicalize()
        .map_err(|e| format!("解析路径失败: {e}"))?;
    let mut trusted_roots = vec![experts_base_dir(&state)];
    if let Some(home) = dirs::home_dir() {
        trusted_roots.push(home.join(".codex").join("skills"));
        trusted_roots.push(home.join(".agents").join("skills"));
    }
    let in_trusted_root = trusted_roots.iter().any(|root| {
        root.canonicalize()
            .map(|r| canonical.starts_with(r))
            .unwrap_or(false)
    });
    if !in_trusted_root {
        return Err("目标目录不在受信任的 Experts 根目录内，拒绝删除".to_string());
    }

    std::fs::remove_dir_all(&canonical).map_err(|e| format!("删除技能目录失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn get_experts_folder(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(experts_base_dir(&state).to_string_lossy().to_string())
}

/// 计算 Experts 文件夹路径
pub(crate) fn experts_base_dir(state: &AppState) -> std::path::PathBuf {
    state
        .base_dir
        .parent()
        .map(|p| p.join("Experts"))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("Experts"))
}
