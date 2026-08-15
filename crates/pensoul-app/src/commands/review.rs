// review.rs — AI 章节审校（调研 F3 完整版 / F4 / F8）
// 双层审校：
// - 本地启发式（无需 LLM）：元叙述、反 AI 味命中、说教密度（show-don't-tell）
// - 可选 LLM 结构化审校：硬约束合规、实体矛盾（与正典比对）、失败模式、改进建议
// 审校为建议制：只返回报告，不修改正文、不落盘。
// LLM 未配置或调用失败时自动降级为「本地模式」，保证审校始终可用。

use axum::extract::{Form, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::llm::{build_llm_request, llm_client, structured_output_tokens};
use crate::commands::techniques;
use crate::commands::writing::anti_slop_scan;
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::chapter::Chapter;
use pensoul_domain::ontology::NovelOntology;
use pensoul_infra::llm::LlmMessage;

#[derive(Deserialize)]
pub struct ReviewParams {
    pub chapter_id: String,
    /// 可选：审校指定正文；缺省时审校正典中已保存的正文
    pub content: Option<String>,
    /// 可选：技巧 id（按技巧检查项审校）
    pub technique_ids: Option<String>,
}

/// 本地启发式检测结果（无需 LLM）
#[derive(Debug, Clone, Serialize, Default)]
pub struct LocalReport {
    pub char_count: usize,
    /// 元叙述命中（如「本章」「故事开始」）
    pub meta_narration_hits: Vec<String>,
    /// 高频 AI 味表达命中（复用反 AI 味词表）
    pub anti_slop_hits: Vec<String>,
    /// 每千字「说教式表达」（感到/觉得/意识到/仿佛/似乎）计数
    pub tell_density: f32,
    /// 说教式表达关键词统计
    pub tell_counts: Vec<serde_json::Value>,
}

/// LLM 结构化审校报告（需配置默认 LLM）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmReport {
    pub hard_constraint_issues: Vec<String>,
    pub entity_conflicts: Vec<String>,
    pub failure_modes: Vec<serde_json::Value>,
    pub suggestions: Vec<String>,
}

/// 元叙述命中词（与生成提示词的禁止项一致）
const META_NARRATION_WORDS: &[&str] = &["本章", "这一章", "故事开始", "从……开始", "从...开始", "接下来本章", "全书"];

/// 说教式表达（show-don't-tell 反面）：情绪/状态被直接贴标签
const TELL_WORDS: &[&str] = &["感到", "觉得", "意识到", "仿佛", "似乎", "看起来"];

