//! 页面内容优化 IPC 命令 — 对世界观/人物志页面的现有条目进行优化与整理
//!
//! 与「灵感」（生成新建议）不同，「优化」不新增内容：
//! 只对用户已有的文字做润色、结构化、去重与补全，保持条目数量与含义不变。
use crate::state::AppState;

use super::llm_helper as lh;

/// 优化页面内容
///
/// # 参数
/// - `content_type`: "world" | "character"
/// - `content_json`: 前端页面数据的 JSON 字符串（保持原结构返回）
/// - `model_id`: 可选，指定使用的模型；缺省时取第一个可用模型
///
/// # 返回值
/// 优化后的同结构 JSON 字符串
#[tauri::command]
pub async fn optimize_content(
    state: tauri::State<'_, AppState>,
    content_type: String,
    content_json: String,
    model_id: Option<String>,
) -> Result<String, String> {
    lh::ensure_api_keys_loaded(&state);

    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);
    let api_keys = { state.api_keys.read().clone() };

    // 模型选择：优先用前端指定的模型（经 resolve_provider 解析供应商），
    // 否则取全局默认模型，其次任意可用模型
    let model_to_provider = lh::build_model_to_provider(&saved_models);
    let provider_api_bases = lh::build_provider_api_bases(&saved_providers);
    let chosen_model = match model_id.as_deref() {
        Some(mid) if !mid.is_empty() => mid.to_string(),
        _ => lh::pick_default_model(&saved_models, &api_keys)
            .ok_or_else(|| "未配置任何 LLM API Key，请在模型设置中配置".to_string())?,
    };
    let (_provider_id, api_key, api_base) = lh::resolve_provider(
        &chosen_model,
        &model_to_provider,
        &provider_api_bases,
        &api_keys,
    )?;
    let model_id = chosen_model.as_str();

    let type_label = match content_type.as_str() {
        "world" => "世界观（地点/时间线/设定规则）",
        "character" => "人物志（角色及其性格特质）",
        other => return Err(format!("不支持的内容类型: {other}")),
    };

    let system = "你是小说设定集的编辑。你的工作是优化与整理作者已有的设定文字，\
        输出严格 JSON。不评论、不解释，只输出 JSON。";
    let user_prompt = format!(
        "以下是一个小说项目的{type_label}页面数据（JSON）。请优化整理：\n\
         1. 润色每条描述：语句通顺、具体、有画面感，风格统一为设定集口吻\n\
         2. 合并语义重复的条目（保留信息更全的那条，并入另一条的独有信息）\n\
         3. 补全明显残缺的描述（基于已有信息合理补全，不虚构新设定）\n\
         4. 统一术语与格式（如时间写法、称呼）\n\
         5. 保持 JSON 结构与原数据完全一致（同样的键、同样的嵌套），id 字段原样保留\n\
         6. 不要新增原数据中没有的条目，不要删除有实质信息的条目\n\n\
         用 ===OPTIMIZED_BEGIN=== 和 ===OPTIMIZED_END=== 包裹输出。\n\n\
         原数据：\n{content_json}"
    );

    let text = lh::call_llm(
        &lh::ProviderAuth {
            provider_id: &_provider_id,
            api_key: &api_key,
            api_base: &api_base,
        },
        model_id,
        system,
        &user_prompt,
        0.4,
        // 优化整页内容输出量大，推理型模型还需 reasoning 预算
        16384,
    )
    .await?;

    let begin = text
        .find("===OPTIMIZED_BEGIN===")
        .map(|i| i + "===OPTIMIZED_BEGIN===".len());
    let end = text.rfind("===OPTIMIZED_END===");
    let (Some(b), Some(e)) = (begin, end) else {
        return Err("LLM 输出缺少优化结果标记，请重试".to_string());
    };
    if e <= b {
        return Err("LLM 优化结果为空，请重试".to_string());
    }

    let json_str = text[b..e]
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 校验是合法 JSON 再返回
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("优化结果不是合法 JSON: {e}"))?;
    serde_json::to_string(&parsed).map_err(|e| e.to_string())
}
