// writing.rs — AI 辅助写作（生成初稿 / 续写）
// AI 产物为建议制：只返回文本给编辑器，不落盘；
// 用户保存时走 save_chapter_content 集成层（修订历史 + 一致性评分 + 事件发布）
//
// 2.0 改造（调研 F1/F3/F13/F14/F12/F15/F2）：
// - F1 写作上下文 = 静态骨架（大纲/前后章/世界观）+ 动态记忆检索（pensoul-memory 管线）
// - F2 静态骨架含「本章相关事件」：按事件而非仅摘要组织上下文
// - F3 硬约束快照注入系统提示词（约束引擎，生成时即遵守）
// - F13 正典 AestheticLayer 风格笔记注入（style_notes / pacing_notes）
// - F14 反 AI 味词表注入 + 生成结果消痕扫描（提示词级，无额外 LLM 调用）
// - F12/F15 用户指定叙事技巧（technique_ids）注入「本章写作技巧」

use axum::extract::{Form, State};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::llm::{build_llm_request, llm_client};
use crate::commands::techniques;
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::chapter::Chapter;
use pensoul_domain::constraint::ConstraintKind;
use pensoul_domain::entity::{EntityRef, EntityType};
use pensoul_domain::ontology::NovelOntology;
use pensoul_infra::llm::LlmMessage;
use pensoul_memory::types::{EditingMode, MemoryPacket, RetrievalContext, WritingIntent};

#[derive(Deserialize)]
pub struct GenerateParams {
    pub chapter_id: String,
    /// draft（生成初稿）/ continue（续写）
    pub mode: String,
    /// 续写时传入编辑器已有内容
    pub existing_content: Option<String>,
    /// 逗号分隔的叙事技巧 id（F12/F15，可选）
    pub technique_ids: Option<String>,
}

/// AI 生成章节初稿或续写（不写正典，由用户保存确认）
pub async fn generate(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<GenerateParams>,
) -> Result<String, ApiError> {
    let mode = params.mode.trim();
    if mode != "draft" && mode != "continue" {
        return Err(ApiError::bad_request(
            "mode 必须是 draft（初稿）或 continue（续写）",
        ));
    }
    let existing = params.existing_content.clone().unwrap_or_default();
    let technique_ids: Vec<String> = params
        .technique_ids
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(|part| part.trim().to_string())
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let result = generate_chapter(&state, &params.chapter_id, mode, &existing, &technique_ids).await?;
    serde_json::to_string(&result).map_err(|e| ApiError::internal(e.to_string()))
}

