//! 阶段执行器：组装 prompt → 调 LLM → 解析 → 效果落库。
//!
//! 每个阶段返回 (给引擎判门控的 signal JSON, 审查 issues)；
//! LLM 调用经过 `call_interruptible`，停止旗标可立即中断。
use tauri::AppHandle;

use pensoul_core::{ChapterId, ChapterStatus};

use crate::integration;
use crate::state::AppState;

use super::stages::{self, STAGE_INJECTION, STAGE_REVIEW, STAGE_WRITING};
use super::{ModelCtx, PipelineEvent, call_interruptible, context, emit, emit_simple};

/// 执行单个阶段，返回 (signal, issues)
pub(super) async fn execute_stage(
    app: &AppHandle,
    state: &AppState,
    ctx: &ModelCtx,
    stage: &str,
    chapter_id: &ChapterId,
    prev_issues: &[String],
) -> Result<(serde_json::Value, Vec<String>), String> {
    // 取最新章节快照（写作落库后内容会变化）
    let chapter = {
        let onto = state.ontology.read();
        onto.get_chapter(chapter_id)
            .cloned()
            .ok_or_else(|| "章节不存在".to_string())?
    };

    match stage {
        STAGE_WRITING => {
            let memo_ctx = state.harness.read().memo.to_context_string();
            let packet = state.memory.read().build_packet(chapter.chapter_no);
            let onto = state.ontology.read().clone();
            let prompt = context::build_writing_prompt(
                &onto,
                &memo_ctx,
                &chapter,
                &packet,
                prev_issues,
                &ctx.writing_cards,
            );
            let raw = call_interruptible(state, ctx, &ctx.writing_model, &prompt, 0.85).await?;
            let content = stages::parse_writing_output(&raw);
            let word_count = content.chars().count();
            if word_count < 50 {
                return Err("模型输出过短，疑似未正常生成正文".to_string());
            }
            // 效果落库：正文写入本体 + 记忆/影响图增量更新
            {
                let mut onto = state.ontology.write();
                if let Some(ch) = onto
                    .chapters
                    .iter_mut()
                    .find(|c| c.chapter_id == *chapter_id)
                {
                    ch.content = content;
                    ch.word_count = word_count as u32;
                    ch.version += 1;
                    ch.status = ChapterStatus::Reviewing;
                    ch.updated_at = chrono::Utc::now().to_rfc3339();
                }
            }
            state.save().map_err(|e| format!("项目落盘失败: {e}"))?;
            integration::on_chapter_saved(state, chapter_id);
            let preview: String = {
                let onto = state.ontology.read();
                onto.get_chapter(chapter_id)
                    .map(|c| c.content.chars().take(300).collect::<String>())
                    .unwrap_or_default()
            };
            emit_simple(
                app,
                state,
                &chapter,
                stage,
                "llm_output",
                format!("正文已生成（{word_count} 字）：\n{preview}…"),
            );
            Ok((serde_json::json!({"word_count": word_count}), vec![]))
        }

        STAGE_REVIEW => {
            let recent = state
                .harness
                .read()
                .memo
                .get("recent_chapters")
                .unwrap_or("")
                .to_string();
            let onto = state.ontology.read().clone();
            let prompt = context::build_review_prompt(&onto, &chapter, &recent, &ctx.review_cards);
            let raw = call_interruptible(state, ctx, &ctx.review_model, &prompt, 0.3).await?;
            let (signal, report) = stages::parse_review_output(&raw)?;
            emit(
                app,
                state,
                PipelineEvent {
                    seq: 0,
                    chapter_id: chapter.chapter_id.to_string(),
                    chapter_title: chapter.title.clone(),
                    stage: stage.to_string(),
                    kind: "review_report".to_string(),
                    status: "info".to_string(),
                    content: report,
                    score: Some(signal.score),
                    attempt: 0,
                },
            );
            let issues = signal.issues.clone();
            Ok((
                serde_json::json!({"score": signal.score, "issues": signal.issues}),
                issues,
            ))
        }

        STAGE_INJECTION => {
            let prompt = context::build_injection_prompt(&chapter);
            let raw = call_interruptible(state, ctx, &ctx.writing_model, &prompt, 0.3).await?;
            let brief = stages::parse_injection_output(&raw, &chapter.content);
            // 滚动备忘录：recent_chapters 保留最近 3 章纪要
            {
                let mut engine = state.harness.write();
                let mut arr: Vec<serde_json::Value> = engine
                    .memo
                    .get("recent_chapters")
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                arr.push(serde_json::json!({
                    "no": chapter.chapter_no,
                    "title": chapter.title,
                    "brief": brief,
                }));
                while arr.len() > 3 {
                    arr.remove(0);
                }
                let _ = engine.inject_memo(
                    "recent_chapters",
                    &serde_json::Value::Array(arr).to_string(),
                );
            }
            emit_simple(
                app,
                state,
                &chapter,
                stage,
                "effect",
                format!("本章纪要已回灌备忘录：{brief}"),
            );
            Ok((serde_json::json!({"brief": brief}), vec![]))
        }

        other => Err(format!("未知阶段: {other}")),
    }
}

/// 更新章节状态并落盘（审查通过后置 Reviewed）
pub(super) fn set_chapter_status(state: &AppState, chapter_id: &ChapterId, status: ChapterStatus) {
    {
        let mut onto = state.ontology.write();
        if let Some(ch) = onto
            .chapters
            .iter_mut()
            .find(|c| c.chapter_id == *chapter_id)
        {
            ch.status = status;
            ch.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }
    let _ = state.save();
}
