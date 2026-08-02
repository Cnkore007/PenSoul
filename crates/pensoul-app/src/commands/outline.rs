//! 情节脉络（大纲规划层）IPC 命令
//!
//! 两层大纲模型：
//! - 「情节脉络」节点（OutlineArc）：讨论成果的剧情规划，覆盖一个章节范围
//!   （如第 1-200 章），本身不可写正文；
//! - 「章节细纲」（Chapter.summary）：脉络节点按范围分批展开生成，
//!   造化工坊只对真正的章节写作。
//!
//! `expand_outline_arc_all` 在后台任务中按批展开该节点的全部剩余章节
//! （默认每批 20 章，批间携带已展开章节作为上下文衔接），实时推送进度事件，
//! 支持取消与页面切换后重连（控制面缓冲 + get_outline_expand_state）。
use crate::state::AppState;
use pensoul_core::workflow::WorkflowRef;
use pensoul_core::{Chapter, ChapterId, ChapterStatus, OutlineArc, VolumeId};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::json_fix;
use super::llm_helper as lh;

/// 每批展开的章节数（一次 LLM 调用的产出量，过多容易截断）
const DEFAULT_BATCH: i64 = 20;

/// 列出全部情节脉络节点
#[tauri::command]
pub async fn list_outline_arcs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<OutlineArc>, String> {
    let ontology = state.ontology.read();
    Ok(ontology.outline_arcs.clone())
}

/// 整体保存情节脉络（新建/编辑/删除都走这里；导入讨论成果由前端组装后调用）
#[tauri::command]
pub async fn save_outline_arcs(
    state: tauri::State<'_, AppState>,
    arcs: Vec<OutlineArc>,
) -> Result<(), String> {
    let samples = {
        let onto = state.ontology.read();
        crate::edits::outline_arcs_diff_samples(&onto.outline_arcs, &arcs)
    };
    {
        let mut ontology = state.ontology.write();
        ontology.outline_arcs = arcs;
    }
    crate::edits::record_edit_samples(&state, samples);
    state.save().map_err(|e| e.to_string())
}

/// 展开结果：本批生成的章节范围与节点完成状态
#[derive(serde::Serialize)]
pub struct ExpandResult {
    pub created: usize,
    pub from: i64,
    pub to: i64,
    /// 该节点是否已全部展开（expanded_until >= chapter_end）
    pub arc_done: bool,
}

/// 细纲条目（LLM 产出的单章规划；章号由后端按批次顺序分配，不信任模型编号）
#[derive(Debug, Deserialize)]
struct BeatPlan {
    title: String,
    #[serde(default)]
    summary: String,
}

/// 展开脉络节点的下一批细纲：调 LLM 把该故事段的剧情规划
/// 拆成逐章梗概，生成真正的章节实体（正文留空，等待造化工坊写作）
/// `skill_cards`：工作流为细纲展开环节绑定的技法卡路径（可空，注入 system prompt）
#[tauri::command]
pub async fn expand_outline_arc(
    state: tauri::State<'_, AppState>,
    arc_id: String,
    model: Option<String>,
    batch: Option<i64>,
    skill_cards: Option<Vec<String>>,
) -> Result<ExpandResult, String> {
    expand_batch(&state, &arc_id, model, batch, skill_cards).await
}