/// 生成核心（P5 批量写作复用）：返回 {content, model, memory_stats, constraints_applied, techniques_applied, anti_slop_warnings}
pub(crate) async fn generate_chapter(
    state: &Arc<RwLock<AppState>>,
    chapter_id: &str,
    mode: &str,
    existing_content: &str,
    technique_ids: &[String],
) -> Result<serde_json::Value, ApiError> {
    // 先校验章节存在并克隆上下文，再加载 LLM 配置
    // 静态骨架 + 动态记忆检索 + 硬约束快照都在读锁内同步完成
    let (
        chapter,
        context_json,
        memory_text,
        memory_stats,
        constraint_text,
        constraints,
        existing,
        base_dir,
        project_id,
    ) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let project_id = ontology.project_id.as_str().to_string();
        let chapter = ontology
            .chapters_in_order()
            .into_iter()
            .find(|c| c.chapter_id.to_string() == chapter_id)
            .cloned()
            .ok_or(ApiError::not_found("章节不存在"))?;

        // 静态骨架：大纲弧 / 前后章 / 世界观 / 角色 / 伏笔
        let context_json = writing_context(ontology, &chapter);

        // F1 动态记忆检索：意图识别 → 相关性评分（涉及实体参与加分）→ 预算分配 → 组装
        let involved_entities = detect_involved_entities(ontology, &chapter);
        // draft 与 continue 均为「新增内容」意图；审校/修改留待后续编辑模式接入
        let editing_mode = EditingMode::Drafting;
        let retrieval = RetrievalContext {
            current_chapter: chapter.chapter_no,
            cursor_position: None,
            editing_mode,
            involved_entities,
            intent: WritingIntent::NewContent,
        };
        let packet = state.memory.retrieve(&retrieval);
        let memory_text = format_memory_packet(&packet);
        let memory_stats = serde_json::json!({
            "entity_count": packet.entities.len(),
            "total_tokens": packet.total_tokens,
            "budget_total": packet.budget_used.total_tokens,
            "budget_entity": packet.budget_used.entity_tokens,
            "budget_temporal": packet.budget_used.temporal_tokens,
            "budget_emotional": packet.budget_used.emotional_tokens,
        });

        // F3 硬约束快照：注入提示词，让生成路径在约束边界内创作
        let (constraint_text, constraint_ids) = hard_constraints_snapshot(&state.constraints);

        let existing = existing_content.to_string();
        (
            chapter,
            context_json,
            memory_text,
            memory_stats,
            constraint_text,
            constraint_ids,
            existing,
            state.base_dir.clone(),
            project_id,
        )
    };

    // P0b：写作 Agent 按角色解析模型（未绑定回退全局默认）
    let provider =
        crate::commands::agent::resolve(&base_dir, crate::commands::agent::AgentRole::Writer)?;
    let client = llm_client(&provider);

    // F12/F15 技巧注入：用户指定技巧 → 生成指导段落 + 命中 id 列表
    let technique_block = techniques::guidance_block(technique_ids);
    let (hit_techniques, _unknown) = techniques::resolve(technique_ids);
    let techniques_applied: Vec<String> =
        hit_techniques.iter().map(|t| t.id.to_string()).collect();

    let anti_slop_rules = anti_slop_rules_text();
    // P3 风格配方注入：书籍蒸馏配方（如有）作为风格参考，受强度参数约束；配方为作品级（按项目隔离）
    let style_recipe_text = crate::commands::distill::load_style_recipe(&base_dir, &project_id)
        .map(|r| crate::commands::distill::recipe_injection_text(&r));
    let system = if mode == "draft" {
        draft_system_prompt(
            &context_json,
            &memory_text,
            &constraint_text,
            &anti_slop_rules,
            technique_block.as_deref(),
            style_recipe_text.as_deref(),
        )
    } else {
        continue_system_prompt(
            &context_json,
            &memory_text,
            &constraint_text,
            &anti_slop_rules,
            technique_block.as_deref(),
            style_recipe_text.as_deref(),
        )
    };
    let user_content = if mode == "draft" {
        format!(
            "请为第 {} 章《{}》写出完整初稿。\n章节摘要：{}",
            chapter.chapter_no,
            chapter.title,
            if chapter.summary.is_empty() {
                "（本章暂无摘要，请根据所属大纲脉络合理展开）".to_string()
            } else {
                chapter.summary.clone()
            }
        )
    } else {
        format!(
            "请续写第 {} 章《{}》。\n已有正文：\n{}",
            chapter.chapter_no,
            chapter.title,
            truncate_chars(&existing, 3000)
        )
    };

    let request = build_llm_request(
        &provider,
        vec![LlmMessage {
            role: "user".to_string(),
            content: user_content,
        }],
        system,
        false,
        provider.max_output_tokens.max(4096),
    );
    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("LLM 调用失败: {e}")))?;

    let content = response.content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::bad_request("LLM 返回了空正文，请重试"));
    }

    // F14 生成后消痕扫描：本地关键词检测，提示用户酌情调整（建议制，不强制）
    let slop_hits = anti_slop_scan(&content);

    Ok(serde_json::json!({
        "content": content,
        "model": response.model,
        "memory_stats": memory_stats,
        "constraints_applied": constraints,
        "techniques_applied": techniques_applied,
        "anti_slop_warnings": slop_hits,
    }))
}

