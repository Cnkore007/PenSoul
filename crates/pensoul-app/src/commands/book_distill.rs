//! 书籍蒸馏 IPC 命令 —— 加载 pensoul-skill-Books 技能，按其方法论调用 LLM
//! 将一本书的写作方法提炼为可绑定工作流环节的写作技能卡组。
//!
//! 产物约定（与 skills/pensoul-skill-Books 一致）：
//! - 目录：`WritingCard/<书名>-book/`
//! - 五维度卡：style / structure / character / tension / genre 各一张 SKILL.md
//! - 卡片六段：R 手法出处 / I 技法骨架 / A1 书中案例 / A2 适用场景 / E 执行步骤 / B 边界
//! - 蒸馏过程写入 `references/research/` 随产物自包含保存
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::book_file;
use super::expert_distill::{DistillRunGuard, extract_skill_md, normalize_frontmatter_name};
use super::experts::{extract_section_any, parse_skill_md};
use super::llm_helper as lh;

/// 技能文件在工作区内的相对路径
const SKILL_RELATIVE_PATH: &str = "skills/pensoul-skill-Books/SKILL.md";

/// 五维度定义：(slug, 中文名, 适用环节, 提取重点)
/// 环节 key 与前端工作流配置一致：outline_expand / chapter_writing / review
pub(crate) const DIMENSIONS: [(&str, &str, &[&str], &str); 5] = [
    (
        "style",
        "文风 DNA",
        &["chapter_writing", "review"],
        "句式、词汇、节奏、视角声音、叙事距离",
    ),
    (
        "structure",
        "叙事结构",
        &["outline_expand"],
        "篇章布局、信息揭示顺序、伏笔与回收、视角调度",
    ),
    (
        "character",
        "人物塑造",
        &["outline_expand", "chapter_writing"],
        "角色登场、动机呈现、成长弧线、群像调度",
    ),
    (
        "tension",
        "冲突与张力",
        &["outline_expand", "chapter_writing"],
        "悬念引擎、张力曲线、场景冲突设计、章末钩子",
    ),
    (
        "genre",
        "类型范式",
        &["outline_expand"],
        "题材惯例、读者期待管理、范式突破点",
    ),
];

/// 单张技能卡的展示信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookCardInfo {
    pub dimension: String,
    pub dimension_label: String,
    pub name: String,
    pub description: String,
    pub skill_path: String,
    pub applicable_stages: Vec<String>,
}

/// 技能包（一次蒸馏的产物）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookPackage {
    pub package: String,
    pub title: String,
    pub author: String,
    pub created_at: String,
    pub cards: Vec<BookCardInfo>,
}

/// 找不到技能文件时的内置简版方法论（保证发布版可用）
const FALLBACK_METHODOLOGY: &str = r#"# PenSoul · 书籍写作蒸馏术（简版）

核心理念：提炼写法，不提炼内容。捕捉 HOW it is written，不是内容梗概。

产物红线：技能卡中不保留内容梗概——不写情节摘要、不抄书摘、不做读后感。

五维度：style 文风 DNA / structure 叙事结构 / character 人物塑造 / tension 冲突与张力 / genre 类型范式。

技法三重验证：跨章复现（书中≥2处体现）、生成力（能指导没写过的新场景）、
独特性（不是所有作者都这样做）。三重通过才是技法，每维取 1-3 个，宁少勿多。

卡片六段：R 手法出处 / I 技法骨架 / A1 书中案例 / A2 适用场景（何时绑这张卡）/
E 执行步骤（可判断完成标准的动作）/ B 边界（何时失效 + 置信度声明）。

知识蒸馏模式（无样章）必须在 B 段标注「基于模型知识储备，非逐字文本核对」。
宁可产 2 张过硬的卡，不产 5 张注水的卡。
"#;

// ── 蒸馏主命令 ──

