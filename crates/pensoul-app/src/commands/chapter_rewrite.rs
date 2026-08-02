//! 笔耕批注重写 —— 修改计划 → 按批注重写正文 → 沉淀写作经验
//!
//! 两步流程（避免「一次合成」遗漏批注）：
//! 修改计划 → 重写正文 → 落库（旧版进版本历史）→ 经验沉淀。
//! 第一步 AI 读原稿 + 全部待处理批注 + 创作上下文，逐条给出
//! accept / reject（附理由）/ merge（折中）决定；
//! 第二步按计划重写整章，遵守写作铁律（章节标记协议 + 反 AI 味规则）；
//! 落库时旧版进 revisions（可回滚）、批注状态流转、派生状态同步；
//! 经验沉淀把已采纳的批注归类进项目经验库，之后注入章节审查 prompt。

use crate::commands::book_distill::load_writing_cards;
use crate::commands::json_fix;
use crate::commands::llm_helper as lh;
use crate::llm_profile::LlmTask;
use crate::pipeline::context::ANTI_AI_RULES;
use crate::pipeline::runner::{
    resolve_project_workflow, resolve_stage_cards, resolve_stage_model,
};
use crate::pipeline::stages::{parse_writing_output, STAGE_WRITING};
use crate::state::AppState;
use pensoul_core::{Chapter, ChapterAnnotation, ChapterId, ChapterRevision, WritingLesson};
use serde::Deserialize;

/// 版本历史上限（超出丢弃最旧）
const MAX_REVISIONS: usize = 30;

/// 批注重写结果
#[derive(serde::Serialize)]
pub struct RewriteResult {
    pub new_version: i32,
    /// 已采纳（含折中）的批注 ID
    pub accepted: Vec<String>,
    /// 已拒绝的批注 ID
    pub rejected: Vec<String>,
    /// 计划未覆盖、保持待处理的批注 ID
    pub untouched: Vec<String>,
    /// 修改计划摘要（给作者看）
    pub plan_summary: String,
    /// 本次沉淀（新增/累计）的写作经验
    pub lessons: Vec<WritingLesson>,
}

/// 修改计划条目
#[derive(Debug, Deserialize)]
struct PlanItem {
    annotation_id: String,
    /// accept / reject / merge
    decision: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Default, Deserialize)]
struct PlanPayload {
    #[serde(default)]
    plan: Vec<PlanItem>,
    #[serde(default)]
    summary: String,
}

/// 经验条目（LLM 归类产物；scope 供编辑经验复用，批注路径缺省 chapter）
#[derive(Debug, Default, Deserialize)]
pub(crate) struct LessonItem {
    #[serde(default)]
    pub(crate) category: String,
    pub(crate) problem: String,
    #[serde(default)]
    pub(crate) fix: String,
    #[serde(default)]
    pub(crate) scope: String,
}

#[derive(Debug, Default, Deserialize)]
struct LessonsPayload {
    #[serde(default)]
    lessons: Vec<LessonItem>,
}