/// 审校章节（建议制，不写正典）
pub async fn review(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<ReviewParams>,
) -> Result<String, ApiError> {
    let (chapter, ontology, content, technique_ids, base_dir, project_id) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let project_id = ontology.project_id.as_str().to_string();
        let chapter = ontology
            .chapters_in_order()
            .into_iter()
            .find(|c| c.chapter_id.to_string() == params.chapter_id)
            .cloned()
            .ok_or(ApiError::not_found("章节不存在"))?;
        let content = match params.content {
            Some(text) if !text.trim().is_empty() => text,
            _ => chapter.content.clone(),
        };
        let technique_ids: Vec<String> = params
            .technique_ids
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        (chapter, ontology.clone(), content, technique_ids, state.base_dir.clone(), project_id)
    };

    if content.trim().is_empty() {
        return Err(ApiError::bad_request(
            "章节正文为空，无法审校。请先写入正文。",
        ));
    }

    // 1. 本地启发式（始终执行）
    let local = local_report(&content);

    // 2. 尝试 LLM 结构化审校；未配置/失败自动降级为本地模式，原因通过 llm_error 显式返回
    let technique_checks = techniques::check_items_for(&technique_ids);
    let mut llm_error: Option<String> = None;
    // P0b：审校 Agent 按角色解析模型（未绑定回退全局默认；失败自动降级本地模式）
    let llm = match crate::commands::agent::resolve(&base_dir, crate::commands::agent::AgentRole::Reviewer) {
        Ok(provider) => {
            let client = llm_client(&provider);
            // P3：风格配方作为审校维度（「这段不符合配方的 X 特征」）；配方为作品级（按项目隔离）
            let style_recipe = crate::commands::distill::load_style_recipe(&base_dir, &project_id)
                .map(|r| crate::commands::distill::recipe_injection_text(&r));
            let system = review_system_prompt(
                &ontology,
                &chapter,
                &technique_checks,
                style_recipe.as_deref(),
            );
            let request = build_llm_request(
                &provider,
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: truncate_chars(&content, 8000),
                }],
                system,
                true,
                structured_output_tokens(&provider, 4096, 16000),
            );
            match client.complete(request).await {
                Ok(resp) => match pensoul_infra::llm::parse_llm_json::<LlmReport>(&resp.content) {
                    Ok(report) => Some(report),
                    Err(e) => {
                        llm_error = Some(format!("审校响应解析失败（{e}），已降级为本地模式"));
                        None
                    }
                },
                Err(e) => {
                    llm_error = Some(format!("审校调用失败（{e}），已降级为本地模式"));
                    None
                }
            }
        }
        Err(e) => {
            llm_error = Some(format!("未配置审校模型（{e}），已降级为本地模式"));
            None
        }
    };

    let mode = if llm.is_some() { "full" } else { "local" };
    let (hit, _unknown) = techniques::resolve(&technique_ids);
    let techniques_checked: Vec<String> = hit.iter().map(|t| t.id.to_string()).collect();

    serde_json::to_string(&serde_json::json!({
        "mode": mode,
        "chapter_id": chapter.chapter_id,
        "local": local,
        "llm": llm,
        "techniques_checked": techniques_checked,
        "llm_error": llm_error,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

// ---- 本地启发式 ----

fn local_report(text: &str) -> LocalReport {
    let char_count = text.chars().count();

    // 元叙述命中
    let meta_narration_hits: Vec<String> = META_NARRATION_WORDS
        .iter()
        .filter(|w| text.contains(**w))
        .map(|w| w.to_string())
        .collect();

    // 反 AI 味命中（复用生成侧词表）
    let anti_slop_hits = anti_slop_scan(text);

    // 说教密度：每千字命中数
    let mut tell_counts: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;
    for word in TELL_WORDS {
        let count = text.matches(word).count();
        if count > 0 {
            total += count;
            tell_counts.push(serde_json::json!({ "word": word, "count": count }));
        }
    }
    let tell_density = if char_count == 0 {
        0.0
    } else {
        (total as f32 / char_count as f32 * 1000.0 * 10.0).round() / 10.0
    };

    LocalReport {
        char_count,
        meta_narration_hits,
        anti_slop_hits,
        tell_density,
        tell_counts,
    }
}

// ---- LLM 审校提示词 ----

fn review_system_prompt(
    ontology: &NovelOntology,
    chapter: &Chapter,
    technique_checks: &[String],
    style_recipe: Option<&str>,
) -> String {
    let characters: Vec<_> = ontology
        .characters
        .characters
        .iter()
        .take(15)
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "personality": c.properties.personality,
                "wants": c.properties.wants,
                "fears": c.properties.fears,
                "secret": c.properties.secret,
            })
        })
        .collect();
    let foreshadows: Vec<_> = ontology
        .narrative
        .foreshadows
        .iter()
        .take(10)
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "status": format!("{:?}", f.status),
                "planted_chapter": f.planted_chapter,
            })
        })
        .collect();
    let checks = if technique_checks.is_empty() {
        "（未指定技巧检查项）".to_string()
    } else {
        technique_checks
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let style_block = style_recipe
        .map(|r| format!("\n风格配方对照（审校维度）：\n{r}"))
        .unwrap_or_default();

    format!(
        "你是 PenSoul 的资深审校编辑，负责长篇小说的章节质量把关。\n\
         请严格按下列维度审校用户提供的章节正文，只输出 JSON，不要任何解释：\n\
         {{\n\
           \"hard_constraint_issues\": [违反硬约束的问题，如与设定/时间线/伏笔矛盾，无则空数组],\n\
           \"entity_conflicts\": [与正典实体状态冲突之处（角色属性/位置/关系），无则空数组],\n\
           \"failure_modes\": [{{\"dimension\": \"失败模式维度\", \"severity\": \"高|中|低\", \"detail\": \"具体说明\"}}],\n\
           \"suggestions\": [可执行的改进建议]\n\
         }}\n\
         失败模式维度参考：说教代替展示(show-don't-tell)、角色漂移、对白生硬、情节可预测、隐喻滥用、紫文堆砌、节奏失衡。\n\
         \n\
         正典参考（JSON）：\n{}\n\
         本章：第 {} 章《{}》\n\
         章节摘要：{}\n\
         技巧检查项：\n{checks}{style_block}",
        serde_json::json!({
            "core_concept": ontology.core_concept,
            "world_rules": ontology.world.rules.iter().take(10).collect::<Vec<_>>(),
            "characters": characters,
            "active_foreshadows": foreshadows,
            "style_notes": ontology.aesthetic.style_notes,
            "pacing_notes": ontology.aesthetic.pacing_notes,
        }),
        chapter.chapter_no,
        chapter.title,
        if chapter.summary.is_empty() {
            "（无摘要）".to_string()
        } else {
            chapter.summary.clone()
        }
    )
}

/// 按字符截断，防止上下文超预算
fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut output: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        output.push_str("…（正文过长已截断）");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_report_detects_meta_narration() {
        let report = local_report("本章，故事开始。他推门走了进去。");
        assert!(report.meta_narration_hits.contains(&"本章".to_string()));
        assert!(report.meta_narration_hits.contains(&"故事开始".to_string()));
        assert!(report.char_count > 0);
    }

    #[test]
    fn local_report_counts_tell_density() {
        let report = local_report("他感到害怕，觉得前路迷茫，仿佛一切都没有意义。她看起来很累。");
        assert!(report.tell_density > 0.0);
        assert!(report.tell_counts.iter().any(|t| t["word"] == "感到"));
        assert!(report.tell_counts.iter().any(|t| t["word"] == "仿佛"));
    }

    #[test]
    fn local_report_detects_anti_slop() {
        let report = local_report("夜色静谧，她眸子深邃。");
        assert!(report.anti_slop_hits.contains(&"静谧".to_string()));
    }

    #[test]
    fn clean_text_has_empty_flags() {
        let report = local_report("他把药瓶放回架子上，转身出了门。店门外的灯笼还亮着。");
        assert!(report.meta_narration_hits.is_empty());
        assert!(report.anti_slop_hits.is_empty());
        assert_eq!(report.tell_density, 0.0);
    }

    #[test]
    fn empty_content_guards() {
        // review 命令层对空正文返回 400；此处仅验证 local_report 对空串不 panic
        let report = local_report("");
        assert_eq!(report.char_count, 0);
        assert_eq!(report.tell_density, 0.0);
    }
}