/// 蒸馏一本书为写作技能卡组。
///
/// - `title`：书名（上传文件时可留空，自动取文件名）
/// - `author`：作者（可选，帮助模型定位作品）
/// - `file_path`：书籍文件路径（可选，txt/md/epub/pdf 等，优先于 sample_text）
/// - `sample_text`：样章文本（可选，提供则为「样章增强」高精度模式）
/// - `dimensions`：勾选的维度 slug 列表（缺省 = 全部 5 维）
/// - `model`：蒸馏用模型（缺省 = 第一个有 API Key 的可用模型）
// Tauri 命令参数平铺为 IPC 字段，无法收敛进结构体，故豁免参数数量检查
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn distill_book(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    title: String,
    author: Option<String>,
    file_path: Option<String>,
    sample_text: Option<String>,
    dimensions: Option<Vec<String>>,
    model: Option<String>,
) -> Result<BookPackage, String> {
    // 独占蒸馏控制面：任务全程占位，结束（成功或失败）时自动发终态事件
    let mut guard = DistillRunGuard::begin(&app_handle, &state, "book", "book-distill-phase")?;
    let author = author.unwrap_or_default().trim().to_string();
    // 书籍来源优先级：上传文件（全书抽样）> 手动粘贴样章 > 纯知识蒸馏
    let file_path = file_path.unwrap_or_default().trim().to_string();
    let (title, sample, sample_mode): (String, String, String) = if file_path.is_empty() {
        let t = title.trim().to_string();
        if t.is_empty() {
            return Err("书名不能为空".to_string());
        }
        // 样章截断保护（防超长粘贴爆上下文）
        let raw = sample_text.unwrap_or_default();
        let trimmed = raw.trim();
        let s: String = if trimmed.chars().count() > 20_000 {
            trimmed.chars().take(20_000).collect()
        } else {
            trimmed.to_string()
        };
        let mode = if s.is_empty() {
            "知识蒸馏"
        } else {
            "样章增强"
        };
        (t, s, mode.to_string())
    } else {
        // epub 解压 / pdf 提取可能耗时，放到阻塞线程池
        let fp = file_path.clone();
        let book = tokio::task::spawn_blocking(move || book_file::read_book_file(&fp))
            .await
            .map_err(|e| format!("解析书籍文件任务失败: {e}"))??;
        let total = book.full_text.chars().count();
        let manual = title.trim();
        let t = if manual.is_empty() {
            book.title_guess.clone()
        } else {
            manual.to_string()
        };
        (
            t,
            book_file::sample_text(&book.full_text),
            format!("全书抽样（全文共 {total} 字，取开头+中段+结尾代表样本）"),
        )
    };

    // 维度过滤：非法 slug 直接忽略；全非法则报错
    let dims: Vec<&'static (&'static str, &'static str, &[&str], &'static str)> = match &dimensions
    {
        Some(list) if !list.is_empty() => DIMENSIONS
            .iter()
            .filter(|(slug, _, _, _)| list.iter().any(|d| d == slug))
            .collect(),
        _ => DIMENSIONS.iter().collect(),
    };
    if dims.is_empty() {
        return Err("未选择任何有效的蒸馏维度".to_string());
    }

    let (model_id, provider_id, api_key, api_base) = resolve_model_and_auth(&state, model)?;
    let auth = lh::ProviderAuth {
        provider_id: &provider_id,
        api_key: &api_key,
        api_base: &api_base,
    };
    let (methodology, skill_source) = load_book_methodology(&state);
    emit_phase(
        &app_handle,
        &state,
        "加载蒸馏技能",
        "done",
        &skill_source,
        "",
    )
    .ok();

    let book_ref = if author.is_empty() {
        format!("《{title}》")
    } else {
        format!("《{title}》（{author}）")
    };
    let dim_list = dims
        .iter()
        .map(|(slug, label, _, focus)| format!("- {slug}（{label}）：{focus}"))
        .collect::<Vec<_>>()
        .join("\n");
    let sample_block = if sample.is_empty() {
        String::new()
    } else if sample_mode.starts_with("全书抽样") {
        format!(
            "\n\n═══════ 全书文本样本（一手素材，权重最高；{sample_mode}）═══════\n{sample}\n═══════ 全文样本结束 ═══════"
        )
    } else {
        format!(
            "\n\n═══════ 用户提供的样章文本（一手素材，权重最高）═══════\n{sample}\n═══════ 样章结束 ═══════"
        )
    };

    // ── 阶段 0：整书理解 ──
    emit_phase(
        &app_handle,
        &state,
        "整书理解",
        "running",
        &format!("正在以「{sample_mode}」模式理解{book_ref}的写作方法..."),
        "",
    )
    .ok();
    let overview_prompt = format!(
        "你是「PenSoul · 书籍写作蒸馏术」的执行者。上述方法论是你的工作手册。\n\
         现在执行阶段 0（整书理解），对象为{book_ref}，模式：{sample_mode}。\n\n\
         注意：你无法联网检索，请基于你的作品知识储备工作，对不确定的信息明确标注置信度；\n\
         若你对这本书了解不足，直接说明并降低产出野心，禁止编造。{sample_block}\n\n\
         严格按以下四部分输出：\n\
         ## 写法骨架\n这本书在写作方法上最值得学的 3-5 个支柱（每个一句话点破，附置信度）\n\
         ## 写法光谱\n句法长短、视角选择、叙事距离、信息控制、节奏取向的总体定位（3-6 条）\n\
         ## 批判\n什么不该学：时代局限、类型局限、作者已知的写作短板（2-4 条）\n\
         ## 维度预判\n针对以下勾选维度，各给一句话提取方向预判：\n{dim_list}\n\n\
         硬性要求：只谈「怎么写」，不复述情节内容；信息不足处直接标注「了解有限」。\n\
         直接以「## 写法骨架」开头输出，不要标题、不要前言。用中文输出。",
    );
    let overview =
        lh::call_llm(&auth, &model_id, &methodology, &overview_prompt, 0.7, 16384).await?;
    let overview = match overview.find("## 写法骨架") {
        Some(idx) => overview[idx..].to_string(),
        None => overview,
    };
    emit_phase(
        &app_handle,
        &state,
        "整书理解",
        "done",
        "整书理解完成",
        &overview,
    )
    .ok();

    // ── 阶段 1：维度提取 + 三重验证 ──
    emit_phase(
        &app_handle,
        &state,
        "技法提取与验证",
        "running",
        &format!("正在从{}个维度提取候选技法并做三重验证...", dims.len()),
        "",
    )
    .ok();
    let extract_prompt = format!(
        "你是「PenSoul · 书籍写作蒸馏术」的提取执行者。基于阶段 0 的整书理解，\n\
         对{book_ref}执行阶段 1（维度提取 + 三重验证）。\n\n\
         ═══════ 阶段 0 整书理解 ═══════\n{overview}\n═══════ 理解结束 ═══════\n\
         {sample_block}\n\n\
         请对以下每个勾选维度提取 3-5 个候选技法（怎么写的方法，不是写了什么）：\n{dim_list}\n\n\
         每个候选技法必须做三重验证并写出结论：\n\
         - V1 跨章复现：书中至少 2 个独立位置有体现？（列出位置）\n\
         - V2 生成力：能用它指导一个书里没写过的新场景？\n\
         - V3 独特性：不是所有作者都会这样做的常识？\n\
         判定：三重全过=「成立」，过 1-2 重=「降级为附注」，0 重=「丢弃」。\n\n\
         输出格式（严格遵守）：每个维度一节，以「## <slug>」开头（slug 原样使用上面的英文标识），\n\
         节内每个候选：\n\
         ### <技法名>\n\
         - 是什么：<一两句话>\n\
         - 验证：V1 <结论+位置> / V2 <结论> / V3 <结论> → <判定>\n\
         直接以第一个「## 」开头输出，不要前言。用中文输出。",
    );
    let extraction =
        lh::call_llm(&auth, &model_id, &methodology, &extract_prompt, 0.7, 16384).await?;
    let extraction = match extraction.find("## ") {
        Some(idx) => extraction[idx..].to_string(),
        None => extraction,
    };
    emit_phase(
        &app_handle,
        &state,
        "技法提取与验证",
        "done",
        "技法提取与验证完成",
        &extraction,
    )
    .ok();

    // ── 阶段 2：逐维构卡 ──
    let safe_title: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            _ => c,
        })
        .collect();
    let package = format!("{safe_title}-book");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let cards_base = writing_cards_base_dir(&state).join(&package);

    let mut cards: Vec<BookCardInfo> = Vec::new();
    for (slug, label, stages, _focus) in &dims {
        let phase_name = format!("构建{label}卡");
        emit_phase(
            &app_handle,
            &state,
            &phase_name,
            "running",
            "正在按 RIA++ 六段构造技能卡...",
            "",
        )
        .ok();

        let dim_section = {
            let s = extract_section_any(&extraction, &[slug, &format!("{slug} {label}"), label]);
            if s.is_empty() {
                extraction.clone() // 切片失败则喂全文，让模型自己定位
            } else {
                s
            }
        };
        let stages_str = stages
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let card_name = format!("{package}-{slug}");
        let build_prompt = format!(
            "你是「PenSoul · 书籍写作蒸馏术」的构卡者。基于以下素材，为{book_ref}构建\n\
             「{label}」（{slug}）维度的写作技能卡。{sample_block}\n\n\
             ═══════ 整书理解 ═══════\n{overview}\n\n\
             ═══════ 本维度提取与验证结果 ═══════\n{dim_section}\n═══════ 素材结束 ═══════\n\n\
             构卡要求：\n\
             1. 只保留判定为「成立」的技法（1-3 个）；若全部为附注/丢弃，挑最强的一个并标注局限\n\
             2. 严格按 RIA++ 六段输出：\n\
                ## R · 手法出处（典型位置；知识蒸馏模式标「凭作品整体印象」）\n\
                ## I · 技法骨架（自己的话重写，5-15 行，没读过原书的人能看懂）\n\
                ## A1 · 书中案例（1-3 个：什么写作问题 → 怎么用 → 效果）\n\
                ## A2 · 适用场景（3-5 个可识别场景 + 何时不绑这张卡）\n\
                ## E · 执行步骤（1-2-3 步骤，每步有可判断的完成标准，禁止态度口号）\n\
                ## B · 边界（什么题材/章节不该用 + 作者盲点 + 本卡为{sample_mode}模式的置信度声明）\n\
             3. 红线：不写内容梗概、不抄书摘、单卡正文 6000 字以内、E 段必须是可执行动作\n\
             4. 今天是 {today}，在 B 段末尾注明「蒸馏时间：{today}」\n\n\
             输出格式（严格遵守）：\n\
             - 用 ===SKILL_MD_BEGIN=== 和 ===SKILL_MD_END=== 包裹全部内容，前后不要任何解释\n\
             - 文件以 ---\\nname: {card_name}\\ndescription: <何时绑这张卡+何时不绑，≤200字>\\n\\\n\
               source_book: {book_ref}\\ndimension: {slug}\\napplicable_stages: [{stages_str}]\\n--- 开头\n\
             - 正文标题：# {book_ref} · {label}技法卡，然后依次六段\n\
             - 用中文输出",
        );
        let raw = lh::call_llm(&auth, &model_id, &methodology, &build_prompt, 0.7, 16384).await?;
        let Some(skill_raw) = extract_skill_md(&raw) else {
            emit_phase(
                &app_handle,
                &state,
                &phase_name,
                "error",
                "LLM 输出中未找到 SKILL.md 标记，跳过本卡",
                "",
            )
            .ok();
            continue;
        };
        let skill_md = normalize_frontmatter_name(&skill_raw, &card_name);
        // 弱校验：六段标记大致齐全（R/I/A1/A2/E/B 各段标题关键词）
        let missing: Vec<&str> = ["## R", "## I", "## A1", "## A2", "## E", "## B"]
            .iter()
            .filter(|m| !skill_md.contains(*m))
            .cloned()
            .collect();
        if missing.len() > 2 {
            emit_phase(
                &app_handle,
                &state,
                &phase_name,
                "error",
                &format!("六段结构缺失过多（缺 {}），跳过本卡", missing.join("、")),
                "",
            )
            .ok();
            continue;
        }

        let dim_dir = cards_base.join(slug);
        std::fs::create_dir_all(&dim_dir).map_err(|e| format!("创建技能卡目录失败: {e}"))?;
        let skill_file = dim_dir.join("SKILL.md");
        std::fs::write(&skill_file, &skill_md).map_err(|e| format!("写入 SKILL.md 失败: {e}"))?;

        let (fm, _body) = parse_skill_md(&skill_md);
        cards.push(BookCardInfo {
            dimension: slug.to_string(),
            dimension_label: label.to_string(),
            name: card_name,
            description: fm.get("description").cloned().unwrap_or_default(),
            skill_path: skill_file.to_string_lossy().to_string(),
            applicable_stages: stages.iter().map(|s| s.to_string()).collect(),
        });
        emit_phase(
            &app_handle,
            &state,
            &phase_name,
            "done",
            &format!(
                "{label}卡构建完成{}",
                if missing.is_empty() {
                    String::new()
                } else {
                    format!("（缺 {} 段标题，已保留待人工审校）", missing.join("、"))
                }
            ),
            "",
        )
        .ok();
    }

    if cards.is_empty() {
        return Err("所有维度的技能卡构建均失败，请重试（可减少勾选维度）".to_string());
    }

    // ── 阶段 3：交付落盘 ──
    let research_dir = cards_base.join("references").join("research");
    std::fs::create_dir_all(&research_dir).map_err(|e| format!("创建存档目录失败: {e}"))?;
    std::fs::write(
        research_dir.join("00-overview.md"),
        format!("# {book_ref} 整书理解\n\n> 模式：{sample_mode} · 时间：{today}\n\n{overview}"),
    )
    .map_err(|e| format!("写入整书理解存档失败: {e}"))?;
    std::fs::write(
        research_dir.join("01-extraction.md"),
        format!(
            "# {book_ref} 技法提取与验证\n\n> 模式：{sample_mode} · 时间：{today}\n\n{extraction}"
        ),
    )
    .map_err(|e| format!("写入提取存档失败: {e}"))?;
    std::fs::write(
        cards_base.join("OVERVIEW.md"),
        format!("# {book_ref} 写作方法整书理解\n\n{overview}"),
    )
    .map_err(|e| format!("写入 OVERVIEW.md 失败: {e}"))?;
    // 包元数据（list 命令的数据源，程序生成不占 LLM 调用）
    let package_json = serde_json::json!({
        "title": title,
        "author": author,
        "created_at": today,
        "sample_mode": sample_mode,
        "dimensions": cards.iter().map(|c| c.dimension.clone()).collect::<Vec<_>>(),
    });
    std::fs::write(
        cards_base.join("package.json"),
        serde_json::to_string_pretty(&package_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写入 package.json 失败: {e}"))?;
    // INDEX.md：书目 + 各卡一句话
    let index_lines = cards
        .iter()
        .map(|c| {
            format!(
                "- **{}**（`{}/SKILL.md`）：{}",
                c.dimension_label, c.dimension, c.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        cards_base.join("INDEX.md"),
        format!(
            "# {book_ref} 写作技能包\n\n> 蒸馏时间：{today} · 模式：{sample_mode} · 共 {} 张卡\n\n{index_lines}\n\n适用环节说明：`outline_expand` 细纲展开 / `chapter_writing` 章节写作 / `review` 一致性审查。\n",
            cards.len()
        ),
    )
    .map_err(|e| format!("写入 INDEX.md 失败: {e}"))?;

    let pkg = BookPackage {
        package: package.clone(),
        title: title.clone(),
        author,
        created_at: today,
        cards,
    };
    emit_phase(
        &app_handle,
        &state,
        "交付",
        "done",
        &format!(
            "蒸馏完成！已生成 {} 张技能卡到 WritingCard/{}",
            pkg.cards.len(),
            package
        ),
        "",
    )
    .ok();
    guard.finish(format!("蒸馏完成！已生成 {} 张技能卡", pkg.cards.len()));
    Ok(pkg)
}

// ── 技能包管理 ──

/// 扫描 WritingCard/ 列出全部技能包
#[tauri::command]
pub async fn list_book_packages(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BookPackage>, String> {
    // 支持两类包：-book（书籍蒸馏）与 -methodology（方法论蒸馏）
    let is_package_dir = |name: &str| name.ends_with("-book") || name.ends_with("-methodology");
    let base = writing_cards_base_dir(&state);
    let mut out = Vec::new();
    if !base.exists() {
        return Ok(out);
    }
    let entries =
        std::fs::read_dir(&base).map_err(|e| format!("读取 WritingCard 目录失败: {e}"))?;
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !is_package_dir(dir_name) {
            continue;
        }
        // 包元数据：优先 package.json，缺失时从目录名推断
        let (mut title, mut author, mut created_at) = (
            dir_name.trim_end_matches("-book").to_string(),
            String::new(),
            String::new(),
        );
        if let Ok(meta) = std::fs::read_to_string(dir.join("package.json"))
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&meta)
        {
            title = v
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or(&title)
                .to_string();
            author = v
                .get("author")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            created_at = v
                .get("created_at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
        }
        let mut cards = Vec::new();
        // 通用扫描：包内每个 <dimension>/SKILL.md 一张卡，维度与适用环节以 frontmatter 为准
        let mut sub_dirs: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| format!("读取技能包目录失败: {e}"))?
            .flatten()
            .filter(|e| e.path().is_dir())
            .collect();
        sub_dirs.sort_by_key(|e| e.file_name());
        for sub in sub_dirs {
            let skill_file = sub.path().join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&skill_file) else {
                continue;
            };
            let (fm, _body) = parse_skill_md(&content);
            let slug = fm
                .get("dimension")
                .cloned()
                .unwrap_or_else(|| sub.file_name().to_str().unwrap_or("").to_string());
            let label = DIMENSIONS
                .iter()
                .find(|(s, _, _, _)| *s == slug)
                .map(|(_, l, _, _)| l.to_string())
                .unwrap_or_else(|| slug.clone());
            // frontmatter 里 applicable_stages 形如 ["chapter_writing", "review"]
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
                    DIMENSIONS
                        .iter()
                        .find(|(s, _, _, _)| *s == slug)
                        .map(|(_, _, st, _)| st.iter().map(|s| s.to_string()).collect())
                        .unwrap_or_default()
                });
            cards.push(BookCardInfo {
                dimension: slug.clone(),
                dimension_label: label,
                name: fm
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| format!("{dir_name}-{slug}")),
                description: fm.get("description").cloned().unwrap_or_default(),
                skill_path: skill_file.to_string_lossy().to_string(),
                applicable_stages: stages,
            });
        }
        out.push(BookPackage {
            package: dir_name.to_string(),
            title,
            author,
            created_at,
            cards,
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// 删除整个技能包目录（不可逆）。
///
/// # 安全校验
/// 1. 包名必须是单个路径组件（防 `..` / 分隔符注入）且以 `-book` 结尾；
/// 2. canonicalize 后必须位于 WritingCard/ 根目录内（防符号链接逃逸）。
#[tauri::command]
pub async fn delete_book_package(
    state: tauri::State<'_, AppState>,
    package: String,
) -> Result<(), String> {
    if package.is_empty()
        || package.contains('/')
        || package.contains('\\')
        || package.contains("..")
        || !(package.ends_with("-book") || package.ends_with("-methodology"))
    {
        return Err("非法的技能包名".to_string());
    }
    let base = writing_cards_base_dir(&state);
    let target = base.join(&package);
    if !target.exists() {
        return Ok(()); // 已不存在，视为成功
    }
    let canonical = target
        .canonicalize()
        .map_err(|e| format!("解析路径失败: {e}"))?;
    let base_canonical = base
        .canonicalize()
        .map_err(|e| format!("解析根目录失败: {e}"))?;
    if !canonical.starts_with(&base_canonical) {
        return Err("目标不在 WritingCard 根目录内，拒绝删除".to_string());
    }
    std::fs::remove_dir_all(&canonical).map_err(|e| format!("删除技能包失败: {e}"))?;
    Ok(())
}

// ── 技能卡注入（供管线 / 细纲展开使用）──

/// 读取写作技能卡内容并拼接为 prompt 注入块。
///
/// 安全约束：文件必须名为 SKILL.md，且 canonicalize 后位于 WritingCard/ 根目录内。
/// 最多注入 5 张卡；单卡 8000 字符截断，总量 30000 字符封顶（保护上下文窗口）。
pub(crate) fn load_writing_cards(state: &AppState, paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let base = writing_cards_base_dir(state);
    let base_canonical = base.canonicalize().unwrap_or_else(|_| base.clone());
    let mut out = String::new();
    for p in paths.iter().take(5) {
        // 兼容两种写法：绝对路径（蒸馏产物自带）与相对 WritingCard/ 的路径（内置模板绑定）
        let path = std::path::Path::new(p);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        };
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(&base_canonical) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&canonical) else {
            continue;
        };
        const MAX: usize = 8_000;
        let capped = if content.chars().count() > MAX {
            format!(
                "{}…（卡内容过长已截断）",
                content.chars().take(MAX).collect::<String>()
            )
        } else {
            content
        };
        // 卡标识：<包名>/<维度slug>
        let label = canonical
            .parent()
            .map(|d| {
                let dim = d.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let pkg = d
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("?");
                format!("{pkg}/{dim}")
            })
            .unwrap_or_else(|| "未知卡".to_string());
        out.push_str(&format!("── 技能卡「{label}」──\n{capped}\n\n"));
        if out.chars().count() > 30_000 {
            break;
        }
    }
    out.trim().to_string()
}