/// 按批注重写本章正文
#[tauri::command]
pub async fn rewrite_chapter_with_annotations(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    model: Option<String>,
    skill_cards: Option<Vec<String>>,
) -> Result<RewriteResult, String> {
    lh::ensure_api_keys_loaded(&state);
    let id = ChapterId::new(chapter_id);

    // ── 1. 快照章节 + 组装上下文 ──
    let (chapter, concept_brief, settings_brief, prev_tail, target_words, model_id, cards_block) = {
        let onto = state.ontology.read();
        let chapter = onto
            .get_chapter(&id)
            .cloned()
            .ok_or_else(|| format!("章节 {} 不存在", id))?;
        let c = &onto.core_concept;
        let concept_brief = format!(
            "高概念：{}；前提：{}；主角：{}；基调：{}；核心冲突：{}",
            c.high_concept, c.premise, c.protagonist_hint, c.tone, c.central_conflict
        );
        let s = &onto.settings;
        let settings_brief = format!(
            "类型：{}；目标总章数：{} 章；每章目标字数：{} 字",
            s.genre, s.target_chapters, s.chapter_target_words
        );
        // 衔接：本章之前最近 2 章的梗概
        let prev_tail: String = onto
            .chapters
            .iter()
            .filter(|ch| ch.chapter_no > 0 && ch.chapter_no < chapter.chapter_no && !ch.summary.is_empty())
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|ch| format!("第{}章《{}》：{}", ch.chapter_no, ch.title, ch.summary))
            .collect::<Vec<_>>()
            .join("\n");
        let target_words = if s.chapter_target_words > 0 {
            s.chapter_target_words as usize
        } else {
            3000
        };
        let template = resolve_project_workflow(&state);
        let model_id = resolve_stage_model(&state, template.as_ref(), model.as_deref(), STAGE_WRITING)
            .or_else(|| first_available_model(&state))
            .ok_or_else(|| "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string())?;
        let cards = resolve_stage_cards(&state, template.as_ref(), skill_cards.as_ref(), STAGE_WRITING);
        let cards_block = load_writing_cards(&state, &cards);
        (
            chapter,
            concept_brief,
            settings_brief,
            prev_tail,
            target_words,
            model_id,
            cards_block,
        )
    };

    let open_annos: Vec<&ChapterAnnotation> = chapter
        .annotations
        .iter()
        .filter(|a| a.status == "open")
        .collect();
    if open_annos.is_empty() {
        return Err("本章没有待处理的批注，先在正文上批注再重写".to_string());
    }

    // ── 2. 解析模型供应商 ──
    let models = lh::load_models(&state);
    let providers = lh::load_providers(&state);
    let m2p = lh::build_model_to_provider(&models);
    let bases = lh::build_provider_api_bases(&providers);
    let api_keys = state.api_keys.read().clone();
    let (provider_id, api_key, api_base) = lh::resolve_provider(&model_id, &m2p, &bases, &api_keys)?;
    let auth = lh::ProviderAuth {
        provider_id: &provider_id,
        api_key: &api_key,
        api_base: &api_base,
    };

    // ── 3. 修改计划（显式化处理每条批注）──
    let plan = build_plan(&auth, &model_id, &chapter, &concept_brief, &settings_brief, &prev_tail, &cards_block, &open_annos).await?;

    // ── 4. 按计划重写正文 ──
    let body_raw = rewrite_body(
        &auth,
        &model_id,
        &chapter,
        &concept_brief,
        &settings_brief,
        &prev_tail,
        &cards_block,
        &plan,
        target_words,
    )
    .await?;
    let new_content = parse_writing_output(&body_raw);
    if new_content.trim().is_empty() {
        return Err("重写未产出正文，请重试".to_string());
    }

    // ── 5. 经验沉淀（best-effort，失败不影响重写）──
    let accepted_annos: Vec<&ChapterAnnotation> = open_annos
        .iter()
        .filter(|a| plan_decision(plan.plan.iter().find(|p| p.annotation_id == a.annotation_id)).is_some())
        .copied()
        .collect();
    let example = format!("第 {} 章《{}》", chapter.chapter_no, chapter.title);
    let lesson_items = distill_lessons(&auth, &model_id, &accepted_annos, &plan.plan, &example).await.unwrap_or_default();

    // ── 6. 落库：快照旧版 → 新正文 → 批注状态流转 → 经验合并 → 派生状态同步 ──
    let (new_version, accepted_ids, rejected_ids, untouched_ids) = {
        let mut onto = state.ontology.write();
        let ch = onto
            .chapters
            .iter_mut()
            .find(|ch| ch.chapter_id == id)
            .ok_or_else(|| format!("章节 {} 不存在", id))?;

        // 旧版进版本历史（可回滚）
        ch.revisions.push(ChapterRevision {
            version: ch.version,
            content: ch.content.clone(),
            word_count: ch.word_count,
            created_at: now(),
            reason: "批注重写前快照".to_string(),
        });
        if ch.revisions.len() > MAX_REVISIONS {
            let excess = ch.revisions.len() - MAX_REVISIONS;
            ch.revisions.drain(..excess);
        }

        let new_version = ch.version + 1;
        let mut accepted_ids = Vec::new();
        let mut rejected_ids = Vec::new();
        let mut untouched_ids = Vec::new();
        for anno in ch.annotations.iter_mut() {
            if anno.status != "open" {
                continue;
            }
            match plan.plan.iter().find(|p| p.annotation_id == anno.annotation_id) {
                Some(p) if p.decision == "reject" => {
                    anno.status = "rejected".to_string();
                    anno.processed_in_version = new_version;
                    rejected_ids.push(anno.annotation_id.clone());
                }
                Some(_) => {
                    anno.status = "accepted".to_string();
                    anno.processed_in_version = new_version;
                    accepted_ids.push(anno.annotation_id.clone());
                }
                None => untouched_ids.push(anno.annotation_id.clone()),
            }
        }

        ch.content = new_content.clone();
        ch.version = new_version;
        ch.word_count = new_content.chars().count() as u32;
        ch.updated_at = now();

        // 合并经验到项目经验库
        let merged = merge_lessons(&mut onto.writing_lessons, lesson_items, &example);
        let _ = merged; // 已在 RewriteResult.lessons 中返回
        (new_version, accepted_ids, rejected_ids, untouched_ids)
    };

    crate::integration::on_chapter_saved(&state, &id);
    state.save().map_err(|e| format!("批注重写落盘失败: {e}"))?;

    Ok(RewriteResult {
        new_version,
        accepted: accepted_ids,
        rejected: rejected_ids,
        untouched: untouched_ids,
        plan_summary: plan.summary.clone(),
        lessons: {
            let onto = state.ontology.read();
            onto.writing_lessons.clone()
        },
    })
}

