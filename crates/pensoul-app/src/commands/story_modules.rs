//! 爆款拆解模块库：从书籍蒸馏卡投影出可复用写作模块
//!
//! 不重新调 LLM——直接读 WritingCard/<书名>-book/<structure|tension>/SKILL.md，
//! 卡片六段（R 手法出处 / I 技法骨架 / A1 案例 / A2 适用场景 / B 边界）已含全部素材，
//! 这里把 A1 案例逐条投影为模块条目，供开书定盘时勾选注入。

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::book_distill::writing_cards_base_dir;
use super::experts::{extract_section_any, parse_skill_md};

/// 可复用的写作模块（灵感库，不是正典）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryModule {
    pub module_id: String,
    pub source_book: String,
    /// hook / opening / transition / ending / payoff / pacing / structure
    pub module_type: String,
    pub name: String,
    /// R 手法出处 + I 技法骨架（合并为一段可执行说明）
    pub technique: String,
    /// A1 书中案例（单条）
    pub example: String,
    /// A2 适用场景
    pub when_to_use: String,
    /// B 边界
    pub boundary: String,
    /// 适用工作流环节（outline_expand / chapter_writing / review）
    pub bound_stage: Vec<String>,
    pub favorite: bool,
}

/// 模块库只从这两张卡投影（结构与张力 = 布局与悬念）
const MODULE_DIMENSIONS: [&str; 2] = ["structure", "tension"];

/// 收藏文件：_config/story_modules.json（module_id -> 是否收藏）
fn favorites_path(state: &AppState) -> std::path::PathBuf {
    state.config_dir().join("story_modules.json")
}

fn load_favorites(state: &AppState) -> HashMap<String, bool> {
    let path = favorites_path(state);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<HashMap<String, bool>>(&s).ok())
        .unwrap_or_default()
}

