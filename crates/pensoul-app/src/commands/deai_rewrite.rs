//! 去 AI 味重写 —— 保真账本 → 有界改写 → 两步回读 → 建议删除清单
//!
//! 对应「去 AI 味」方法论（MrGeDiao/shuorenhua）的落地：
//! - 保真合同：改写不新增事实、不删核心事实、不改责任主体、数字与修饰对象同留；
//! - bounded scope：只做句内清洗；整句空话不直接删，进「建议删除清单」交作者确认；
//!   不合并相邻句、不重排段落；
//! - 两步回读：保真回读（新增/丢失/主体/术语/断裂 5 项）+ 残留审计（开场/总结/narrator/
//!   空泛判断/句长过匀 5 项），发现问题由独立一轮修复，修复只动问题点。
use crate::commands::chapter_rewrite::{cap_chars, extract_block, first_available_model, now};
use crate::commands::json_fix;
use crate::commands::llm_helper as lh;
use crate::llm_profile::LlmTask;
use crate::pipeline::runner::{resolve_project_workflow, resolve_stage_cards, resolve_stage_model};
use crate::pipeline::stages::{STAGE_WRITING, parse_writing_output};
use crate::state::AppState;
use pensoul_core::{Chapter, ChapterId, ChapterRevision};
use regex::Regex;
use serde::Deserialize;

const DEAI_BEGIN: &str = "===DEAI_BEGIN===";
const DEAI_END: &str = "===DEAI_END===";
const LIST_BEGIN: &str = "===LIST_BEGIN===";
const LIST_END: &str = "===LIST_END===";
/// 版本历史上限（与批注重写一致）
const MAX_REVISIONS: usize = 30;

/// 建议删除清单条目（整句空话，交给作者确认后才删）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeletionItem {
    pub sentence: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize)]
struct DeletionPayload {
    #[serde(default)]
    deletions: Vec<DeletionItem>,
    #[serde(default)]
    summary: String,
}

/// 两步回读结果
#[derive(Debug, Default, Deserialize)]
struct RereadPayload {
    #[serde(default)]
    fidelity_issues: Vec<IssueItem>,
    #[serde(default)]
    residual_issues: Vec<IssueItem>,
}

#[derive(Debug, Default, Deserialize)]
struct IssueItem {
    #[serde(default)]
    item: String,
    #[serde(default)]
    detail: String,
}

/// 去 AI 味重写结果
#[derive(serde::Serialize)]
pub struct DeaiRewriteResult {
    pub new_version: i32,
    pub word_count: u32,
    pub original_word_count: u32,
    /// 建议删除清单（整句空话，未删，交作者确认）
    pub suggested_deletions: Vec<DeletionItem>,
    /// 保真回读发现的问题（已修复或由作者知悉）
    pub fidelity_issues: Vec<String>,
    /// 残留审计发现的问题（不改写全文，仅提示）
    pub residual_issues: Vec<String>,
    /// 是否执行过修复轮
    pub repaired: bool,
    /// 给作者的改动说明
    pub summary: String,
}