/// 列出章节版本历史
#[tauri::command]
pub async fn list_chapter_revisions(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
) -> Result<Vec<ChapterRevision>, String> {
    let onto = state.ontology.read();
    let id = ChapterId::new(chapter_id);
    onto.get_chapter(&id)
        .map(|c| c.revisions.clone())
        .ok_or_else(|| format!("章节 {} 不存在", id))
}

/// 回滚到指定版本：当前版进历史，目标版恢复为正文
#[tauri::command]
pub async fn rollback_chapter(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    target_version: i32,
) -> Result<i32, String> {
    let id = ChapterId::new(chapter_id);
    let (rollback_content, rollback_words) = {
        let onto = state.ontology.read();
        let ch = onto
            .get_chapter(&id)
            .ok_or_else(|| format!("章节 {} 不存在", id))?;
        let rev = ch
            .revisions
            .iter()
            .find(|r| r.version == target_version)
            .ok_or_else(|| format!("版本 {} 不在历史中", target_version))?;
        (rev.content.clone(), rev.word_count)
    };

    let new_version = {
        let mut onto = state.ontology.write();
        let ch = onto
            .chapters
            .iter_mut()
            .find(|ch| ch.chapter_id == id)
            .ok_or_else(|| format!("章节 {} 不存在", id))?;
        // 当前版进历史，保留回滚线索
        ch.revisions.push(ChapterRevision {
            version: ch.version,
            content: ch.content.clone(),
            word_count: ch.word_count,
            created_at: now(),
            reason: format!("回滚前快照（回滚到第 {target_version} 版）"),
        });
        if ch.revisions.len() > MAX_REVISIONS {
            let excess = ch.revisions.len() - MAX_REVISIONS;
            ch.revisions.drain(..excess);
        }
        let new_version = ch.version + 1;
        ch.content = rollback_content;
        ch.word_count = rollback_words;
        ch.version = new_version;
        ch.updated_at = now();
        new_version
    };

    crate::integration::on_chapter_saved(&state, &id);
    state.save().map_err(|e| format!("回滚落盘失败: {e}"))?;
    Ok(new_version)
}

/// 项目写作经验库
#[tauri::command]
pub async fn get_writing_lessons(state: tauri::State<'_, AppState>) -> Result<Vec<WritingLesson>, String> {
    let onto = state.ontology.read();
    Ok(onto.writing_lessons.clone())
}

/// 保存（编辑/删除）项目写作经验库
#[tauri::command]
pub async fn save_writing_lessons(
    state: tauri::State<'_, AppState>,
    lessons: Vec<WritingLesson>,
) -> Result<(), String> {
    {
        let mut onto = state.ontology.write();
        onto.writing_lessons = lessons;
    }
    state.save().map_err(|e| e.to_string())
}

// ── 内部实现 ──────────────────────────────────────────────────────────