/// 计算 WritingCard/ 根目录（与 Experts/ 同级：base_dir 的上一级）
pub(crate) fn writing_cards_base_dir(state: &AppState) -> std::path::PathBuf {
    state
        .base_dir
        .parent()
        .map(|p| p.join("WritingCard"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("WritingCard")
        })
}

// ── 内部辅助 ──

/// 解析蒸馏模型与供应商：指定模型优先，否则取第一个「供应商有 Key」的可用模型。
/// 返回 (model_id, provider_id, api_key, api_base)，由调用方组装 ProviderAuth。
pub(crate) fn resolve_model_and_auth(
    state: &AppState,
    model: Option<String>,
) -> Result<(String, String, String, String), String> {
    lh::ensure_api_keys_loaded(state);
    let models = lh::load_models(state);
    let providers = lh::load_providers(state);
    let m2p = lh::build_model_to_provider(&models);
    let bases = lh::build_provider_api_bases(&providers);
    let keys = state.api_keys.read().clone();

    let model_id = match model.filter(|m| !m.trim().is_empty()) {
        Some(m) => m,
        None => models
            .iter()
            .find_map(|m| {
                let mid = m.get("model_id")?.as_str()?.to_string();
                let pid = m.get("provider_id")?.as_str()?;
                let available = m
                    .get("is_available")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                (available && keys.contains_key(pid)).then_some(mid)
            })
            .ok_or_else(|| {
                "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string()
            })?,
    };
    let (pid, key, base) = lh::resolve_provider(&model_id, &m2p, &bases, &keys)?;
    Ok((model_id, pid, key, base))
}

