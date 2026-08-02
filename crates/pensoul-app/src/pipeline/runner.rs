//! 管线运行循环：选章 → 注册阶段 → 逐章驱动引擎（含暂停/停止控制面）。
use std::sync::atomic::Ordering;

use tauri::AppHandle;
use tokio::time::{Duration, sleep};

use pensoul_core::workflow::{WorkflowRef, WorkflowTemplate};
use pensoul_core::{Chapter, ChapterStatus, StageName};
use pensoul_harness::{StageInstance, StageStatus};

use crate::commands::llm_helper as lh;
use crate::state::AppState;

use super::executor::{execute_stage, set_chapter_status};
use super::stages::{self, STAGE_INJECTION, STAGE_REVIEW, STAGE_WRITING};
use super::{ModelCtx, PipelineEvent, STOP_ERR, emit, emit_simple, stage_display};

/// 单章内层循环安全上限（写作→审查→重写 轮回，正常最多 ~8 次）
const MAX_STAGE_ITERATIONS: usize = 12;
/// LLM 执行失败（网络/解析）在单个阶段内的最大重试次数
const MAX_EXEC_RETRIES: u32 = 2;

/// 解析项目引用的全局工作流模板（未配置/解析失败返回 None）。
fn resolve_project_workflow(state: &AppState) -> Option<WorkflowTemplate> {
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    let wf_ref: WorkflowRef = serde_json::from_value(ref_json).ok()?;
    let template_id = wf_ref.template_id?;
    let templates = state.workflow_templates.read();
    templates
        .iter()
        .find(|t| t.template_id == template_id)
        .cloned()
}

/// 解析某环节的技法卡：显式参数 > 项目覆盖 > 模板绑定 > 空
fn resolve_stage_cards(
    state: &AppState,
    template: Option<&WorkflowTemplate>,
    explicit: Option<&Vec<String>>,
    stage: &str,
) -> Vec<String> {
    if let Some(cards) = explicit
        && !cards.is_empty()
    {
        return cards.clone();
    }
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    if let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) {
        let overridden = wf_ref
            .overrides
            .get(stage)
            .and_then(|v| v.get("cards"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });
        if let Some(cards) = overridden
            && !cards.is_empty()
        {
            return cards;
        }
    }
    if let Some(tpl) = template {
        return tpl
            .stage_bindings(stage)
            .get("cards")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }
    Vec::new()
}

/// 解析某环节的模型：显式参数 > 项目覆盖 > 模板绑定 > None
fn resolve_stage_model(
    state: &AppState,
    template: Option<&WorkflowTemplate>,
    explicit: Option<&str>,
    stage: &str,
) -> Option<String> {
    if let Some(m) = explicit.filter(|m| !m.trim().is_empty()) {
        return Some(m.to_string());
    }
    let ref_json = {
        let onto = state.ontology.read();
        onto.workflow_ref.clone()
    };
    if let Ok(wf_ref) = serde_json::from_value::<WorkflowRef>(ref_json) {
        if let Some(m) = wf_ref
            .overrides
            .get(stage)
            .and_then(|v| v.get("model"))
            .and_then(|v| v.as_str())
            .filter(|m| !m.trim().is_empty())
        {
            return Some(m.to_string());
        }
    }
    if let Some(tpl) = template {
        if let Some(m) = tpl
            .stage_bindings(stage)
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|m| !m.trim().is_empty())
        {
            return Some(m.to_string());
        }
    }
    None
}

/// 解析写作/审查模型：显式参数 > 项目覆盖/模板绑定 > 第一个可用模型。
/// 审查模型尽量与写作不同（学生不自己判卷），没有第二个就复用同一个。
fn resolve_models(
    state: &AppState,
    writing_model: Option<String>,
    review_model: Option<String>,
    template: Option<&WorkflowTemplate>,
) -> Result<(String, String), String> {
    let providers = lh::load_providers(state);
    let models = lh::load_models(state);
    let keys = state.api_keys.read().clone();

    // 所有「provider 有 Key」的可用模型 ID 列表
    let mut available: Vec<String> = models
        .iter()
        .filter_map(|m| {
            let model_id = m.get("model_id")?.as_str()?.to_string();
            let provider_id = m.get("provider_id")?.as_str()?;
            keys.contains_key(provider_id).then_some(model_id)
        })
        .collect();

    // models.json 为空时，从 provider 兜底推断一个模型名
    if available.is_empty()
        && let Some((pid, _, _)) = lh::find_any_available_provider(&providers, &keys)
    {
        let fallback = match pid.as_str() {
            "deepseek" => "deepseek-chat",
            "moonshot" => "moonshot-v1-8k",
            "anthropic" => "claude-sonnet-4-20250514",
            _ => "gpt-4o-mini",
        };
        available.push(fallback.to_string());
    }

    let writing =
        match resolve_stage_model(state, template, writing_model.as_deref(), STAGE_WRITING) {
            Some(m) => m,
            None => available.first().cloned().ok_or_else(|| {
                "未配置可用模型。请先在「模型设置」添加模型并配置 API Key。".to_string()
            })?,
        };
    let review = match resolve_stage_model(state, template, review_model.as_deref(), STAGE_REVIEW) {
        Some(m) => m,
        None => available
            .iter()
            .find(|m| **m != writing)
            .cloned()
            .unwrap_or_else(|| writing.clone()),
    };
    Ok((writing, review))
}

