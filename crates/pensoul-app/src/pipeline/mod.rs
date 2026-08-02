//! 章节连写管线编排器 —— 把 Harness 引擎、记忆管道、LLM 调用串成闭环。
//!
//! 流程：选章（有梗概、无正文，按 chapter_no 升序）→ 逐章执行
//! 写作(auto) → 审查(conditional, 异模型) → 回灌(auto) 三阶段，
//! 引擎负责门控/回退/熔断/WAL，本模块负责上下文组装、模型调用、效果落库。
//! 控制面：暂停（阶段边界自旋）、停止（select! 立即中断 LLM）、续写（再跑一次即可）。
pub mod context;
pub mod stages;

mod executor;
pub(crate) mod runner;

pub use runner::run_pipeline;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use pensoul_core::Chapter;

use crate::commands::llm_helper as lh;
use crate::state::AppState;

/// 「停止」哨兵错误：select! 被 notify 抢先时返回，用于区分正常失败
pub(crate) const STOP_ERR: &str = "__pipeline_stopped__";

/// 事件缓冲区上限（环形：满了丢最旧），供页面切换后重放
const EVENT_BUFFER_CAP: usize = 200;

/// 管线控制面（挂 AppState，跨命令共享）
pub struct PipelineControl {
    /// 是否有管线正在运行（防重入）
    pub running: AtomicBool,
    /// 暂停旗标：阶段边界自旋等待
    pub paused: AtomicBool,
    /// 停止旗标：LLM 调用立即中断
    pub stop: AtomicBool,
    /// 停止信号通知器
    pub notify: tokio::sync::Notify,
    /// 当前正在写作的章节标题（给状态查询用）
    pub current_chapter: RwLock<Option<String>>,
    /// 事件环形缓冲：页面切换后前端可重放恢复现场
    pub events: RwLock<Vec<PipelineEvent>>,
    /// 事件单调序号（前端去重快照与实时事件用）
    pub seq: AtomicU64,
    /// 本次运行实际使用的写作/审查模型（恢复页面选择状态用）
    pub writing_model: RwLock<Option<String>>,
    pub review_model: RwLock<Option<String>>,
}

impl PipelineControl {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
            current_chapter: RwLock::new(None),
            events: RwLock::new(Vec::new()),
            seq: AtomicU64::new(0),
            writing_model: RwLock::new(None),
            review_model: RwLock::new(None),
        }
    }

    /// 新一轮运行开始时重置现场：清空事件缓冲与序号，记录实际使用的模型
    pub fn begin_run(&self, writing: &str, review: &str) {
        self.events.write().clear();
        self.seq.store(0, Ordering::SeqCst);
        *self.writing_model.write() = Some(writing.to_string());
        *self.review_model.write() = Some(review.to_string());
    }

    /// 事件入缓冲并赋序号（发射前调用）
    fn record(&self, ev: &mut PipelineEvent) {
        ev.seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut buf = self.events.write();
        if buf.len() >= EVENT_BUFFER_CAP {
            buf.remove(0);
        }
        buf.push(ev.clone());
    }

    /// 状态快照（get_pipeline_state 用）：运行旗标 + 事件缓冲 + 模型选择
    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "running": self.running.load(Ordering::SeqCst),
            "paused": self.paused.load(Ordering::SeqCst),
            "current_chapter": self.current_chapter.read().clone(),
            "events": self.events.read().clone(),
            "writing_model": self.writing_model.read().clone(),
            "review_model": self.review_model.read().clone(),
        })
    }
}

impl Default for PipelineControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 推送给前端的管线事件（`harness-event`）
#[derive(Debug, Clone, Serialize)]
pub struct PipelineEvent {
    /// 单调序号：0 表示未入缓冲（构造默认值，emit 时覆写）
    pub seq: u64,
    pub chapter_id: String,
    pub chapter_title: String,
    pub stage: String,
    /// stage_start | llm_output | review_report | gate | effect |
    /// chapter_start | chapter_done | chapter_failed | paused | resumed | pipeline_done
    pub kind: String,
    pub status: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    pub attempt: u32,
}

/// 发射事件：先入控制面缓冲（供页面重放），再推送给前端
pub(crate) fn emit(app: &AppHandle, state: &AppState, mut ev: PipelineEvent) {
    state.pipeline.record(&mut ev);
    let _ = app.emit("harness-event", ev);
}

pub(crate) fn emit_simple(
    app: &AppHandle,
    state: &AppState,
    chapter: &Chapter,
    stage: &str,
    kind: &str,
    content: String,
) {
    emit(
        app,
        state,
        PipelineEvent {
            seq: 0,
            chapter_id: chapter.chapter_id.to_string(),
            chapter_title: chapter.title.clone(),
            stage: stage.to_string(),
            kind: kind.to_string(),
            status: "info".to_string(),
            content,
            score: None,
            attempt: 0,
        },
    );
}

/// 模型解析上下文（避免在函数间传 5 个 HashMap）
pub(crate) struct ModelCtx {
    pub m2p: HashMap<String, String>,
    pub bases: HashMap<String, String>,
    pub keys: HashMap<String, String>,
    pub writing_model: String,
    pub review_model: String,
    /// 写作阶段绑定的技法卡注入块（工作流配置，可为空）
    pub writing_cards: String,
    /// 审查阶段绑定的技法卡注入块（可为空）
    pub review_cards: String,
    /// 审查阶段是否启用「黄金三章」硬门控（模板 review 环节声明时开启）
    pub golden_review: bool,
}

impl ModelCtx {
    /// 解析供应商并调用 LLM（照抄 discussion.rs 的 call_with_system 模式）
    /// `light` 为 true 时按模型档案关闭/降低思考（评审、纪要等结构任务）
    pub async fn call(
        &self,
        model: &str,
        system: &str,
        user: &str,
        temperature: f64,
        max_tokens: u32,
        light: bool,
    ) -> Result<String, String> {
        let (provider_id, api_key, api_base) =
            lh::resolve_provider(model, &self.m2p, &self.bases, &self.keys)?;
        lh::call_llm_task(
            &lh::ProviderAuth {
                provider_id: &provider_id,
                api_key: &api_key,
                api_base: &api_base,
            },
            model,
            system,
            user,
            temperature,
            max_tokens,
            if light {
                crate::llm_profile::LlmTask::Light
            } else {
                crate::llm_profile::LlmTask::Deep
            },
        )
        .await
    }
}

/// 带停止中断的 LLM 调用：stop 旗标置位时 notify 抢先返回哨兵错误
pub(crate) async fn call_interruptible(
    state: &AppState,
    ctx: &ModelCtx,
    model: &str,
    prompt: &context::StagePrompt,
    temperature: f64,
) -> Result<String, String> {
    let call = ctx.call(
        model,
        &prompt.system,
        &prompt.user,
        temperature,
        prompt.max_tokens,
        prompt.light,
    );
    tokio::select! {
        out = call => out,
        _ = state.pipeline.notify.notified() => Err(STOP_ERR.to_string()),
    }
}

/// 阶段显示名
pub(crate) fn stage_display(stage: &str) -> &'static str {
    match stage {
        stages::STAGE_PLANNING => "章前策划",
        stages::STAGE_WRITING => "章节写作",
        stages::STAGE_REVIEW => "一致性审查",
        stages::STAGE_INJECTION => "状态回灌",
        _ => "未知阶段",
    }
}