/// 修改计划：AI 逐条决定批注处理方式
#[allow(clippy::too_many_arguments)]
async fn build_plan(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    chapter: &Chapter,
    concept_brief: &str,
    settings_brief: &str,
    prev_tail: &str,
    cards_block: &str,
    open_annos: &[&ChapterAnnotation],
) -> Result<PlanPayload, String> {
    let system = "你是资深网文编辑，负责对作者在正文上留下的批注做处理计划。\
        输出严格 JSON，不评论、不解释。";
    let user = format!(
        "【核心概念】\n{concept_brief}\n\n\
         【创作设定】\n{settings_brief}\n\n\
         {cards_section}\
         【前情衔接】\n{}\n\n\
         【待重写章节】第 {} 章《{}》（梗概：{}）\n\
         原正文：\n{}\n\n\
         【作者批注】\n{}\n\n\
         请逐条决定如何处理批注，输出：\n\
         {{\"summary\": \"给作者的修改计划摘要（200字以内：采纳了什么、拒绝什么、为什么）\",\n\
         \"plan\": [{{\"annotation_id\": \"批注ID（必须与输入一致）\", \"decision\": \"accept 或 reject 或 merge\", \
         \"reason\": \"一句话理由（拒绝时必填，说明为什么不改；merge 说明折中方案）\"}}]}}\n\
         要求：\n\
         1. 每条批注都要有决定，不得遗漏\n\
         2. decision：accept=完全采纳；reject=不改（必须给理由，如与设定冲突/会破坏节奏/批注本身有误）；\
         merge=部分采纳并折中（给折中方案）\n\
         3. 用 ===PLAN_BEGIN=== 与 ===PLAN_END=== 包裹纯 JSON，全部内容用中文",
        if prev_tail.trim().is_empty() {
            "（无前章信息）".to_string()
        } else {
            prev_tail.to_string()
        },
        chapter.chapter_no,
        chapter.title,
        chapter.summary,
        cap_chars(&chapter.content, 12000),
        render_annotations(open_annos),
        cards_section = if cards_block.is_empty() {
            String::new()
        } else {
            format!("【写作技法卡】\n{cards_block}\n\n")
        },
    );
    let raw = lh::call_llm_task(auth, model_id, system, &user, 0.2, 4096, LlmTask::Light).await?;
    let json_str = extract_block(&raw, "===PLAN_BEGIN===", "===PLAN_END===");
    serde_json::from_str::<PlanPayload>(json_str)
        .or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<PlanPayload>(v).ok())
                .ok_or(strict_err.to_string())
        })
        .map_err(|e| format!("修改计划解析失败: {e}"))
}

/// 按修改计划重写正文
#[allow(clippy::too_many_arguments)]
async fn rewrite_body(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    chapter: &Chapter,
    concept_brief: &str,
    settings_brief: &str,
    prev_tail: &str,
    cards_block: &str,
    plan: &PlanPayload,
    target_words: usize,
) -> Result<String, String> {
    let system = format!(
        "你是一位长篇小说作家，正在按作者的批注意见重写一章正文。\n\
         铁律：只输出章节正文本身——不输出标题、不输出批注复述、不输出解释或元信息；\n\
         严格承接前文情节与人物状态，不得与世界观设定矛盾；\n\
         输出协议：正文必须严格包裹在 ===CHAPTER_BEGIN=== 与 ===CHAPTER_END=== 两个标记之间，\
         标记之外不得出现任何内容。\n\n{ANTI_AI_RULES}"
    );
    let mut plan_text = String::new();
    for item in &plan.plan {
        plan_text.push_str(&format!(
            "- 批注 {}：{}（{}）\n",
            item.annotation_id,
            match item.decision.as_str() {
                "reject" => "不改",
                "merge" => "折中采纳",
                _ => "采纳",
            },
            item.reason
        ));
    }
    let user = format!(
        "【核心概念】\n{concept_brief}\n\n\
         【创作设定】\n{settings_brief}\n\n\
         {cards_section}\
         【前情衔接】\n{}\n\n\
         【本章信息】第 {} 章《{}》，目标约 {} 字。本章梗概：{}\n\n\
         【原正文（供修改，不是照抄）】\n{}\n\n\
         【修改计划（必须逐条落实）】\n{plan_text}\n\n\
         请按修改计划重写本章正文：采纳的批注落实到正文中，拒绝的保持原样，\
         折中的按折中方案改写。重写后正文约 {} 字，直接以正文第一段开始。",
        if prev_tail.trim().is_empty() {
            "（无前章信息）".to_string()
        } else {
            prev_tail.to_string()
        },
        chapter.chapter_no,
        chapter.title,
        target_words,
        chapter.summary,
        cap_chars(&chapter.content, 12000),
        target_words,
        cards_section = if cards_block.is_empty() {
            String::new()
        } else {
            format!("【写作技法卡】\n{cards_block}\n\n")
        },
    );
    lh::call_llm_task(
        auth,
        model_id,
        &system,
        &user,
        0.7,
        (target_words as u32 * 2 + 8192).clamp(16384, 32768),
        LlmTask::Deep,
    )
    .await
}

