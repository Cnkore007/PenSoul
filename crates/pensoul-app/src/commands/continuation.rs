//! 半成品续写：导入已有正文 → 反推蓝图 → 用现有 pipeline 继续扩写
//!
//! 与 book_distill 不同：蒸馏是「学别人的书的方法」，本模块是「接管自己写了一半的书」。
//! 流程：解析书籍文件 → 按章导入 chapters → LLM 从正文提取设定候选
//! （包装成 DiscussionSynthesis）→ 复用 build_blueprint + llm_convert_blueprint 反推蓝图
//! → 落盘。作者随后在蓝图页修正，再按现有 pipeline 从已写章之后继续写。

use crate::state::AppState;
use pensoul_core::{
    BookBlueprint, Chapter, ChapterId, ChapterStatus, DiscussionSynthesis, OutlineBeat,
    Volume, VolumeId,
};
use serde::Deserialize;
use std::collections::HashMap;

use super::blueprint::{build_blueprint_from_syn, now};
use super::blueprint_llm::{llm_convert_blueprint, pick_blueprint_model};
use super::book_file::{read_book_file, sample_text};
use super::discussion::call_with_system_task;
use super::llm_helper as lh;

/// 提取结果直接复用讨论合成的核心类型（字段名对齐 + serde default 兜底）
#[derive(Debug, Default, Deserialize)]
struct Extracted {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    characters: Vec<pensoul_core::CharacterItem>,
    #[serde(default)]
    locations: Vec<pensoul_core::NamedDesc>,
    #[serde(default)]
    setting_rules: Vec<pensoul_core::NamedDesc>,
    #[serde(default)]
    outline_beats: Vec<OutlineBeat>,
    #[serde(default)]
    commitments: Vec<pensoul_core::CommitmentItem>,
    #[serde(default)]
    subplots: Vec<pensoul_core::SubplotItem>,
}

/// 导入已有正文并反推蓝图
#[tauri::command]
pub async fn import_book_for_continuation(
    state: tauri::State<'_, AppState>,
    file_path: String,
    model: Option<String>,
) -> Result<BookBlueprint, String> {
    lh::ensure_api_keys_loaded(&state);

    // 1. 解析书籍文件（epub/pdf 解压耗时，放阻塞线程池）
    let fp = file_path.clone();
    let book = tokio::task::spawn_blocking(move || read_book_file(&fp))
        .await
        .map_err(|e| format!("解析书籍文件任务失败: {e}"))??;
    let chapters = split_chapters(&book.full_text);
    if chapters.is_empty() {
        return Err("没有解析到章节内容，请确认文件是分章的正文（txt/md/epub/pdf）".to_string());
    }

    // 2. LLM 从正文提取设定骨架；失败回退到「章标题骨架」（导入不中断）
    let syn = match extract_synthesis(&state, &book, &chapters, model.as_deref()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("续写反推提取失败，使用章标题骨架: {e}");
            fallback_synthesis(&book, &chapters)
        }
    };

    // 3. 导入章节 + 构建确定性骨架（持锁、无 await）
    let fallback = {
        let mut onto = state.ontology.write();
        import_chapters(&mut onto, &book.title_guess, &chapters);
        build_blueprint_from_syn(&syn, &onto)?
    };
    // 4. LLM 账本化转换（无锁调用）；失败回退确定性映射
    let converted = llm_convert_blueprint(&state, &syn, &fallback, &[]).await;
    // 5. 写回蓝图并落盘
    let bp = {
        let mut onto = state.ontology.write();
        let mut bp = converted.unwrap_or_else(|e| {
            eprintln!("续写反推账本化转换失败，使用确定性映射: {e}");
            fallback.clone()
        });
        bp.settled = true;
        bp.settled_at = now();
        bp.settled_from = format!("从正文反推（《{}》）", book.title_guess);
        bp.source_stamp = format!("import|{}|{}", chapters.len(), syn.summary.chars().count());
        onto.blueprint = bp.clone();
        bp
    };
    state.save().map_err(|e| format!("保存续写蓝图失败: {e}"))?;
    Ok(bp)
}