fn save_favorites(state: &AppState, fav: &HashMap<String, bool>) -> Result<(), String> {
    let path = favorites_path(state);
    std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")))
        .map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(fav).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// 按内容关键词判定模块类型
fn classify_module(text: &str) -> String {
    if ["钩子", "悬念", "章末", "抓人"].iter().any(|k| text.contains(k)) {
        return "hook".to_string();
    }
    if ["开头", "开篇", "开场", "开卷", "起笔"].iter().any(|k| text.contains(k)) {
        return "opening".to_string();
    }
    if ["结尾", "结局", "收束", "收尾"].iter().any(|k| text.contains(k)) {
        return "ending".to_string();
    }
    if ["转场", "过渡", "转折", "视角切换"].iter().any(|k| text.contains(k)) {
        return "transition".to_string();
    }
    if ["爽点", "高潮", "爆发", "兑现", "爆点"].iter().any(|k| text.contains(k)) {
        return "payoff".to_string();
    }
    if ["节奏", "张力", "松紧", "起伏", "留白"].iter().any(|k| text.contains(k)) {
        return "pacing".to_string();
    }
    "structure".to_string()
}

/// A1 案例段按行拆成单条案例（过滤空行与列表符号）
fn split_examples(section: &str) -> Vec<String> {
    section
        .lines()
        .map(|l| l.trim().trim_start_matches(['-', '*', '•']).trim().to_string())
        .filter(|l| !l.is_empty() && l.chars().count() >= 6)
        .collect()
}

/// 从一张技能卡投影出模块条目
fn modules_from_card(
    package: &str,
    title: &str,
    dim: &str,
    content: &str,
    favorites: &HashMap<String, bool>,
) -> Vec<StoryModule> {
    let (fm, body) = parse_skill_md(content);
    let stages: Vec<String> = fm
        .get("applicable_stages")
        .map(|v| {
            v.trim_matches('[')
                .trim_matches(']')
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| {
            crate::commands::book_distill::DIMENSIONS
                .iter()
                .find(|(s, _, _, _)| *s == dim)
                .map(|(_, _, st, _)| st.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default()
        });
    let r = extract_section_any(&body, &["R · 手法出处", "R 手法出处", "R · 技法出处"]);
    let i = extract_section_any(&body, &["I · 技法骨架", "I 技法骨架"]);
    let a1 = extract_section_any(&body, &["A1 · 书中案例", "A1 书中案例"]);
    let a2 = extract_section_any(&body, &["A2 · 适用场景", "A2 适用场景"]);
    let b = extract_section_any(&body, &["B · 边界", "B 边界"]);
    let mut technique = String::new();
    if !r.is_empty() {
        technique.push_str(&format!("手法出处：{r}\n"));
    }
    if !i.is_empty() {
        technique.push_str(&format!("技法骨架：{i}\n"));
    }
    if !a2.is_empty() {
        technique.push_str(&format!("适用场景：{a2}\n"));
    }
    let examples = split_examples(&a1);
    if examples.is_empty() {
        return Vec::new();
    }
    examples
        .into_iter()
        .enumerate()
        .map(|(idx, ex)| {
            let name: String = ex.chars().take(18).collect();
            let module_id = format!("{package}-{dim}-{:02}", idx + 1);
            StoryModule {
                favorite: favorites.get(&module_id).copied().unwrap_or(false),
                module_id,
                source_book: title.to_string(),
                module_type: classify_module(&format!("{ex}\n{name}")),
                name: if name.len() >= 18 { format!("{name}…") } else { name },
                technique: technique.trim().to_string(),
                example: ex,
                when_to_use: a2.clone(),
                boundary: b.clone(),
                bound_stage: stages.clone(),
            }
        })
        .collect()
}

/// 列出全部模块库条目（扫描 WritingCard 下所有 -book 包）
#[tauri::command]
pub async fn list_modules(state: tauri::State<'_, AppState>) -> Result<Vec<StoryModule>, String> {
    let favorites = load_favorites(&state);
    let base = writing_cards_base_dir(&state);
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&base).map_err(|e| format!("读取 WritingCard 失败: {e}"))?;
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !dir_name.ends_with("-book") {
            continue;
        }
        // 包标题：优先 package.json，否则取目录名
        let title = std::fs::read_to_string(dir.join("package.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("title").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| dir_name.trim_end_matches("-book").to_string());
        for dim in MODULE_DIMENSIONS {
            let skill_file = dir.join(dim).join("SKILL.md");
            let Ok(content) = std::fs::read_to_string(&skill_file) else {
                continue;
            };
            out.extend(modules_from_card(dir_name, &title, dim, &content, &favorites));
        }
    }
    // 收藏的排前面，其余按书名稳定排序
    out.sort_by(|a, b| {
        b.favorite
            .cmp(&a.favorite)
            .then_with(|| a.source_book.cmp(&b.source_book))
            .then_with(|| a.module_id.cmp(&b.module_id))
    });
    Ok(out)
}

/// 收藏/取消收藏模块
#[tauri::command]
pub async fn save_module_favorite(
    state: tauri::State<'_, AppState>,
    module_id: String,
    favorite: bool,
) -> Result<(), String> {
    let mut fav = load_favorites(&state);
    if favorite {
        fav.insert(module_id, true);
    } else {
        fav.remove(&module_id);
    }
    save_favorites(&state, &fav)
}

/// 把勾选模块整理成注入 prompt 的文本（每条一句话：书 + 手法名 + 案例摘要）
pub(crate) fn module_ref_lines(modules: &[StoryModule]) -> String {
    modules
        .iter()
        .map(|m| {
            let ex: String = m.example.chars().take(60).collect();
            format!("- 《{}》·{}：{}（案例：{}）", m.source_book, m.module_type, m.name, ex)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一张符合蒸馏产物六段格式的技能卡
    fn sample_card(dim: &str) -> String {
        format!(
            "---\nname: 测试书-book-{dim}\ndescription: 测试卡\ndimension: {dim}\n\
             applicable_stages: [outline_expand, chapter_writing]\n---\n\
             ## R · 手法出处\n第3章 开篇\n\n\
             ## I · 技法骨架\n用异常事件开场，让主角带着误解进入冲突\n\n\
             ## A1 · 书中案例\n\
             1. 开头用陌生名字砸出悬念，主角误判身份\n\
             2. 章末用危机断章，钩住读者\n\
             3. 铺垫两章后爆发爽点，兑现读者期待\n\n\
             ## A2 · 适用场景\n开书/卷首；需要立刻抓住读者时；何时不绑：慢热题材\n\n\
             ## E · 执行步骤\n1. 开场300字内丢异常事件\n2. 章末留疑问\n\n\
             ## B · 边界\n连用会显得套路，最多每卷一次"
        )
    }

    #[test]
    fn test_modules_from_card_projects_examples() {
        let favorites = HashMap::new();
        let content = sample_card("tension");
        let modules = modules_from_card("测试书-book", "测试书", "tension", &content, &favorites);
        // A1 三条案例 → 三个模块
        assert_eq!(modules.len(), 3);
        // 类型分类：悬念/钩子 → hook，爽点 → payoff
        assert_eq!(modules[0].module_type, "hook");
        assert_eq!(modules[1].module_type, "hook");
        assert_eq!(modules[2].module_type, "payoff");
        // 六段素材完整落入字段
        assert!(modules[0].technique.contains("手法出处：第3章 开篇"));
        assert!(modules[0].technique.contains("技法骨架：用异常事件开场"));
        assert!(modules[0].technique.contains("适用场景：开书/卷首"));
        assert!(modules[0].boundary.contains("连用会显得套路"));
        assert_eq!(modules[0].bound_stage, vec!["outline_expand", "chapter_writing"]);
        assert_eq!(modules[0].source_book, "测试书");
        // id 稳定且唯一
        assert_eq!(modules[0].module_id, "测试书-book-tension-01");
    }

    #[test]
    fn test_modules_from_card_empty_a1_returns_empty() {
        let favorites = HashMap::new();
        let content = "## R · 手法出处\nx\n\n## I · 技法骨架\ny\n\n## A1 · 书中案例\n（无）\n";
        let modules = modules_from_card("pkg", "书", "structure", content, &favorites);
        assert!(modules.is_empty());
    }

    #[test]
    fn test_favorite_flag_from_persistence() {
        let mut favorites = HashMap::new();
        favorites.insert("测试书-book-tension-01".to_string(), true);
        let modules = modules_from_card("测试书-book", "测试书", "tension", &sample_card("tension"), &favorites);
        assert!(modules[0].favorite);
        assert!(!modules[1].favorite);
    }

    #[test]
    fn test_module_ref_lines_builds_injection_text() {
        let m = StoryModule {
            module_id: "m1".into(),
            source_book: "测试书".into(),
            module_type: "hook".into(),
            name: "身份悬念开场".into(),
            technique: "手法".into(),
            example: "开头用陌生名字砸出悬念".into(),
            when_to_use: "开书".into(),
            boundary: "".into(),
            bound_stage: vec!["outline_expand".into()],
            favorite: false,
        };
        let lines = module_ref_lines(&[m]);
        assert!(lines.contains("《测试书》·hook"));
        assert!(lines.contains("开头用陌生名字砸出悬念"));
    }
}