// ---- 静态上下文构建（项目骨架）----

/// 组装写作骨架上下文：大纲弧、前后章、世界观、角色、伏笔、核心概念
pub(crate) fn writing_context(ontology: &NovelOntology, chapter: &Chapter) -> String {
    let arcs: Vec<_> = ontology
        .outline_arcs
        .iter()
        .filter(|a| chapter.chapter_no >= a.chapter_start && chapter.chapter_no <= a.chapter_end)
        .map(|a| {
            serde_json::json!({
                "title": a.title,
                "description": a.description,
                "chapters": format!("{}-{}", a.chapter_start, a.chapter_end),
            })
        })
        .collect();

    let ordered = ontology.chapters_in_order();
    let prev = ordered
        .iter().rfind(|c| c.chapter_no < chapter.chapter_no)
        .map(|c| {
            serde_json::json!({
                "chapter_no": c.chapter_no,
                "title": c.title,
                "summary": c.summary,
                "content_tail": truncate_chars(&c.content, 1200),
            })
        });
    let next = ordered
        .iter()
        .find(|c| c.chapter_no > chapter.chapter_no)
        .map(|c| {
            serde_json::json!({
                "chapter_no": c.chapter_no,
                "title": c.title,
                "summary": c.summary,
            })
        });

    // 500 万字级长篇不能只取“档案前 20 个角色/前 15 个地点”。
    // 优先纳入本章摘要、所属脉络与相邻章节中实际提到的实体，再按档案顺序补足。
    let haystack = chapter_entity_haystack(ontology, chapter);

    let character_json = |c: &pensoul_domain::Character| {
        serde_json::json!({
            "name": c.name,
            "personality": c.properties.personality,
            "wants": c.properties.wants,
            "fears": c.properties.fears,
            "secret": c.properties.secret,
            "backstory": truncate_chars(c.properties.backstory.as_deref().unwrap_or(""), 200),
        })
    };
    let mut characters: Vec<serde_json::Value> = ontology
        .characters
        .characters
        .iter()
        .filter(|c| !c.name.is_empty() && haystack.contains(c.name.as_str()))
        .map(|c| character_json(c))
        .collect();
    let more_characters: Vec<serde_json::Value> = ontology
        .characters
        .characters
        .iter()
        .filter(|c| !characters.iter().any(|j| j["name"].as_str() == Some(c.name.as_str())))
        .take(20usize.saturating_sub(characters.len()))
        .map(|c| character_json(c))
        .collect();
    characters.extend(more_characters);

    let foreshadow_json = |f: &pensoul_domain::Foreshadow| {
        serde_json::json!({
            "name": f.name,
            "description": f.description,
            "expected_payoff": f.expected_payoff,
            "planted_chapter": f.planted_chapter,
        })
    };
    let active_foreshadows: Vec<&pensoul_domain::Foreshadow> = ontology
        .narrative
        .foreshadows
        .iter()
        .filter(|f| {
            !matches!(
                f.status,
                pensoul_domain::ForeshadowStatus::Resolved
                    | pensoul_domain::ForeshadowStatus::Abandoned
            )
        })
        .collect();
    let mut foreshadows: Vec<serde_json::Value> = active_foreshadows
        .iter()
        .filter(|f| !f.name.is_empty() && haystack.contains(f.name.as_str()))
        .map(|f| foreshadow_json(f))
        .collect();
    let more_foreshadows: Vec<serde_json::Value> = active_foreshadows
        .iter()
        .filter(|f| !foreshadows.iter().any(|j| j["name"].as_str() == Some(f.name.as_str())))
        .take(10usize.saturating_sub(foreshadows.len()))
        .map(|f| foreshadow_json(f))
        .collect();
    foreshadows.extend(more_foreshadows);

    let setting_json = |s: &pensoul_domain::Setting| {
        serde_json::json!({
            "name": s.name,
            "category": s.category,
            "description": s.description,
            "rules": s.rules,
        })
    };
    let mut world_settings: Vec<serde_json::Value> = ontology
        .world
        .locations
        .iter()
        .filter(|s| !s.name.is_empty() && haystack.contains(s.name.as_str()))
        .map(|s| setting_json(s))
        .collect();
    let more_settings: Vec<serde_json::Value> = ontology
        .world
        .locations
        .iter()
        .filter(|s| !world_settings.iter().any(|j| j["name"].as_str() == Some(s.name.as_str())))
        .take(15usize.saturating_sub(world_settings.len()))
        .map(|s| setting_json(s))
        .collect();
    world_settings.extend(more_settings);

    let organization_json = |o: &pensoul_domain::Organization| {
        serde_json::json!({
            "name": o.name,
            "category": o.category,
            "description": truncate_chars(&o.description, 200),
        })
    };
    let mut organizations: Vec<serde_json::Value> = ontology
        .world
        .organizations
        .iter()
        .filter(|o| !o.name.is_empty() && haystack.contains(o.name.as_str()))
        .map(|o| organization_json(o))
        .collect();
    let more_organizations: Vec<serde_json::Value> = ontology
        .world
        .organizations
        .iter()
        .filter(|o| !organizations.iter().any(|j| j["name"].as_str() == Some(o.name.as_str())))
        .take(10usize.saturating_sub(organizations.len()))
        .map(|o| organization_json(o))
        .collect();
    organizations.extend(more_organizations);

    // F2 本章相关事件：时间线中已发生且最接近当前章的最近 3 个事件（含参与者）
    let mut happened: Vec<_> = ontology
        .world
        .timeline
        .iter()
        .filter(|e| e.chapter_id <= chapter.chapter_no)
        .collect();
    happened.sort_by_key(|e| e.chapter_id);
    let related_events: Vec<_> = happened
        .into_iter()
        .rev()
        .take(3)
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "chapter": e.chapter_id,
                "participants": e.participants.iter().map(|p| p.label.as_deref().unwrap_or(&p.entity_id)).collect::<Vec<_>>(),
                "description": truncate_chars(&e.description, 200),
            })
        })
        .collect();

    serde_json::json!({
        "core_concept": ontology.core_concept,
        "genre": ontology.settings.genre,
        "world_rules": ontology.world.rules.iter().take(20).collect::<Vec<_>>(),
        "world_settings": world_settings,
        "outline_arcs": arcs,
        "previous_chapter": prev,
        "next_chapter": next,
        "characters": characters,
        "organizations": organizations,
        "active_foreshadows": foreshadows,
        // F2 事件级上下文
        "recent_events": related_events,
        // F13 风格笔记（来自正典 AestheticLayer）
        "style_notes": ontology.aesthetic.style_notes,
        "pacing_notes": ontology.aesthetic.pacing_notes,
    })
    .to_string()
}