/// 单批展开（内部复用）：取节点快照 → 组装上下文 → 调 LLM → 落库更新进度。
/// 供单批命令与「全部展开」后台任务共用。
async fn expand_batch(
    state: &AppState,
    arc_id: &str,
    model: Option<String>,
    batch: Option<i64>,
    skill_cards: Option<Vec<String>>,
) -> Result<ExpandResult, String> {
    lh::ensure_api_keys_loaded(state);

    // 取节点快照并计算本批范围
    let (arc, from, to) = {
        let ontology = state.ontology.read();
        let arc = ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .cloned()
            .ok_or_else(|| "脉络节点不存在".to_string())?;
        let from = if arc.expanded_until <= 0 {
            arc.chapter_start
        } else {
            arc.expanded_until + 1
        };
        if from > arc.chapter_end {
            return Err("该节点已全部展开为细纲".to_string());
        }
        let to = (from + batch.unwrap_or(DEFAULT_BATCH) - 1).min(arc.chapter_end);
        (arc, from, to)
    };

    // 组装上下文：核心概念 + 创作设定 + 节点规划 + 衔接前情
    let (concept_brief, settings_brief, prev_tail, volume_id) = {
        let ontology = state.ontology.read();
        let c = &ontology.core_concept;
        let concept_brief = format!(
            "高概念：{}；前提：{}；主角：{}；基调：{}；核心冲突：{}",
            c.high_concept, c.premise, c.protagonist_hint, c.tone, c.central_conflict
        );
        let s = &ontology.settings;
        let settings_brief = format!(
            "类型：{}；目标总章数：{} 章；每章目标字数：{} 字",
            s.genre, s.target_chapters, s.chapter_target_words
        );
        // 上下文衔接：本批起点之前的章节梗概（取最近 12 章，保证剧情连续；
        // 跨节点时前面节点的已展开章节也自然包含在内，因章节号全局连续）
        let prev_tail: String = ontology
            .chapters
            .iter()
            .filter(|ch| ch.chapter_no > 0 && ch.chapter_no < from && !ch.summary.is_empty())
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .take(12)
            .rev()
            .map(|ch| format!("第{}章《{}》：{}", ch.chapter_no, ch.title, ch.summary))
            .collect::<Vec<_>>()
            .join("\n");
        // 章节落卷规则与讨论导入一致：优先第一个真实卷，否则隐式默认卷
        let volume_id = ontology
            .volumes
            .iter()
            .find(|v| v.volume_id.as_str() != "_default")
            .map(|v| v.volume_id.as_str().to_string())
            .unwrap_or_else(|| "_default".to_string());
        (concept_brief, settings_brief, prev_tail, volume_id)
    };

    let model_id = resolve_expand_model(state, model)?;
    let count = to - from + 1;
    let mut system = "你是小说大纲规划师。你的任务是把一段剧情脉络拆解为逐章细纲，输出严格 JSON。\
        不评论、不解释，只输出 JSON 数组。"
        .to_string();
    // 工作流为细纲展开绑定的技法卡（结构/人物/张力/类型维度），注入为方法手册。
    // 显式参数优先，缺省时按「项目覆盖 → 模板绑定」解析（与造化工坊同一套规则）
    let cards_block =
        super::book_distill::load_writing_cards(state, &resolve_expand_cards(state, skill_cards));
    if !cards_block.is_empty() {
        system.push_str(&format!(
            "\n\n【写作技法卡】\n\
            以下是本书选定工作流绑定的写作技法卡，是你拆解细纲的方法手册：\n\
            篇章布局遵循其「I · 技法骨架」与「E · 执行步骤」，节奏与结构遵守其「B · 边界」。\n\n{cards_block}"
        ));
    }
    let user = format!(
        "【核心概念】\n{concept_brief}\n\n\
         【创作设定】\n{settings_brief}\n\n\
         【情节脉络节点】\n标题：{}\n覆盖范围：第 {}-{} 章（全段共 {} 章）\n剧情规划：\n{}\n\n\
         {}\
         现在请为本段生成第 {from} 章到第 {to} 章（共 {count} 章）的逐章细纲。\n\
         输出 JSON 数组，每章一个对象：\n\
         [{{\"title\": \"章节标题\", \"summary\": \"本章梗概（80-150字：本章发生什么、核心冲突是什么、如何推进主线）\"}}]\n\
         要求：\n\
         - 严格生成 {count} 章，不多不少，按故事发生顺序排列\n\
         - 每章梗概必须是独立的章节规划，承接前情、彼此衔接，不要把整段剧情压缩进一章\n\
         - 本批只是全段的一部分，节奏按全段跨度把控（该铺垫时铺垫，该推进时推进）\n\
         - 章节标题要有辨识度，不要「第一章」「第二章」式命名\n\
         - 所有内容用中文",
        arc.title,
        arc.chapter_start,
        arc.chapter_end,
        arc.chapter_end - arc.chapter_start + 1,
        arc.description,
        if prev_tail.is_empty() {
            String::new()
        } else {
            format!("【前情衔接（已展开的最近章节）】\n{prev_tail}\n\n")
        },
    );

    let (provider_id, api_key, api_base) = {
        let models = lh::load_models(state);
        let providers = lh::load_providers(state);
        let m2p = lh::build_model_to_provider(&models);
        let bases = lh::build_provider_api_bases(&providers);
        let keys = state.api_keys.read().clone();
        lh::resolve_provider(&model_id, &m2p, &bases, &keys)?
    };
    let raw = lh::call_llm_task(
        &lh::ProviderAuth {
            provider_id: &provider_id,
            api_key: &api_key,
            api_base: &api_base,
        },
        &model_id,
        &system,
        &user,
        0.6,
        // 每章梗概约 150 字，20 章约 4000 字正文；推理型模型还要预留思考预算
        16384,
        crate::llm_profile::LlmTask::Light,
    )
    .await?;

    let plans = parse_beat_plans(&raw)?;
    if plans.is_empty() {
        return Err("模型未产出任何细纲条目，请重试".to_string());
    }

    // 落库：按批次顺序分配章号（from + 下标），已有该章号的章节跳过防重复
    let created = {
        let mut ontology = state.ontology.write();
        let existing: std::collections::HashSet<i64> =
            ontology.chapters.iter().map(|c| c.chapter_no).collect();
        let now = chrono::Utc::now().to_rfc3339();
        let mut created = 0usize;
        for (i, plan) in plans.iter().enumerate() {
            let chapter_no = from + i as i64;
            if chapter_no > to {
                break; // 模型多产出的部分丢弃
            }
            if existing.contains(&chapter_no) {
                continue;
            }
            let title = plan.title.trim();
            if title.is_empty() {
                continue;
            }
            ontology.chapters.push(Chapter {
                chapter_id: ChapterId::new(format!(
                    "ch-{}-{}",
                    chapter_no,
                    uuid::Uuid::new_v4().simple()
                )),
                chapter_no,
                volume_id: VolumeId::new(volume_id.clone()),
                title: title.to_string(),
                summary: plan.summary.trim().to_string(),
                content: String::new(),
                word_count: 0,
                version: 1,
                status: ChapterStatus::Draft,
                consistency_score: 1.0,
                created_at: now.clone(),
                updated_at: now.clone(),
                annotations: Vec::new(),
                revisions: Vec::new(),
            });
            created += 1;
        }
        // 更新节点展开进度（按实际落库的最后一章）
        if created > 0 {
            let last = ontology
                .chapters
                .iter()
                .filter(|c| c.chapter_no >= from && c.chapter_no <= to)
                .map(|c| c.chapter_no)
                .max()
                .unwrap_or(from);
            if let Some(a) = ontology
                .outline_arcs
                .iter_mut()
                .find(|a| a.arc_id == arc_id)
            {
                a.expanded_until = last.max(a.expanded_until);
            }
        }
        // 同步卷的章节列表
        let mut by_volume: std::collections::HashMap<String, Vec<ChapterId>> =
            std::collections::HashMap::new();
        for ch in &ontology.chapters {
            by_volume
                .entry(ch.volume_id.as_str().to_string())
                .or_default()
                .push(ch.chapter_id.clone());
        }
        for vol in ontology.volumes.iter_mut() {
            if let Some(ids) = by_volume.get(vol.volume_id.as_str()) {
                vol.chapter_ids = ids.clone();
            }
        }
        created
    };

    if created == 0 {
        return Err("本批细纲没有新章节落库（对应章号可能已存在）".to_string());
    }
    state.save().map_err(|e| e.to_string())?;

    let arc_done = {
        let ontology = state.ontology.read();
        ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .map(|a| a.expanded_until >= a.chapter_end)
            .unwrap_or(false)
    };
    Ok(ExpandResult {
        created,
        from,
        to,
        arc_done,
    })
}