/// 加载 pensoul-skill-Books 技能内容作为蒸馏方法论；找不到时回退内置简版
fn load_book_methodology(state: &AppState) -> (String, String) {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            candidates.push(ancestor.join(SKILL_RELATIVE_PATH));
        }
    }
    if let Some(root) = writing_cards_base_dir(state)
        .parent()
        .map(|p| p.to_path_buf())
    {
        candidates.push(root.join(SKILL_RELATIVE_PATH));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(SKILL_RELATIVE_PATH));
    }
    for candidate in candidates {
        if candidate.exists()
            && let Ok(content) = std::fs::read_to_string(&candidate)
        {
            return (content, format!("已加载蒸馏技能: {}", candidate.display()));
        }
    }
    (
        FALLBACK_METHODOLOGY.to_string(),
        "未找到 skills/pensoul-skill-Books/SKILL.md，使用内置简版方法论".to_string(),
    )
}

/// 向 Tauri 前端发射蒸馏阶段事件（事件名独立于专家蒸馏，互不干扰）
fn emit_phase(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    phase: &str,
    status: &str,
    message: &str,
    detail: &str,
) -> Result<(), String> {
    let event = super::expert_distill::DistillPhaseEvent {
        phase: phase.to_string(),
        status: status.to_string(),
        message: message.to_string(),
        detail: detail.to_string(),
    };
    state.distills.record(&event);
    let _ = app_handle.emit("book-distill-phase", event);
    Ok(())
}