// ---- F1 动态记忆检索 ----

/// 组装“本章可能涉及哪些实体”的检索文本：
/// 当前章标题/细纲 + 所属大纲弧描述 + 相邻章节标题/细纲。
/// 静态上下文与动态记忆共用同一逻辑，避免两套相关性判断打架。
fn chapter_entity_haystack(ontology: &NovelOntology, chapter: &Chapter) -> String {
    let arc_text = ontology
        .outline_arcs
        .iter()
        .filter(|a| chapter.chapter_no >= a.chapter_start && chapter.chapter_no <= a.chapter_end)
        .map(|a| format!("{} {}", a.title, a.description))
        .collect::<Vec<_>>()
        .join("\n");

    let ordered = ontology.chapters_in_order();
    let neighbor_text = ordered
        .iter()
        .filter(|c| {
            (c.chapter_no - chapter.chapter_no).abs() <= 1 && c.chapter_id != chapter.chapter_id
        })
        .map(|c| format!("{} {}", c.title, c.summary))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{}\n{}\n{}\n{}",
        chapter.title, chapter.summary, arc_text, neighbor_text
    )
}

/// 从章节摘要 / 所属大纲弧 / 前后章摘要中推断本章涉及的实体（真实匹配，非 mock）
/// 角色名与地点名以子串形式出现在上述文本中即视为涉及，供记忆评分器加分。
fn detect_involved_entities(ontology: &NovelOntology, chapter: &Chapter) -> Vec<EntityRef> {
    let haystack = chapter_entity_haystack(ontology, chapter);
    let mut seen = HashSet::new();
    let mut refs = Vec::new();

    // 角色名匹配
    for c in &ontology.characters.characters {
        if c.name.is_empty() {
            continue;
        }
        if haystack.contains(c.name.as_str()) && seen.insert(c.id.to_string()) {
            refs.push(
                EntityRef::new(EntityType::Character, c.id.to_string()).with_label(c.name.clone()),
            );
        }
    }
    // 地点名匹配（注意：图谱中 Setting 的实体 ID 是 SettingId，不是名称）
    for s in &ontology.world.locations {
        if s.name.is_empty() {
            continue;
        }
        if haystack.contains(s.name.as_str()) && seen.insert(s.id.to_string()) {
            refs.push(
                EntityRef::new(EntityType::Setting, s.id.to_string()).with_label(s.name.clone()),
            );
        }
    }
    // 组织名匹配（P0 档案）
    for o in &ontology.world.organizations {
        if o.name.is_empty() {
            continue;
        }
        if haystack.contains(o.name.as_str()) && seen.insert(o.id.to_string()) {
            refs.push(
                EntityRef::new(EntityType::Organization, o.id.to_string())
                    .with_label(o.name.clone()),
            );
        }
    }
    // 事件名匹配
    for ev in &ontology.world.timeline {
        if ev.name.is_empty() {
            continue;
        }
        if haystack.contains(ev.name.as_str()) && seen.insert(ev.id.to_string()) {
            refs.push(
                EntityRef::new(EntityType::Event, ev.id.to_string()).with_label(ev.name.clone()),
            );
        }
    }

    refs
}