/// 解析展开模型：指定优先，其次项目覆盖/模板绑定的 outline_expand 模型，
/// 最后取第一个「供应商有 Key」的可用模型
fn resolve_expand_model(state: &AppState, model: Option<String>) -> Result<String, String> {
    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        return Ok(m);
    }
    if let Some(m) = resolve_bound_expand_model(state) {
        return Ok(m);
    }
    let models = lh::load_models(state);
    let keys = state.api_keys.read().clone();
    models
        .iter()
        .find_map(|m| {
            let model_id = m.get("model_id")?.as_str()?.to_string();
            let provider_id = m.get("provider_id")?.as_str()?;
            keys.contains_key(provider_id).then_some(model_id)
        })
        .ok_or_else(|| "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string())
}

// ── 全部展开：后台任务控制面 ──

/// 细纲展开进度事件 —— 实时推送给前端（同时入控制面缓冲供重连重放）
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutlineExpandEvent {
    pub arc_id: String,
    /// running / progress / done / error / cancelled
    pub phase: String,
    pub expanded: i64,
    pub total: i64,
    pub message: String,
}

/// 细纲展开控制面（挂 AppState）：运行旗标 + 当前节点 + 进度 + 取消旗标 + 事件缓冲，
/// 支持页面切换后通过 `get_outline_expand_state` 重连恢复。
pub struct OutlineExpandControl {
    pub running: AtomicBool,
    pub arc_id: parking_lot::Mutex<Option<String>>,
    pub cancel: AtomicBool,
    pub progress: parking_lot::RwLock<Option<(i64, i64)>>,
    pub events: parking_lot::RwLock<Vec<OutlineExpandEvent>>,
}