/// 去 AI 味重写本章正文（bounded scope，默认）
#[tauri::command]
pub async fn rewrite_chapter_deai(
    state: tauri::State<'_, AppState>,
    chapter_id: String,
    model: Option<String>,
    skill_cards: Option<Vec<String>>,
) -> Result<DeaiRewriteResult, String> {
    lh::ensure_api_keys_loaded(&state);
    let id = ChapterId::new(chapter_id);

    // ── 1. 快照章节 + 组装上下文 ──
    let (chapter, ledger, prev_tail, anti_ai, model_id) = {
        let onto = state.ontology.read();
        let chapter = onto
            .get_chapter(&id)
            .cloned()
            .ok_or_else(|| format!("章节 {} 不存在", id))?;
        if chapter.content.trim().is_empty() {
            return Err("本章没有正文可重写".to_string());
        }
        let ledger = build_fact_ledger(&onto, &chapter);
        let prev_tail: String = onto
            .chapters
            .iter()
            .filter(|ch| {
                ch.chapter_no > 0 && ch.chapter_no < chapter.chapter_no && !ch.summary.is_empty()
            })
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|ch| format!("第{}章《{}》：{}", ch.chapter_no, ch.title, ch.summary))
            .collect::<Vec<_>>()
            .join("\n");
        let template = resolve_project_workflow(&state);
        let model_id =
            resolve_stage_model(&state, template.as_ref(), model.as_deref(), STAGE_WRITING)
                .or_else(|| first_available_model(&state))
                .ok_or_else(|| {
                    "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string()
                })?;
        let cards = resolve_stage_cards(
            &state,
            template.as_ref(),
            skill_cards.as_ref(),
            STAGE_WRITING,
        );
        let cards_block = crate::commands::book_distill::load_writing_cards(&state, &cards);
        let anti_ai = state.anti_ai.read().prompt.clone();
        let _ = cards_block; // 去 AI 味重写以规则为主，技法卡暂不注入
        (chapter, ledger, prev_tail, anti_ai, model_id)
    };

    // ── 2. 解析模型供应商 ──
    let models = lh::load_models(&state);
    let providers = lh::load_providers(&state);
    let m2p = lh::build_model_to_provider(&models);
    let bases = lh::build_provider_api_bases(&providers);
    let api_keys = state.api_keys.read().clone();
    let (provider_id, api_key, api_base) =
        lh::resolve_provider(&model_id, &m2p, &bases, &api_keys)?;
    let auth = lh::ProviderAuth {
        provider_id: &provider_id,
        api_key: &api_key,
        api_base: &api_base,
    };

    let original_words = chapter.content.chars().count() as u32;

    // ── 3. 有界改写（只做句内清洗；整句空话进清单不直接删）──
    let raw = rewrite_pass(&auth, &model_id, &chapter, &ledger, &prev_tail, &anti_ai).await?;
    let (body_raw, list_raw) = split_deai_output(&raw);
    let content = parse_writing_output(body_raw);
    if content.trim().is_empty() {
        return Err("去 AI 味重写未产出正文，请重试".to_string());
    }
    let deletions = parse_deletions(list_raw);

    // ── 4. 两步回读（保真回读 + 残留审计）──
    let reread = reread_pass(&auth, &model_id, &chapter, &content).await?;

    // ── 5. 保真问题自动修复（只修问题点，不再全文重写）──
    let (final_content, repaired) = if reread.fidelity_issues.is_empty() {
        (content, false)
    } else {
        let fix = repair_pass(
            &auth,
            &model_id,
            &chapter,
            &content,
            &reread.fidelity_issues,
        )
        .await?;
        let fixed = parse_writing_output(&fix);
        if fixed.trim().is_empty() {
            (content, false)
        } else {
            (fixed, true)
        }
    };

    // ── 6. 落库：快照旧版 → 新正文（版本历史可回滚）──
    let (new_version, word_count) = {
        let mut onto = state.ontology.write();
        let ch = onto
            .chapters
            .iter_mut()
            .find(|ch| ch.chapter_id == id)
            .ok_or_else(|| format!("章节 {} 不存在", id))?;
        ch.revisions.push(ChapterRevision {
            version: ch.version,
            content: ch.content.clone(),
            word_count: ch.word_count,
            created_at: now(),
            reason: "去AI味重写前快照".to_string(),
        });
        if ch.revisions.len() > MAX_REVISIONS {
            let excess = ch.revisions.len() - MAX_REVISIONS;
            ch.revisions.drain(..excess);
        }
        let new_version = ch.version + 1;
        ch.content = final_content.clone();
        ch.word_count = final_content.chars().count() as u32;
        ch.version = new_version;
        ch.updated_at = now();
        (new_version, ch.word_count)
    };

    crate::integration::on_chapter_saved(&state, &id);
    state
        .save()
        .map_err(|e| format!("去 AI 味重写落盘失败: {e}"))?;

    Ok(DeaiRewriteResult {
        new_version,
        word_count,
        original_word_count: original_words,
        suggested_deletions: deletions.deletions,
        fidelity_issues: reread
            .fidelity_issues
            .iter()
            .map(|i| format!("{}：{}", i.item, i.detail))
            .collect(),
        residual_issues: reread
            .residual_issues
            .iter()
            .map(|i| format!("{}：{}", i.item, i.detail))
            .collect(),
        repaired,
        summary: deletions.summary,
    })
}