/// 把记忆包格式化为 LLM 可读的动态上下文（只取内容三段，不含预算调试信息）
pub(crate) fn format_memory_packet(packet: &MemoryPacket) -> String {
    let mut parts = Vec::new();

    if !packet.entities.is_empty() {
        let mut lines = vec!["## 动态记忆 · 相关实体（按相关性排序）".to_string()];
        for entity in &packet.entities {
            lines.push(format!(
                "- **{}** (相关度 {:.0}%): {}",
                entity
                    .entity
                    .label
                    .as_deref()
                    .unwrap_or(&entity.entity.entity_id),
                entity.relevance_score * 100.0,
                entity.summary
            ));
            if !entity.details.is_empty() {
                lines.push(format!("    {}", entity.details));
            }
        }
        parts.push(lines.join("\n"));
    }

    if !packet.temporal_context.is_empty() {
        parts.push(format!("## 时间上下文\n{}", packet.temporal_context));
    }

    if !packet.emotional_context.is_empty() {
        parts.push(format!("## 情感上下文\n{}", packet.emotional_context));
    }

    if parts.is_empty() {
        "## 动态记忆\n（当前章节未检索到相关实体，请以静态上下文为准）".to_string()
    } else {
        parts.join("\n\n")
    }
}

// ---- F3 硬约束快照 ----

