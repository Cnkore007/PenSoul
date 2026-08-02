//! 方法论蒸馏 IPC 命令 —— 把一段「写作方法论」切成可绑定工作流环节的写作技能卡。
//!
//! 产物约定（与书籍蒸馏一致，包后缀为 -methodology）：
//! - 目录：`WritingCard/<标题>-methodology/`
//! - 六维度卡：style / structure / character / tension / genre / review 各一张 SKILL.md
//! - 卡片六段：R 手法出处 / I 技法骨架 / A1 原文案例 / A2 适用场景 / E 执行步骤 / B 边界
//! - 蒸馏过程写入 `references/research/` 随产物自包含保存
use crate::state::AppState;
use tauri::Emitter;

use super::book_distill::{
    BookCardInfo, BookPackage, resolve_model_and_auth, writing_cards_base_dir,
};
use super::expert_distill::{
    DistillPhaseEvent, DistillRunGuard, extract_skill_md, normalize_frontmatter_name,
};
use super::experts::{extract_section_any, parse_skill_md};
use super::llm_helper as lh;

/// 技能文件在工作区内的相对路径
const SKILL_RELATIVE_PATH: &str = "skills/pensoul-skill-Methodology/SKILL.md";

/// 编译进二进制的完整蒸馏技能内容（发布版必然可用，不依赖运行时路径）。
/// 技能源文件随仓库分发，构建时用 include_str! 嵌入。
const EMBEDDED_SKILL_MD: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/pensoul-skill-Methodology/SKILL.md"
));

/// 方法论六维度：(slug, 中文名, 适用环节, 提取重点)
pub const METHODOLOGY_DIMENSIONS: [(&str, &str, &[&str], &str); 6] = [
    (
        "style",
        "文风规则",
        &["chapter_writing", "review"],
        "语言铁律、句式节奏、去套话与去 AI 味清单",
    ),
    (
        "structure",
        "结构与编排",
        &["outline_expand"],
        "章节/场景编排、节拍表、情绪曲线、断章钩子",
    ),
    (
        "character",
        "人物塑造",
        &["outline_expand", "chapter_writing"],
        "动机、对话、人物状态、金手指与限制",
    ),
    (
        "tension",
        "冲突与张力",
        &["outline_expand", "chapter_writing"],
        "冲突升级、爽点、事件冷却、断章",
    ),
    (
        "genre",
        "类型范式",
        &["outline_expand"],
        "题材惯例、读者期待管理、开篇节奏",
    ),
    (
        "review",
        "审查标准",
        &["review"],
        "评分维度、门禁标准、问题清单格式",
    ),
];