// ── 内部实现 ──────────────────────────────────────────────────────────

/// 保真账本：从本体确定性提取重写不得破坏的事实要素
fn build_fact_ledger(onto: &pensoul_core::NovelOntology, chapter: &Chapter) -> String {
    let mut lines: Vec<String> = Vec::new();
    let names: Vec<String> = onto
        .characters
        .characters
        .iter()
        .map(|c| c.name.clone())
        .collect();
    if !names.is_empty() {
        lines.push(format!("人物：{}", names.join("、")));
    }
    let locs: Vec<String> = onto
        .world
        .spatial_model
        .locations
        .iter()
        .map(|l| l.name.clone())
        .collect();
    if !locs.is_empty() {
        lines.push(format!("地点：{}", locs.join("、")));
    }
    let terms: Vec<String> = onto.world.glossary.iter().map(|t| t.term.clone()).collect();
    if !terms.is_empty() {
        lines.push(format!("设定术语：{}", terms.join("、")));
    }
    // 数字与修饰对象：粗提取「数字 + 后随 4 字」，避免改写时拆散
    let digits: Vec<String> = {
        let re = Regex::new(r"\d+(?:\.\d+)?[^，。！？；\n]{0,4}").unwrap();
        let mut seen = Vec::new();
        for m in re.find_iter(&chapter.content).take(20) {
            let s = m.as_str().trim().to_string();
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
        seen
    };
    if !digits.is_empty() {
        lines.push(format!(
            "数字/专名（与修饰对象同留）：{}",
            digits.join("、")
        ));
    }
    format!(
        "【本章保真账本（重写不得破坏）】\n{}\n\
         规则：不新增事实；不删核心事实；人物动作/对话内容/事件结果必须保留；\n\
         谁做的动作、谁说的话不得改嫁；数字与修饰对象必须一起保留；\n\
         不补虚构来源；对话不改变原意、不擅自扩写。",
        lines.join("\n")
    )
}

/// 第一步：有界改写（句内清洗 + 空句进清单）
#[allow(clippy::too_many_arguments)]
async fn rewrite_pass(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    chapter: &Chapter,
    ledger: &str,
    prev_tail: &str,
    anti_ai: &str,
) -> Result<String, String> {
    let system = format!(
        "你是一位严谨的网文编辑，负责把章节正文里的「AI 味」去掉，同时绝不伤害信息。\n\
         【保真合同（硬性）】\n\
         1. 不新增事实：不得添加原文没有的人物、事件、数字、设定；\n\
         2. 不删核心事实：人物动作、对话内容、事件结果、数字与单位必须保留；\n\
         3. 不改责任主体：谁做的动作、谁说的话不得改嫁他人；\n\
         4. 数字与它修饰的对象必须一起保留；\n\
         5. 无源引用（研究表明/据说）不得补虚构来源；\n\
         6. 对话不改变原意、不擅自扩写。\n\
         【改写力度（bounded）】\n\
         - 只做句内清洗：删句首可剥离的引导词（值得注意的是/与此同时/然而…）、\n\
           拆翻译腔、去掉套话与空泛修饰、把情绪直说改成具体动作；\n\
         - 整句都是空话（删掉后该段信息点不变）→ 不直接删，列入「建议删除清单」；\n\
         - 不合并相邻句、不重排段落、不调整叙事顺序；\n\
         - 不把短句拉长，不把长句拆得七零八落。\n\
         【语言铁律（项目配置）】\n{anti_ai}\n\
         【结构禁令（命中即处理）】\n\
         1. 二元对比空转（不是X而是Y / 不仅X还Y / 不仅仅X更是Y）→ 直接说成立的判断；\n\
         2. 章末预告式收束（然而事情远没有结束 / 这仅仅是个开始 / 接下来要…）→ 让场景自然收尾；\n\
         3. 翻译腔（对于…而言 / 基于… / 使得…得以 / 在…的过程中 / 长「的」字链）→ 拆成主动短句；\n\
         4. 抽象转义（本质上 / 归根结底 / 真正重要的是）→ 落到具体事实或动作；\n\
         5. 同一连接词全章最多出现一次，能用短句断开就不用连接词粘合；\n\
         6. 情绪直说 → 用动作和感知代替。\n\
         【输出协议（严格）】\n\
         正文包裹在 {DEAI_BEGIN} 与 {DEAI_END} 之间；\n\
         正文之后输出 {LIST_BEGIN} 与 {LIST_END} 包裹的纯 JSON：\n\
         {{\"deletions\": [{{\"sentence\": \"整句空话原句\", \"reason\": \"为什么删了不丢信息（30字内）\"}}], \
         \"summary\": \"给作者的改动说明（300字内：改了什么、保留了什么）\"}}\n\
         没有建议删除的句子时 deletions 为空数组。标记之外不得输出任何内容。"
    );
    let user = format!(
        "{ledger}\n\n\
         【前情衔接】\n{}\n\n\
         【待重写章节】第 {} 章《{}》（梗概：{}）\n\
         原正文：\n{}\n\n\
         请按上述规则做去 AI 味句内清洗，输出完整改写稿。",
        if prev_tail.trim().is_empty() {
            "（无前章信息）".to_string()
        } else {
            prev_tail.to_string()
        },
        chapter.chapter_no,
        chapter.title,
        chapter.summary,
        cap_chars(&chapter.content, 12000),
    );
    lh::call_llm_task(auth, model_id, &system, &user, 0.3, 16384, LlmTask::Deep).await
}

/// 第二步：两步回读（保真回读 5 项 + 残留审计 5 项），输出诊断 JSON
async fn reread_pass(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    chapter: &Chapter,
    rewritten: &str,
) -> Result<RereadPayload, String> {
    let system = "你是一位极其严格的质检员。对改写稿做两步回读，输出严格 JSON，不评论、不解释。\
        第一步保真回读，只查 5 项：\n\
        ① 新增事实（改写里出现了原文没有的人物/事件/数字/设定）\n\
        ② 信息丢失（核心事实/数字/专名/对话内容被删或被稀释）\n\
        ③ 责任主体变化（动作或话语被安到别人头上）\n\
        ④ 术语与设定失真（世界观术语被同义替换、改义）\n\
        ⑤ 生硬断裂（删改后句子读不通、指代悬空）\n\
        第二步残留审计，只查 5 项：\n\
        ① 开场/总结残留（值得注意的是/综上所述/总而言之等）\n\
        ② narrator 残留（还在解释「这说明了什么」而不是直接写）\n\
        ③ 空泛判断残留（方向是对的/意义重大等）\n\
        ④ 二元对比与预告式收束残留\n\
        ⑤ 句长过匀（每句差不多长、像被统一抛光）\n\
        输出：{\"fidelity_issues\": [{\"item\": \"违反了哪条（如 新增事实）\", \"detail\": \"原文片段 → 改写片段（40字内）\"}], \
        \"residual_issues\": [{\"item\": \"残留类型\", \"detail\": \"改写稿中的具体句子（40字内）\"}], \
        \"ok\": true 或 false}。没有问题的数组留空。";
    let user = format!(
        "【原文】\n{}\n\n\
         【改写稿】\n{}\n\n\
         请按两步回读输出 JSON。",
        cap_chars(&chapter.content, 12000),
        cap_chars(rewritten, 12000),
    );
    let raw = lh::call_llm_task(auth, model_id, system, &user, 0.1, 4096, LlmTask::Light).await?;
    let json_str = extract_block(&raw, "===REREAD_BEGIN===", "===REREAD_END===");
    serde_json::from_str::<RereadPayload>(json_str)
        .or_else(|strict_err| {
            json_fix::repair_to_value(json_str)
                .ok()
                .and_then(|v| serde_json::from_value::<RereadPayload>(v).ok())
                .ok_or(strict_err.to_string())
        })
        .map_err(|e| format!("两步回读解析失败: {e}"))
}

/// 第三步：保真问题修复（只修问题点，其余保持不动）
async fn repair_pass(
    auth: &lh::ProviderAuth<'_>,
    model_id: &str,
    chapter: &Chapter,
    rewritten: &str,
    fidelity_issues: &[IssueItem],
) -> Result<String, String> {
    let issues_text = fidelity_issues
        .iter()
        .map(|i| format!("- {}：{}", i.item, i.detail))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "你是严谨的网文编辑。只修复下面列出的保真问题，其余文字保持一字不动。\n\
         输出协议：修复后的完整正文包裹在 {DEAI_BEGIN} 与 {DEAI_END} 之间，标记之外不得输出任何内容。"
    );
    let user = format!(
        "【原文（事实基准）】\n{}\n\n\
         【当前改写稿】\n{}\n\n\
         【需要修复的保真问题】\n{issues_text}\n\n\
         请逐条修复，输出修复后的完整正文。",
        cap_chars(&chapter.content, 12000),
        cap_chars(rewritten, 12000),
    );
    let raw = lh::call_llm_task(auth, model_id, &system, &user, 0.2, 16384, LlmTask::Deep).await?;
    Ok(raw)
}