impl OutlineExpandControl {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            arc_id: parking_lot::Mutex::new(None),
            cancel: AtomicBool::new(false),
            progress: parking_lot::RwLock::new(None),
            events: parking_lot::RwLock::new(Vec::new()),
        }
    }

    fn record(&self, ev: &OutlineExpandEvent) {
        let mut buf = self.events.write();
        if buf.len() >= 200 {
            buf.remove(0);
        }
        buf.push(ev.clone());
    }
}

impl Default for OutlineExpandControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 一键展开脉络节点的全部剩余细纲：后台任务按批循环（每批默认 20 章），
/// 批间携带已展开章节作为上下文衔接，进度经 `outline-expand-phase` 事件推送；
/// 前端切换页面后可调用 `get_outline_expand_state` 重连恢复。
#[tauri::command]
pub async fn expand_outline_arc_all(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    arc_id: String,
    model: Option<String>,
    skill_cards: Option<Vec<String>>,
) -> Result<(), String> {
    if state.outline_expand.running.swap(true, Ordering::SeqCst) {
        return Err("已有细纲展开任务正在进行，请等待完成或先取消".to_string());
    }

    // 校验节点存在并计算总数
    let (total, already) = {
        let ontology = state.ontology.read();
        let arc = ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .cloned()
            .ok_or_else(|| "脉络节点不存在".to_string())?;
        (
            arc.chapter_end - arc.chapter_start + 1,
            if arc.expanded_until <= 0 {
                0
            } else {
                (arc.expanded_until - arc.chapter_start + 1).max(0)
            },
        )
    };
    if already >= total {
        state.outline_expand.running.store(false, Ordering::SeqCst);
        return Err("该节点已全部展开为细纲".to_string());
    }

    state
        .outline_expand
        .arc_id
        .lock()
        .replace(arc_id.clone());
    state.outline_expand.cancel.store(false, Ordering::SeqCst);
    *state.outline_expand.progress.write() = Some((already, total));
    state.outline_expand.events.write().clear();

    // 后台循环展开（命令立刻返回，前端通过事件与状态查询跟进）
    let state_owned = state.inner().clone();
    let app_owned = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let result = expand_all_loop(&app_owned, &state_owned, &arc_id, model, skill_cards).await;
        state_owned.outline_expand.running.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            let ev = OutlineExpandEvent {
                arc_id: arc_id.clone(),
                phase: "error".to_string(),
                expanded: state_owned
                    .outline_expand
                    .progress
                    .read()
                    .map(|p| p.0)
                    .unwrap_or(0),
                total,
                message: e,
            };
            emit_expand_event(&app_owned, &state_owned, &ev);
        }
    });
    Ok(())
}