/// 选章：显式 ids 优先；缺省 = 有梗概且无正文的章节，按 chapter_no 升序
fn select_chapters(state: &AppState, chapter_ids: Option<Vec<String>>) -> Vec<Chapter> {
    let onto = state.ontology.read();
    let mut list: Vec<Chapter> = match chapter_ids {
        Some(ids) => onto
            .chapters
            .iter()
            .filter(|c| ids.iter().any(|id| id == c.chapter_id.as_str()))
            .cloned()
            .collect(),
        None => onto
            .chapters
            .iter()
            .filter(|c| !c.summary.trim().is_empty() && c.word_count == 0)
            .cloned()
            .collect(),
    };
    list.sort_by_key(|c| c.chapter_no);
    list
}

/// 管线主流程（async 长跑，事件通过 harness-event 实时推送）
/// `writing_cards` / `review_cards`：工作流为写作/审查环节绑定的技法卡 SKILL.md 路径
pub async fn run_pipeline(
    app_handle: AppHandle,
    state: AppState,
    chapter_ids: Option<Vec<String>>,
    writing_model: Option<String>,
    review_model: Option<String>,
    writing_cards: Option<Vec<String>>,
    review_cards: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    // 防重入：已有管线在跑直接报错
    if state.pipeline.running.swap(true, Ordering::SeqCst) {
        return Err("已有写作管线在运行中".to_string());
    }
    state.pipeline.paused.store(false, Ordering::SeqCst);
    state.pipeline.stop.store(false, Ordering::SeqCst);

    let result = run_pipeline_inner(
        &app_handle,
        &state,
        chapter_ids,
        writing_model,
        review_model,
        writing_cards,
        review_cards,
    )
    .await;

    state.pipeline.running.store(false, Ordering::SeqCst);
    state.pipeline.paused.store(false, Ordering::SeqCst);
    *state.pipeline.current_chapter.write() = None;
    result
}