/// 蒸馏一段方法论为写作技能卡组。
///
/// - `title`：方法论的名称（如「猫神写作经验」）
/// - `methodology_text`：方法论文本（文章/经验贴/讲稿摘录，2 万字内）
/// - `dimensions`：勾选的维度 slug 列表（缺省 = 全部 6 维）
/// - `model`：蒸馏用模型（缺省 = 第一个有 API Key 的可用模型）
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn distill_methodology(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    title: String,
    methodology_text: String,
    dimensions: Option<Vec<String>>,
    model: Option<String>,
) -> Result<BookPackage, String> {
    // 独占蒸馏控制面：任务全程占位，结束（成功或失败）时自动发终态事件
    let mut guard = DistillRunGuard::begin(
        &app_handle,
        &state,
        "methodology",
        "methodology-distill-phase",
    )?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err("方法论名称不能为空".to_string());
    }
    // 文本截断保护（防超长粘贴爆上下文）
    let raw = methodology_text.trim().to_string();
    let text: String = if raw.chars().count() > 20_000 {
        raw.chars().take(20_000).collect()
    } else {
        raw
    };
    if text.is_empty() {
        return Err("方法论文本不能为空".to_string());
    }

    // 维度过滤：非法 slug 直接忽略；全非法则报错
    let dims: Vec<&'static (&'static str, &'static str, &[&str], &'static str)> = match &dimensions
    {
        Some(list) if !list.is_empty() => METHODOLOGY_DIMENSIONS
            .iter()
            .filter(|(slug, _, _, _)| list.iter().any(|d| d == slug))
            .collect(),
        _ => METHODOLOGY_DIMENSIONS.iter().collect(),
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
    let (methodology, skill_source) = load_methodology_skill(&state);
    emit_phase(
        &app_handle,
        &state,
        "加载蒸馏技能",
        "done",
        &skill_source,
        "",
    )
    .ok();

    let source_ref = format!("《{title}》方法论");
    let dim_list = dims
        .iter()
        .map(|(slug, label, _, focus)| format!("- {slug}（{label}）：{focus}"))
        .collect::<Vec<_>>()
        .join("\n");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // ── 阶段 0：方法论骨架 ──
    emit_phase(
        &app_handle,
        &state,
        "方法论骨架",
        "running",
        &format!("正在通读{source_ref}并提炼写作骨架..."),
        "",
    )
    .ok();
    let overview_prompt = format!(
        "你是「PenSoul · 方法论蒸馏术」的执行者。上述方法论是你的工作手册。\n\
         现在执行阶段 0（方法论骨架），对象为{source_ref}。\n\n\
         ═══════ 方法论文本 ═══════\n{text}\n═══════ 文本结束 ═══════\n\n\
         严格按以下四部分输出：\n\
         ## 方法骨架\n这套方法论最值得学的 3-5 个支柱（每个一句话点破，附置信度）\n\
         ## 方法光谱\n它强在哪里、适用于什么写作场景、整体取向（3-6 条）\n\
         ## 批判\n什么不该学：局限、适用边界、可能误导的点（2-4 条）\n\
         ## 维度预判\n针对以下勾选维度，各给一句话提取方向预判：\n{dim_list}\n\n\
         硬性要求：只提炼「怎么写」，不复述原文观点原文；信息不足处直接标注「原文未提及」。\n\
         直接以「## 方法骨架」开头输出，不要标题、不要前言。用中文输出。",
    );
    let overview =
        lh::call_llm(&auth, &model_id, &methodology, &overview_prompt, 0.7, 16384).await?;
    let overview = match overview.find("## 方法骨架") {
        Some(idx) => overview[idx..].to_string(),
        None => overview,
    };
    emit_phase(
        &app_handle,
        &state,
        "方法论骨架",
        "done",
        "方法论骨架完成",
        &overview,
    )
    .ok();

    // ── 阶段 1：维度提取 + 三重验证 ──
    emit_phase(
        &app_handle,
        &state,
        "技法提取与验证",
        "running",
        &format!("正在从 {} 个维度提取候选技法并做三重验证...", dims.len()),
        "",
    )
    .ok();
    let extract_prompt = format!(
        "你是「PenSoul · 方法论蒸馏术」的提取执行者。基于阶段 0 的方法论骨架，\n\
         对{source_ref}执行阶段 1（维度提取 + 三重验证）。\n\n\
         ═══════ 阶段 0 方法论骨架 ═══════\n{overview}\n═══════ 骨架结束 ═══════\n\
         ═══════ 方法论文本 ═══════\n{text}\n═══════ 文本结束 ═══════\n\n\
         请对以下每个勾选维度提取 3-5 个候选技法（可执行的方法，不是观点）：\n{dim_list}\n\n\
         每个候选技法必须做三重验证并写出结论：\n\
         - V1 跨方法复现：原文至少 2 个独立位置有体现？（列出位置）\n\
         - V2 生成力：能用它指导一段原文没写到的具体写作动作？\n\
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
    let package = format!("{safe_title}-methodology");
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
            "你是「PenSoul · 方法论蒸馏术」的构卡者。基于以下素材，为{source_ref}构建\n\
             「{label}」（{slug}）维度的写作技能卡。\n\n\
             ═══════ 方法论骨架 ═══════\n{overview}\n\n\
             ═══════ 本维度提取与验证结果 ═══════\n{dim_section}\n═══════ 素材结束 ═══════\n\n\
             构卡要求：\n\
             1. 只保留判定为「成立」的技法（1-3 个）；若全部为附注/丢弃，挑最强的一个并标注局限\n\
             2. 严格按 RIA++ 六段输出：\n\
                ## R · 手法出处（方法原文里这条技法的典型位置）\n\
                ## I · 技法骨架（自己的话重写 5-15 行，没读过原文的人能看懂）\n\
                ## A1 · 原文案例（1-3 个：原文怎么说的 → 怎么用 → 效果）\n\
                ## A2 · 适用场景（3-5 个可识别场景 + 何时不绑这张卡）\n\
                ## E · 执行步骤（1-2-3 步骤，每步有可判断的完成标准，禁止态度口号）\n\
                ## B · 边界（什么题材/章节不该用 + 来源局限 + 本卡为方法论蒸馏模式的置信度声明）\n\
             3. 红线：不写观点总结、不抄原文长段落、单卡正文 6000 字以内、E 段必须是可执行动作\n\
             4. 今天是 {today}，在 B 段末尾注明「蒸馏时间：{today}」\n\n\
             输出格式（严格遵守）：\n\
             - 用 ===SKILL_MD_BEGIN=== 和 ===SKILL_MD_END=== 包裹全部内容，前后不要任何解释\n\
             - 文件以 ---\\nname: {card_name}\\ndescription: <何时绑这张卡+何时不绑，≤200字>\\n\\\n\
               source: {source_ref}\\ndimension: {slug}\\napplicable_stages: [{stages_str}]\\n--- 开头\n\
             - 正文标题：# {source_ref} · {label}技法卡，然后依次六段\n\
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
        format!("# {source_ref} 方法论骨架\n\n> 模式：方法论蒸馏 · 时间：{today}\n\n{overview}"),
    )
    .map_err(|e| format!("写入骨架存档失败: {e}"))?;
    std::fs::write(
        research_dir.join("01-extraction.md"),
        format!(
            "# {source_ref} 技法提取与验证\n\n> 模式：方法论蒸馏 · 时间：{today}\n\n{extraction}"
        ),
    )
    .map_err(|e| format!("写入提取存档失败: {e}"))?;
    std::fs::write(
        cards_base.join("OVERVIEW.md"),
        format!("# {source_ref} 方法论骨架\n\n{overview}"),
    )
    .map_err(|e| format!("写入 OVERVIEW.md 失败: {e}"))?;
    let package_json = serde_json::json!({
        "title": title,
        "author": "方法论蒸馏",
        "created_at": today,
        "sample_mode": "方法论蒸馏",
        "dimensions": cards.iter().map(|c| c.dimension.clone()).collect::<Vec<_>>(),
    });
    std::fs::write(
        cards_base.join("package.json"),
        serde_json::to_string_pretty(&package_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写入 package.json 失败: {e}"))?;
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
            "# {source_ref} 写作技能包\n\n> 蒸馏时间：{today} · 模式：方法论蒸馏 · 共 {} 张卡\n\n{index_lines}\n\n适用环节说明：`outline_expand` 细纲展开 / `chapter_writing` 章节写作 / `review` 一致性审查。\n",
            cards.len()
        ),
    )
    .map_err(|e| format!("写入 INDEX.md 失败: {e}"))?;

    let pkg = BookPackage {
        package: package.clone(),
        title: title.clone(),
        author: "方法论蒸馏".to_string(),
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

/// 加载 pensoul-skill-Methodology 技能内容作为蒸馏方法论；找不到时回退内置简版
fn load_methodology_skill(state: &AppState) -> (String, String) {
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
        EMBEDDED_SKILL_MD.to_string(),
        "已加载内置蒸馏技能（编译进应用）".to_string(),
    )
}

/// 向 Tauri 前端发射方法论蒸馏阶段事件
fn emit_phase(
    app_handle: &tauri::AppHandle,
    state: &AppState,
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
    state.distills.record(&event);
    let _ = app_handle.emit("methodology-distill-phase", event);
    Ok(())
}