/// 全部展开主循环：逐批调用单批展开，直到节点完成或用户取消
async fn expand_all_loop(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    arc_id: &str,
    model: Option<String>,
    skill_cards: Option<Vec<String>>,
) -> Result<(), String> {
    let total = {
        let ontology = state.ontology.read();
        let arc = ontology
            .outline_arcs
            .iter()
            .find(|a| a.arc_id == arc_id)
            .cloned()
            .ok_or_else(|| "脉络节点不存在".to_string())?;
        arc.chapter_end - arc.chapter_start + 1
    };

    let mut batches = 0u32;
    loop {
        if state.outline_expand.cancel.load(Ordering::SeqCst) {
            let ev = OutlineExpandEvent {
                arc_id: arc_id.to_string(),
                phase: "cancelled".to_string(),
                expanded: current_expanded(state, arc_id),
                total,
                message: "已取消展开，已生成部分保留".to_string(),
            };
            emit_expand_event(app_handle, state, &ev);
            return Ok(());
        }

        match expand_batch(state, arc_id, model.clone(), None, skill_cards.clone()).await {
            Ok(res) => {
                let expanded = current_expanded(state, arc_id);
                *state.outline_expand.progress.write() = Some((expanded, total));
                let ev = OutlineExpandEvent {
                    arc_id: arc_id.to_string(),
                    phase: if res.arc_done { "done" } else { "progress" }.to_string(),
                    expanded,
                    total,
                    message: if res.arc_done {
                        "全部细纲展开完成".to_string()
                    } else {
                        format!("已展开 {expanded}/{total} 章（本批第 {}-{} 章）", res.from, res.to)
                    },
                };
                emit_expand_event(app_handle, state, &ev);
                if res.arc_done {
                    return Ok(());
                }
            }
            Err(e) if e.contains("已全部展开") => return Ok(()),
            Err(e) => return Err(e),
        }

        batches += 1;
        if batches >= 200 {
            return Err("展开批次数超过上限已自动停止，可再次点击继续".to_string());
        }
    }
}

fn current_expanded(state: &AppState, arc_id: &str) -> i64 {
    let ontology = state.ontology.read();
    ontology
        .outline_arcs
        .iter()
        .find(|a| a.arc_id == arc_id)
        .map(|a| {
            if a.expanded_until <= 0 {
                0
            } else {
                (a.expanded_until - a.chapter_start + 1).max(0)
            }
        })
        .unwrap_or(0)
}

/// 发射细纲展开事件：先入控制面缓冲（供页面重连重放），再推送前端
fn emit_expand_event(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    ev: &OutlineExpandEvent,
) {
    state.outline_expand.record(ev);
    let _ = app_handle.emit("outline-expand-phase", ev);
}

/// 查询细纲展开状态（页面切换后重连恢复进度）
#[tauri::command]
pub async fn get_outline_expand_state(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let running = state.outline_expand.running.load(Ordering::SeqCst);
    let arc_id = state.outline_expand.arc_id.lock().clone();
    let progress = *state.outline_expand.progress.read();
    let events = state.outline_expand.events.read().clone();
    Ok(serde_json::json!({
        "running": running,
        "arc_id": arc_id,
        "progress": progress.map(|(e, t)| serde_json::json!({ "expanded": e, "total": t })),
        "events": events,
    }))
}