/// 把已采纳批注蒸馏为写作经验
async fn distill_lessons(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    accepted: &[&ChapterAnnotation],
    plan: &[PlanItem],
    example: &str,
) -> Result<Vec<LessonItem>, String> {
    if accepted.is_empty() {
        return Ok(Vec::new());
    }
    let system = "你是写作复盘教练，负责把作者的批注意见提炼为可复用的写作经验。\
        输出严格 JSON，不评论、不解释。";
    let user = format!(
        "以下是一章正文中被采纳/折中处理的批注意见（含处理决定）：\n{}\n\n\
         请把它们归类为写作经验，输出：\n\
         {{\"lessons\": [{{\"category\": \"措辞 或 节奏 或 对话 或 一致性 或 反AI味 或 结构 或 其他\", \
         \"problem\": \"具体问题（30字内，可复用的一句话教训）\", \
         \"fix\": \"改正方法（50字内，可直接执行的写法）\"}}]}}\n\
         要求：\n\
         1. 每条批注最多归纳为一条经验，合并高度相似的批注\n\
         2. 只归纳真实存在的写作问题，备注类批注不纳入\n\
         3. 用 ===LESSONS_BEGIN=== 与 ===LESSONS_END=== 包裹纯 JSON，全部内容用中文",
        accepted
            .iter()
            .map(|a| {
                let decision = plan
                    .iter()
                    .find(|p| p.annotation_id == a.annotation_id)
                    .map(|p| p.decision.clone())
                    .unwrap_or_default();
                let anchor = a
                    .anchor
                    .as_ref()
                    .map(|x| format!("（原文片段：…{}…）", cap_chars(&x.text, 40)))
                    .unwrap_or_default();
                format!("- [{}] {}{}（出自{example}）", decision, a.content, anchor)
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    let raw = lh::call_llm_task(auth, model_id, system, &user, 0.2, 4096, LlmTask::Light).await?;
    let json_str = extract_block(&raw, "===LESSONS_BEGIN===", "===LESSONS_END===");
    let payload: LessonsPayload = serde_json::from_str(json_str)
        .or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<LessonsPayload>(v).ok())
                .ok_or(strict_err.to_string())
        })
        .map_err(|e| format!("经验提炼解析失败: {e}"))?;
    Ok(payload.lessons)
}

/// 合并经验到项目库：同类（分类 + 问题近似）累计次数，否则新增
pub(crate) fn merge_lessons(
    existing: &mut Vec<WritingLesson>,
    new: Vec<LessonItem>,
    example: &str,
) -> Vec<WritingLesson> {
    let ts = now();
    let mut out = Vec::new();
    for item in new {
        let problem = item.problem.trim();
        if problem.is_empty() {
            continue;
        }
        let category = if item.category.trim().is_empty() {
            "其他".to_string()
        } else {
            item.category.trim().to_string()
        };
        let fix = item.fix.trim().to_string();
        if let Some(l) = existing.iter_mut().find(|l| {
            l.category == category
                && (l.problem == problem || l.problem.contains(problem) || problem.contains(&l.problem))
        }) {
            l.count += 1;
            if !item.scope.is_empty() {
                l.scope = item.scope.clone();
            }
            if !fix.is_empty() {
                l.fix = fix;
            }
            if l.example.is_empty() {
                l.example = example.to_string();
            }
            out.push(l.clone());
        } else {
            let l = WritingLesson {
                lesson_id: format!("lesson-{}", uuid::Uuid::new_v4().simple()),
                category,
                problem: problem.to_string(),
                fix,
                example: example.to_string(),
                count: 1,
                created_at: ts.clone(),
                scope: if item.scope.is_empty() {
                    "chapter".to_string()
                } else {
                    item.scope.clone()
                },
            };
            out.push(l.clone());
            existing.push(l);
        }
    }
    out
}