/// 章节切分：按「第X章/第X回/第X节/第X卷/序章/楔子/尾声/Chapter N」标题分行
fn split_chapters(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut cur_title = "全文".to_string();
    let mut buf = String::new();
    for line in text.lines() {
        let t = line.trim();
        if looks_like_chapter_heading(t) {
            if !buf.trim().is_empty() {
                out.push((cur_title.clone(), std::mem::take(&mut buf)));
            }
            cur_title = t.to_string();
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if !buf.trim().is_empty() {
        out.push((cur_title, buf));
    }
    if out.is_empty() {
        out.push(("全文".to_string(), text.to_string()));
    }
    out
}

fn looks_like_chapter_heading(t: &str) -> bool {
    if t.is_empty() || t.chars().count() > 30 {
        return false;
    }
    let chars: Vec<char> = t.chars().collect();
    if t.starts_with("第") && chars.iter().any(|c| "章回节卷".contains(*c)) {
        return true;
    }
    matches!(
        t,
        "序章" | "楔子" | "尾声" | "番外" | "后记" | "前言" | "引子"
    ) || t.to_lowercase().starts_with("chapter ")
        || t.to_lowercase().starts_with("chapter")
}

/// 把切出的章节写入项目本体（新卷「导入正文《书名》」，章节按序追加）
fn import_chapters(
    onto: &mut pensoul_core::NovelOntology,
    book_title: &str,
    chapters: &[(String, String)],
) {
    let vid = VolumeId::new(format!(
        "vol-import-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    let now_str = now();
    let mut ids = Vec::with_capacity(chapters.len());
    for (i, (title, content)) in chapters.iter().enumerate() {
        let cid = ChapterId::new(format!("ch-import-{:04}", i + 1));
        onto.chapters.push(Chapter {
            chapter_id: cid.clone(),
            chapter_no: (onto.chapters.len() + 1) as i64,
            volume_id: vid.clone(),
            title: title.clone(),
            summary: String::new(),
            content: content.clone(),
            word_count: content.chars().count() as u32,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 0.0,
            created_at: now_str.clone(),
            updated_at: now_str.clone(),
            annotations: Vec::new(),
            revisions: Vec::new(),
        });
        ids.push(cid);
    }
    onto.volumes.push(Volume {
        volume_id: vid,
        title: format!("导入正文《{book_title}》"),
        chapter_ids: ids,
        summary: format!("从已有正文导入 {} 章，可继续扩写", chapters.len()),
        expanded: true,
    });
}

/// LLM 从正文抽样提取设定骨架（复用讨论合成的核心类型）
async fn extract_synthesis(
    state: &AppState,
    book: &super::book_file::BookText,
    chapters: &[(String, String)],
    model: Option<&str>,
) -> Result<DiscussionSynthesis, String> {
    let model = match model {
        Some(m) if !m.trim().is_empty() => m.to_string(),
        _ => pick_blueprint_model(state)?,
    };
    let saved_providers = lh::load_providers(state);
    let saved_models = lh::load_models(state);
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let api_keys: HashMap<String, String> = { state.api_keys.read().clone() };

    let sample = sample_text(&book.full_text);
    let chapter_index: Vec<String> = chapters
        .iter()
        .enumerate()
        .map(|(i, (t, _))| format!("第{}章 {t}", i + 1))
        .collect();
    let user = format!(
        "你是资深小说编辑，任务是从一部已写小说的正文样本中反推它的设定骨架。\n\
         书名：《{}》\n\
         章节清单（共 {} 章）：\n{}\n\n\
         【正文样本（开头+中段+结尾抽样）】\n{}\n\n\
         【提取要求】\n\
         1. characters：只列有名字的具体个体（entity_kind=individual），给出 wants（核心欲望）、\
         fears（恐惧）、secret（秘密）、speech_style、arc（成长阶段，chapter_range 用「第X-Y章」）\n\
         2. locations：主要地点（description/level/region/faction/unlocked_chapter）\n\
         3. setting_rules：正文明确展现的力量体系/世界规则/铁律（description/constraints）\n\
         4. outline_beats：把已写内容归纳成 5-15 个情节节点（title/description/chapter_hint 用\
         「第X-Y章」/volume 用「第N卷」/beat_type 用 铺垫|转折|高潮|爽点|收束/hook/payoff/\
         foreshadowing 列出已埋伏笔）\n\
         5. commitments：正文反复强调或明显作为卖点的承诺（statement ≤60字，kind 用\
         theme|promise|tone|rule|no_go，ongoing 标是否持续型）\n\
         6. subplots：贯穿的副线（mainline_relation/chapter_range/open_threads/characters）\n\
         7. summary：150 字内全书概览\n\
         所有内容必须来自正文样本与章节清单，禁止臆造；全部用中文。\n\
         用 ===EXTRACT_BEGIN=== 与 ===EXTRACT_END=== 包裹纯 JSON，\
         结构：{{\"summary\":\"\",\"characters\":[{{\"name\":\"\",\"entity_kind\":\"individual\",\
         \"wants\":\"\",\"fears\":\"\",\"secret\":\"\",\"speech_style\":\"\",\"arc\":\
         [{{\"name\":\"\",\"chapter_range\":\"\",\"goal\":\"\",\"trait_desc\":\"\"}}],\
         \"knows\":[],\"does_not_know\":[]}}],\"locations\":[{{\"name\":\"\",\"description\":\"\",\
         \"level\":\"\",\"region\":\"\",\"faction\":\"\",\"unlocked_chapter\":\"\"}}],\
         \"setting_rules\":[{{\"name\":\"\",\"description\":\"\",\"constraints\":[]}}],\
         \"outline_beats\":[{{\"title\":\"\",\"description\":\"\",\"chapter_hint\":\"第1-10章\",\
         \"volume\":\"第1卷\",\"beat_type\":\"铺垫\",\"hook\":\"\",\"payoff\":\"\",\
         \"foreshadowing\":[{{\"plant\":\"\",\"payoff_hint\":\"\",\"payoff_anchor_type\":\
         \"chapter|volume|event\",\"payoff_anchor\":\"\"}}]}}],\
         \"commitments\":[{{\"statement\":\"\",\"kind\":\"theme\",\"scope\":\"book\",\
         \"ongoing\":true}}],\"subplots\":[{{\"name\":\"\",\"mainline_relation\":\"\",\
         \"chapter_range\":\"\",\"open_threads\":[],\"characters\":[]}}]}}",
        book.title_guess,
        chapters.len(),
        chapter_index.join("\n"),
        sample,
    );
    let system = "你是资深小说编辑，负责从已写正文反推设定骨架。严格按用户给定的 JSON 结构输出，\
        不要输出任何解释、标记或额外文本。";
    let text = call_with_system_task(
        &model,
        system,
        &user,
        0.2,
        16_384,
        crate::llm_profile::LlmTask::Light,
        &model_to_provider,
        &provider_api_bases,
        &api_keys,
    )
    .await?;
    let block = extract_json_block(&text)?;
    let parsed = serde_json::from_str::<Extracted>(&block)
        .map_err(|e| format!("解析提取结果失败: {e}"))?;
    Ok(to_synthesis(parsed))
}

/// 提取失败时的章标题骨架：保证「导入 → 反推 → 续写」链路不中断
fn fallback_synthesis(book: &super::book_file::BookText, chapters: &[(String, String)]) -> DiscussionSynthesis {
    let beats = chapters
        .iter()
        .enumerate()
        .map(|(i, (t, _))| OutlineBeat {
            title: t.clone(),
            description: String::new(),
            chapter_hint: format!("第{}章", i + 1),
            volume: String::new(),
            beat_type: "导入".to_string(),
            ..Default::default()
        })
        .collect();
    DiscussionSynthesis {
        summary: format!(
            "从《{}》导入的 {} 章正文。正文已入库，设定骨架待补充。",
            book.title_guess,
            chapters.len()
        ),
        outline_beats: beats,
        ..Default::default()
    }
}

fn to_synthesis(ex: Extracted) -> DiscussionSynthesis {
    DiscussionSynthesis {
        summary: ex.summary,
        characters: ex.characters,
        locations: ex.locations,
        setting_rules: ex.setting_rules,
        outline_beats: ex.outline_beats,
        commitments: ex.commitments,
        subplots: ex.subplots,
        ..Default::default()
    }
}

/// 提取 ===EXTRACT_BEGIN=== 与 ===EXTRACT_END=== 之间的 JSON
fn extract_json_block(text: &str) -> Result<String, String> {
    let begin = "===EXTRACT_BEGIN===";
    let end = "===EXTRACT_END===";
    let s = match text.find(begin) {
        Some(i) => &text[i + begin.len()..],
        None => text.trim(),
    };
    let s = match s.find(end) {
        Some(i) => &s[..i],
        None => s,
    };
    let block = s.trim().trim_start_matches("```json").trim_end_matches("```").trim();
    if block.is_empty() {
        return Err("提取结果为空".to_string());
    }
    Ok(block.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_chapters_basic() {
        let text = "第一章 开局\n内容A\n\n第二章 转折\n内容B\n尾声\n收尾内容";
        let ch = split_chapters(text);
        assert_eq!(ch.len(), 3);
        assert_eq!(ch[0].0, "第一章 开局");
        assert_eq!(ch[1].0, "第二章 转折");
        assert_eq!(ch[2].0, "尾声");
        assert!(ch[2].1.contains("收尾内容"));
    }

    #[test]
    fn test_split_chapters_no_headings() {
        let ch = split_chapters("没有分章的整篇文本");
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].0, "全文");
    }

    #[test]
    fn test_looks_like_chapter_heading() {
        assert!(looks_like_chapter_heading("第1章 风起"));
        assert!(looks_like_chapter_heading("第十二回 迷局"));
        assert!(looks_like_chapter_heading("Chapter 7"));
        assert!(looks_like_chapter_heading("楔子"));
        assert!(!looks_like_chapter_heading("这是一段很长的正文内容，超过三十个字会被拒绝作为标题因为太长了"));
    }

    #[test]
    fn test_extract_json_block_strips_fences() {
        let text = "好的\n===EXTRACT_BEGIN===\n```json\n{\"summary\":\"x\"}\n```\n===EXTRACT_END===\n完毕";
        let block = extract_json_block(text).unwrap();
        assert_eq!(block, "{\"summary\":\"x\"}");
    }
}