/// 取消当前细纲展开任务（已生成部分保留，可再次点击继续）
#[tauri::command]
pub async fn cancel_outline_expand(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.outline_expand.cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// 从项目工作流引用解析细纲展开的技法卡：显式参数 > 项目覆盖 > 模板绑定 > 空
fn resolve_expand_cards(state: &AppState, skill_cards: Option<Vec<String>>) -> Vec<String> {
    if let Some(cards) = skill_cards
        && !cards.is_empty()
    {
        return cards;
    }
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) else {
        return Vec::new();
    };
    if let Some(cards) = wf_ref
        .overrides
        .get("outline_expand")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        && !cards.is_empty()
    {
        return cards;
    }
    let Some(template_id) = wf_ref.template_id else {
        return Vec::new();
    };
    let templates = state.workflow_templates.read();
    let Some(bindings) = templates
        .iter()
        .find(|t| t.template_id == template_id)
        .map(|t| t.stage_bindings("outline_expand"))
    else {
        return Vec::new();
    };
    bindings
        .get("cards")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// 从项目工作流引用解析细纲展开的绑定模型（覆盖 > 模板绑定）
fn resolve_bound_expand_model(state: &AppState) -> Option<String> {
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) else {
        return None;
    };
    if let Some(m) = wf_ref
        .overrides
        .get("outline_expand")
        .and_then(|v| v.get("model"))
        .and_then(|v| v.as_str())
        .filter(|m| !m.trim().is_empty())
    {
        return Some(m.to_string());
    }
    let template_id = wf_ref.template_id?;
    let templates = state.workflow_templates.read();
    let Some(bindings) = templates
        .iter()
        .find(|t| t.template_id == template_id)
        .map(|t| t.stage_bindings("outline_expand"))
    else {
        return None;
    };
    bindings
        .get("model")
        .and_then(|v| v.as_str())
        .filter(|m| !m.trim().is_empty())
        .map(|m| m.to_string())
}

/// 解析 LLM 产出的细纲 JSON 数组（先严格解析，失败则走容错修复）
fn parse_beat_plans(raw: &str) -> Result<Vec<BeatPlan>, String> {
    let text = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<Vec<BeatPlan>>(text) {
        return Ok(v);
    }
    let v = json_fix::repair_to_value(text).map_err(|e| format!("细纲 JSON 解析失败: {e}"))?;
    // 模型可能包一层对象（如 {"chapters": [...]}），取第一个数组值
    let arr = match &v {
        serde_json::Value::Array(_) => v,
        serde_json::Value::Object(m) => m
            .values()
            .find(|x| x.is_array())
            .cloned()
            .ok_or_else(|| "细纲 JSON 中找不到章节数组".to_string())?,
        _ => return Err("细纲 JSON 不是数组结构".to_string()),
    };
    serde_json::from_value::<Vec<BeatPlan>>(arr).map_err(|e| format!("细纲条目解析失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_beat_plans_plain_array() {
        let raw = r#"[{"title": "井边惊魂", "summary": "主角发现尸体"}, {"title": "初探", "summary": "展开调查"}]"#;
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].title, "井边惊魂");
    }

    #[test]
    fn test_parse_beat_plans_with_fence_and_prose() {
        let raw = "好的，以下是细纲：\n```json\n[{\"title\": \"a\", \"summary\": \"b\"}]\n```\n希望有帮助。";
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 1);
    }

    #[test]
    fn test_parse_beat_plans_wrapped_object() {
        let raw = r#"{"chapters": [{"title": "a", "summary": "b"}]}"#;
        let plans = parse_beat_plans(raw).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].summary, "b");
    }

    #[test]
    fn test_parse_beat_plans_invalid() {
        assert!(parse_beat_plans("这不是 JSON").is_err());
    }
}