/// 渲染批注列表
fn render_annotations(annos: &[&ChapterAnnotation]) -> String {
    annos
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let anchor = a
                .anchor
                .as_ref()
                .map(|x| format!("（锚定：…{}…）", cap_chars(&x.text, 40)))
                .unwrap_or_default();
            format!(
                "{}. [{}] {}{}",
                i + 1,
                match a.kind.as_str() {
                    "issue" => "问题",
                    "suggestion" => "修改建议",
                    _ => "备注",
                },
                a.content,
                anchor
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取第一条批注的处理决定（None = 未覆盖）
fn plan_decision(item: Option<&PlanItem>) -> Option<String> {
    item.map(|p| p.decision.clone())
}

/// 第一个「供应商有 Key」的可用模型
fn first_available_model(state: &AppState) -> Option<String> {
    let models = lh::load_models(state);
    let keys = state.api_keys.read().clone();
    models.iter().find_map(|m| {
        let model_id = m.get("model_id")?.as_str()?.to_string();
        let provider_id = m.get("provider_id")?.as_str()?;
        keys.contains_key(provider_id).then_some(model_id)
    })
}

/// 按字符截断（不切坏 UTF-8）
fn cap_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max).collect();
    format!("{truncated}…")
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 提取标记之间的内容
fn extract_between<'a>(text: &'a str, begin: &str, end: &str) -> Option<&'a str> {
    let b = text.find(begin)? + begin.len();
    let e = text.rfind(end)?;
    if e <= b {
        return None;
    }
    Some(&text[b..e])
}

/// 从模型输出提取 JSON 块：优先标记包裹内容，剥围栏，再截最外层花括号
fn extract_block<'a>(text: &'a str, begin: &str, end: &str) -> &'a str {
    let inner = extract_between(text, begin, end).unwrap_or(text);
    let cleaned = inner
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let (Some(b), Some(e)) = (cleaned.find('{'), cleaned.rfind('}'))
        && e > b
    {
        return &cleaned[b..=e];
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_block_with_markers() {
        let text = "前言\n===PLAN_BEGIN===\n{\"summary\": \"x\"}\n===PLAN_END===\n后缀";
        assert_eq!(extract_block(text, "===PLAN_BEGIN===", "===PLAN_END==="), "{\"summary\": \"x\"}");
    }

    #[test]
    fn test_extract_block_without_markers() {
        let text = "好的：\n```json\n{\"plan\": []}\n```\n完。";
        assert_eq!(extract_block(text, "===PLAN_BEGIN===", "===PLAN_END==="), "{\"plan\": []}");
    }

    #[test]
    fn test_cap_chars() {
        assert_eq!(cap_chars("短文本", 10), "短文本");
        assert!(cap_chars("这是一段比较长的文本", 5).ends_with('…'));
    }

    #[test]
    fn test_merge_lessons_dedup_and_count() {
        let mut existing = Vec::new();
        let new = vec![
            LessonItem {
                category: "措辞".to_string(),
                problem: "重复使用「不禁」".to_string(),
                fix: "换成具体动作".to_string(),
                scope: String::new(),
            },
            LessonItem {
                category: "措辞".to_string(),
                problem: "重复使用「不禁」".to_string(),
                fix: "换成具体动作".to_string(),
                scope: String::new(),
            },
            LessonItem {
                category: "节奏".to_string(),
                problem: "开篇铺垫过长".to_string(),
                fix: "300字内出钩子".to_string(),
                scope: String::new(),
            },
        ];
        let out = merge_lessons(&mut existing, new, "第 1 章");
        assert_eq!(out.len(), 3); // 每条新经验都返回（含累计后的重复项）
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].count, 2);
        assert_eq!(existing[1].count, 1);
    }

    #[test]
    fn test_plan_decision_mapping() {
        assert_eq!(plan_decision(None), None);
        assert_eq!(
            plan_decision(Some(&PlanItem {
                annotation_id: "a1".to_string(),
                decision: "reject".to_string(),
                reason: String::new(),
            })),
            Some("reject".to_string())
        );
    }
}