/// 从约束引擎导出硬约束快照：提示词段落 + 约束 id 列表（供响应统计）
pub(crate) fn hard_constraints_snapshot(
    engine: &pensoul_constraints::ConstraintEngine,
) -> (String, Vec<String>) {
    let hard = engine.list_constraints(Some(ConstraintKind::Hard));
    if hard.is_empty() {
        return ("".to_string(), Vec::new());
    }
    let ids: Vec<String> = hard.iter().map(|c| c.id.to_string()).collect();
    let text = hard
        .iter()
        .map(|c| format!("- [{}] {}：{}", c.id, c.name, c.description))
        .collect::<Vec<_>>()
        .join("\n");
    (
        format!("## 硬约束（以下为作品级硬性边界，生成内容不得违反）\n{text}"),
        ids,
    )
}

// ---- F14 反 AI 味（提示词级）----

/// 中文文本中常见的高频 AI 味表达（软提示：检测并提示，不硬性禁用）
pub(crate) const ANTI_SLOP_WORDS: &[&str] = &[
    "仿佛", "宛如", "犹如", "如同", "画卷", "氤氲", "静谧", "沧桑", "呢喃", "涟漪", "深邃", "眸子",
    "眼底", "嘴角", "勾起", "一抹", "刹那", "转瞬", "定格", "低语",
];

/// 注入系统提示词的反 AI 味写作规范
pub(crate) fn anti_slop_rules_text() -> String {
    let words = ANTI_SLOP_WORDS.join("、");
    format!(
        "## 写作语言规范（反 AI 味）\n\
         1. 以下表达在 AI 生成文本中高频出现，请尽量避免（除非情节必需且自然）：{words}。\n\
         2. 优先用具体名词与动作推进叙事，少用空泛比喻与四字排比堆砌；\n\
         3. 对话要有角色差异，避免所有人说话一个腔调；\n\
         4. 情绪靠行为与细节展示（show, don't tell），不要直接贴标签式陈述。"
    )
}

/// 对生成结果做本地消痕扫描：命中词返回提示（建议制，不修改正文）
pub(crate) fn anti_slop_scan(text: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for word in ANTI_SLOP_WORDS {
        if text.contains(word) {
            hits.push((*word).to_string());
        }
    }
    hits
}

// ---- 系统提示词 ----

fn draft_system_prompt(
    context: &str,
    memory: &str,
    constraints: &str,
    anti_slop: &str,
    techniques: Option<&str>,
    style_recipe: Option<&str>,
) -> String {
    let technique_block = techniques
        .map(|t| format!("\n\n{t}"))
        .unwrap_or_default();
    let style_block = style_recipe
        .map(|r| format!("\n\n{r}"))
        .unwrap_or_default();
    format!(
        "你是 PenSoul 的执笔人，为一部 500 万字级长篇小说的指定章节写初稿。\n\
         要求：\n\
         1. 严格贴合章节摘要与所属大纲脉络，不得擅自改变故事走向；\n\
         2. 遵守世界观规则与已有设定，角色言行必须与人物卡一致；\n\
         3. 延续前文章节的内容与文风（若有前文），结尾为下一章留出自然接续空间；\n\
         4. 不要与活跃伏笔的设定矛盾；\n\
         5. 中文写作，长篇叙事风格，单章约 2000 字；\n\
         6. 只输出正文本身，不要章节标题、不要任何解释、不要 Markdown 标记；\n\
         7. 禁止元叙述：不要出现「本章」「故事开始」「从……开始」等自我指涉的语句；\n\
         8. 严格贴合「风格笔记」与「节奏笔记」（若有）；落实「本章写作技巧」（若指定）。\n\
         \n\
         项目骨架（JSON）：\n{context}\n\n\
         {memory}\n\n\
         {constraints}\n\n\
         {anti_slop}{technique_block}{style_block}"
    )
}