/// 拆分改写输出：正文块 + 删除清单块
fn split_deai_output(raw: &str) -> (&str, &str) {
    let body =
        crate::commands::chapter_rewrite::extract_between(raw, DEAI_BEGIN, DEAI_END).unwrap_or(raw);
    let list =
        crate::commands::chapter_rewrite::extract_between(raw, LIST_BEGIN, LIST_END).unwrap_or("");
    (body, list)
}

/// 解析建议删除清单（best-effort，失败返回空清单不阻断重写）
fn parse_deletions(list_raw: &str) -> DeletionPayload {
    if list_raw.trim().is_empty() {
        return DeletionPayload::default();
    }
    let json_str = extract_block(list_raw, "", "");
    serde_json::from_str::<DeletionPayload>(json_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deletions_ok() {
        let raw = "===LIST_BEGIN===\n{\"deletions\": [{\"sentence\": \"总而言之，此事意义重大。\", \"reason\": \"空总结，删后信息不变\"}], \"summary\": \"已做句内清洗\"}\n===LIST_END===";
        let p = parse_deletions(raw);
        assert_eq!(p.deletions.len(), 1);
        assert_eq!(p.summary, "已做句内清洗");
    }

    #[test]
    fn test_parse_deletions_empty() {
        let p = parse_deletions("");
        assert!(p.deletions.is_empty());
    }

    #[test]
    fn test_split_deai_output() {
        let raw = format!(
            "前言\n{DEAI_BEGIN}\n正文第一段\n{DEAI_END}\n{LIST_BEGIN}\n{{\"deletions\": []}}\n{LIST_END}"
        );
        let (body, list) = split_deai_output(&raw);
        assert!(body.contains("正文第一段"));
        assert!(list.contains("deletions"));
    }

    #[test]
    fn test_build_fact_ledger_extracts_digits() {
        let onto =
            pensoul_core::NovelOntology::new(pensoul_core::ProjectId::new("p"), String::new());
        let mut chapter = pensoul_core::Chapter {
            chapter_id: pensoul_core::ChapterId::new("c1"),
            chapter_no: 1,
            volume_id: pensoul_core::VolumeId::new("vol-1"),
            title: "测试章".to_string(),
            summary: String::new(),
            content: String::new(),
            word_count: 0,
            version: 1,
            status: pensoul_core::ChapterStatus::Draft,
            consistency_score: 1.0,
            created_at: String::new(),
            updated_at: String::new(),
            annotations: Vec::new(),
            revisions: Vec::new(),
        };
        chapter.content = "p95 延迟从 480ms 降到 160ms。他走了 3 里路。".to_string();
        let ledger = build_fact_ledger(&onto, &chapter);
        assert!(ledger.contains("480ms"));
        assert!(ledger.contains("3 里路"));
    }
}