async fn run_pipeline_inner(
    app: &AppHandle,
    state: &AppState,
    chapter_ids: Option<Vec<String>>,
    writing_model: Option<String>,
    review_model: Option<String>,
    writing_cards: Option<Vec<String>>,
    review_cards: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    lh::ensure_api_keys_loaded(state);
    let template = resolve_project_workflow(state);
    let (writing, review) = resolve_models(state, writing_model, review_model, template.as_ref())?;
    // 新一轮运行：清空事件缓冲，记录实际使用的模型（供页面恢复）
    state.pipeline.begin_run(&writing, &review);
    // 技法卡：显式参数（造化工坊页面）优先，否则按项目引用模板 + 覆盖解析
    let resolved_writing_cards = resolve_stage_cards(
        state,
        template.as_ref(),
        writing_cards.as_ref(),
        STAGE_WRITING,
    );
    let resolved_review_cards = resolve_stage_cards(
        state,
        template.as_ref(),
        review_cards.as_ref(),
        STAGE_REVIEW,
    );
    // 黄金三章硬门控：模板 review 环节声明后，前 3 章额外要求钩子/爽点维度达标
    let golden_review = template
        .as_ref()
        .and_then(|t| t.find_stage(STAGE_REVIEW))
        .map(|d| d.golden_gate)
        .unwrap_or(false);
    let ctx = ModelCtx {
        m2p: lh::build_model_to_provider(&lh::load_models(state)),
        bases: lh::build_provider_api_bases(&lh::load_providers(state)),
        keys: state.api_keys.read().clone(),
        writing_model: writing.clone(),
        review_model: review.clone(),
        writing_cards: crate::commands::book_distill::load_writing_cards(
            state,
            &resolved_writing_cards,
        ),
        review_cards: crate::commands::book_distill::load_writing_cards(
            state,
            &resolved_review_cards,
        ),
        golden_review,
    };

    let chapters = select_chapters(state, chapter_ids);
    if chapters.is_empty() {
        return Err(
            "没有待写的章节：请先在「大纲」展开情节脉络的细纲，或为章节填写梗概（正文为空的章节才会进入写作队列）"
                .to_string(),
        );
    }

    // 注册阶段模板（默认三阶段；模板声明章前策划时为四阶段）+ 注入创作备忘录
    let stage_names: Vec<String> = stages::pipeline_stages(template.as_ref())
        .iter()
        .map(|s| s.name.as_str().to_string())
        .collect();
    {
        let onto = state.ontology.read();
        let mut engine = state.harness.write();
        for stage in stages::pipeline_stages(template.as_ref()) {
            engine.register_stage(stage);
        }
        let settings_memo = serde_json::json!({
            "genre": onto.settings.genre,
            "chapter_target_words": onto.settings.chapter_target_words,
            "target_chapters": onto.settings.target_chapters,
        });
        let _ = engine.inject_memo("creation_settings", &settings_memo.to_string());
        let concept = &onto.core_concept;
        if !concept.high_concept.is_empty() {
            let _ = engine.inject_memo(
                "core_concept",
                &format!(
                    "高概念：{}；前提：{}；主角：{}；基调：{}；核心冲突：{}",
                    concept.high_concept,
                    concept.premise,
                    concept.protagonist_hint,
                    concept.tone,
                    concept.central_conflict
                ),
            );
        }
    }

    let total = chapters.len();
    let mut completed = 0usize;
    let mut failed_titles: Vec<String> = Vec::new();
    let mut stopped = false;
    // 审查放行阈值：模板可自定义（默认 80）
    let review_pass_score = template
        .as_ref()
        .map(|t| t.review_pass_score)
        .unwrap_or(80.0);
    // 审查阶段定义快照：每章按黄金三章开关重设门控条件后重新注册
    let mut review_stage = stages::pipeline_stages(template.as_ref())
        .into_iter()
        .find(|s| s.name.as_str() == STAGE_REVIEW);

    for (idx, chapter) in chapters.iter().enumerate() {
        if state.pipeline.stop.load(Ordering::SeqCst) {
            stopped = true;
            break;
        }
        *state.pipeline.current_chapter.write() = Some(chapter.title.clone());
        emit_simple(
            app,
            state,
            chapter,
            "",
            "chapter_start",
            format!(
                "开始写作第 {} 章《{}》（{}/{total}）",
                chapter.chapter_no,
                chapter.title,
                idx + 1
            ),
        );

        // 每章开始前重置各阶段实例状态，起始阶段取编排的第一个（含章前策划）
        {
            let mut engine = state.harness.write();
            // 黄金三章硬门控：前 3 章要求总分达标 且 钩子/爽点 ≥ 8；其余章节仅总分达标
            if let Some(rev) = review_stage.as_mut() {
                let golden_chapter = golden_review && chapter.chapter_no <= 3;
                rev.gate_condition = Some(if golden_chapter {
                    format!("score >= {review_pass_score} && hook >= 8 && payoff >= 8")
                } else {
                    format!("score >= {review_pass_score}")
                });
                engine.register_stage(rev.clone());
            }
            for name in &stage_names {
                engine
                    .stages_status
                    .insert(name.to_string(), StageInstance::new(StageName::new(name)));
            }
            let first = stage_names
                .first()
                .cloned()
                .unwrap_or_else(|| STAGE_WRITING.to_string());
            engine.set_current_stage(StageName::new(first));
        }

        let mut prev_issues: Vec<String> = Vec::new();
        let mut chapter_failed = false;
        let mut fail_reason = String::new();

        for _ in 0..MAX_STAGE_ITERATIONS {
            // ── 停止：立即中断 ──
            if state.pipeline.stop.load(Ordering::SeqCst) {
                stopped = true;
                break;
            }
            // ── 暂停：阶段边界自旋等待 ──
            if state.pipeline.paused.load(Ordering::SeqCst) {
                emit_simple(
                    app,
                    state,
                    chapter,
                    "",
                    "paused",
                    "已暂停，等待继续".to_string(),
                );
                while state.pipeline.paused.load(Ordering::SeqCst)
                    && !state.pipeline.stop.load(Ordering::SeqCst)
                {
                    sleep(Duration::from_millis(500)).await;
                }
                if state.pipeline.stop.load(Ordering::SeqCst) {
                    stopped = true;
                    break;
                }
                emit_simple(app, state, chapter, "", "resumed", "继续写作".to_string());
            }

            let stage_name = match state.harness.read().current_stage().cloned() {
                Some(n) => n,
                None => break,
            };
            let stage_str = stage_name.as_str().to_string();

            // start_stage（WAL StageStart）
            let attempt = {
                let mut engine = state.harness.write();
                match engine.start_stage() {
                    Ok(inst) => inst.attempt,
                    Err(e) => {
                        chapter_failed = true;
                        fail_reason = format!("阶段启动失败: {e}");
                        break;
                    }
                }
            };
            emit_simple(
                app,
                state,
                chapter,
                &stage_str,
                "stage_start",
                format!(
                    "阶段「{}」开始（第 {attempt} 次尝试）",
                    stage_display(&stage_str)
                ),
            );

            // 执行阶段（LLM 调用 + 解析 + 效果落库），执行失败重试 MAX_EXEC_RETRIES 次
            let mut exec_attempt = 0u32;
            let (signal, issues) = loop {
                exec_attempt += 1;
                match execute_stage(
                    app,
                    state,
                    &ctx,
                    &stage_str,
                    &chapter.chapter_id,
                    &prev_issues,
                )
                .await
                {
                    Ok(v) => break v,
                    Err(e) if e == STOP_ERR => {
                        stopped = true;
                        break (serde_json::Value::Null, vec![]);
                    }
                    Err(e) if exec_attempt <= MAX_EXEC_RETRIES => {
                        emit_simple(
                            app,
                            state,
                            chapter,
                            &stage_str,
                            "llm_output",
                            format!("执行失败（{e}），{exec_attempt}/{MAX_EXEC_RETRIES} 次重试…"),
                        );
                    }
                    Err(e) => {
                        chapter_failed = true;
                        fail_reason = e;
                        break (serde_json::Value::Null, vec![]);
                    }
                }
            };
            if stopped || chapter_failed {
                break;
            }

            // 审查阶段记录 issues 供重写注入
            if stage_str == STAGE_REVIEW {
                prev_issues = issues;
            }

            // complete_stage → 引擎门控判定（推进/回退/熔断）
            {
                let mut engine = state.harness.write();
                if let Err(e) = engine.complete_stage(signal) {
                    chapter_failed = true;
                    fail_reason = format!("门控判定失败: {e}");
                    break;
                }
            }

            // 读取门控结果发 gate 事件；审查通过则章节状态置 Reviewed
            let (gate_reason, gate_score, injection_done, review_failed) = {
                let engine = state.harness.read();
                let cur = engine
                    .stages_status
                    .get(&stage_str)
                    .and_then(|i| i.gate_result.clone());
                let inj_done = engine
                    .stages_status
                    .get(STAGE_INJECTION)
                    .map(|i| i.status == StageStatus::Completed)
                    .unwrap_or(false);
                let rev_failed = engine
                    .stages_status
                    .get(STAGE_REVIEW)
                    .map(|i| i.status == StageStatus::Failed)
                    .unwrap_or(false);
                (
                    cur.as_ref().map(|g| g.reason.clone()).unwrap_or_default(),
                    cur.as_ref().and_then(|g| g.score),
                    inj_done,
                    rev_failed,
                )
            };
            emit(
                app,
                state,
                PipelineEvent {
                    seq: 0,
                    chapter_id: chapter.chapter_id.to_string(),
                    chapter_title: chapter.title.clone(),
                    stage: stage_str.clone(),
                    kind: "gate".to_string(),
                    status: "info".to_string(),
                    content: gate_reason,
                    score: gate_score,
                    attempt,
                },
            );
            if stage_str == STAGE_REVIEW
                && gate_score.map(|s| s >= review_pass_score).unwrap_or(false)
            {
                set_chapter_status(state, &chapter.chapter_id, ChapterStatus::Reviewed);
            }
            if injection_done {
                break; // 本章闭环完成
            }
            if review_failed {
                chapter_failed = true;
                fail_reason =
                    "审查超过最大重试次数，已熔断（可在笔耕手动修改后重新送审）".to_string();
                break;
            }
            // 引擎已回退到写作阶段，带 issues 继续下一轮
        }

        if stopped {
            break;
        }
        if chapter_failed {
            failed_titles.push(chapter.title.clone());
            emit_simple(app, state, chapter, "", "chapter_failed", fail_reason);
        } else {
            completed += 1;
            emit_simple(
                app,
                state,
                chapter,
                "",
                "chapter_done",
                format!("第 {} 章《{}》写作完成", chapter.chapter_no, chapter.title),
            );
        }
    }

    emit(
        app,
        state,
        PipelineEvent {
            seq: 0,
            chapter_id: String::new(),
            chapter_title: String::new(),
            stage: String::new(),
            kind: "pipeline_done".to_string(),
            status: if stopped {
                "stopped".into()
            } else {
                "done".into()
            },
            content: format!(
                "管线结束：完成 {completed}/{total} 章{}",
                if failed_titles.is_empty() {
                    String::new()
                } else {
                    format!("，失败：{}", failed_titles.join("、"))
                }
            ),
            score: None,
            attempt: 0,
        },
    );

    Ok(serde_json::json!({
        "completed": completed,
        "failed": failed_titles,
        "stopped": stopped,
        "total": total,
    }))
}