fn continue_system_prompt(
    context: &str,
    memory: &str,
    constraints: &str,
    anti_slop: &str,
    techniques: Option<&str>,
    style_recipe: Option<&str>,
) -> String {
    let technique_block = techniques
        .map(|t| format!("\n\n{t}"))
        .unwrap_or_default();
    let style_block = style_recipe
        .map(|r| format!("\n\n{r}"))
        .unwrap_or_default();
    format!(
        "你是 PenSoul 的执笔人，继续书写当前章节。\n\
         要求：\n\
         1. 从已有正文的自然断点继续，不重复已有内容；\n\
         2. 文风、视角、叙述节奏与已有正文保持一致；\n\
         3. 遵守上下文中的世界观规则、人物设定与大纲走向；\n\
         4. 单次续写约 1000 字，结尾自然，不要强行收束；\n\
         5. 只输出续写正文，不要任何解释；\n\
         6. 与已有正文的设定保持一致，不要推翻已有内容；\n\
         7. 贴合「风格笔记」与「本章写作技巧」（若有指定）。\n\
         \n\
         项目骨架（JSON）：\n{context}\n\n\
         {memory}\n\n\
         {constraints}\n\n\
         {anti_slop}{technique_block}{style_block}"
    )
}

/// 按字符截断，防止上下文超预算
pub(crate) fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut output: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        output.push_str("…（前文截断）");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_domain::entity::Character;
    use pensoul_domain::ontology::NovelOntology;

    /// 构造一个最小正典：一个角色 + 一个地点 + 一章
    fn sample_ontology() -> NovelOntology {
        let mut ontology = NovelOntology::new(pensoul_domain::id::ProjectId::default(), "测试项目");
        ontology.core_concept.high_concept = "少年复仇".to_string();
        ontology.settings.genre = "玄幻".to_string();

        let mut hero = Character::new("林默");
        hero.properties.occupation = Some("医师".to_string());
        hero.properties.wants = Some("查明真相".to_string());
        ontology.characters.characters.push(hero);

        ontology
            .world
            .locations
            .push(pensoul_domain::entity::Setting::new("长安城", "城池"));

        let chapter = pensoul_domain::chapter::Chapter::new(1, "初入长安");
        ontology.chapters.push(chapter);
        ontology
    }

    #[test]
    fn involved_entities_matches_chapter_text() {
        let ontology = sample_ontology();
        let chapter = ontology.chapters[0].clone();
        // 摘要里提到角色名与地点名
        let mut chapter = chapter;
        chapter.summary = "林默来到长安城，准备查明真相。".to_string();
        let refs = detect_involved_entities(&ontology, &chapter);

        assert!(
            refs.iter().any(|r| r.label.as_deref() == Some("林默")),
            "应识别出角色林默: {refs:?}"
        );
        assert!(
            refs.iter().any(|r| r.label.as_deref() == Some("长安城")),
            "应识别出地点长安城: {refs:?}"
        );
    }

    #[test]
    fn involved_entities_deduplicated() {
        let ontology = sample_ontology();
        let mut chapter = ontology.chapters[0].clone();
        chapter.summary = "林默在长安城。林默再次望向长安城。".to_string();
        let refs = detect_involved_entities(&ontology, &chapter);
        let ids: HashSet<&str> = refs.iter().map(|r| r.entity_id.as_str()).collect();
        assert_eq!(ids.len(), refs.len(), "实体引用不应重复");
    }

    #[test]
    fn anti_slop_scan_finds_hits() {
        let hits = anti_slop_scan("夜色静谧，她眸子深邃，嘴角勾起一抹苦笑。");
        assert!(hits.contains(&"静谧".to_string()));
        assert!(hits.contains(&"眸子".to_string()));
        assert!(hits.contains(&"深邃".to_string()));
        assert!(hits.contains(&"嘴角".to_string()));
    }

    #[test]
    fn anti_slop_scan_ignores_clean_text() {
        let hits = anti_slop_scan("他把药瓶放回架子上，转身出了门。");
        assert!(hits.is_empty());
    }

    #[test]
    fn memory_packet_formatted_without_debug_line() {
        let packet = MemoryPacket::default();
        let text = format_memory_packet(&packet);
        assert!(text.contains("动态记忆"));
        assert!(!text.contains("Token 使用"));
    }
}
