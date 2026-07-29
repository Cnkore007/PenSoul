/// 灵感生成 IPC 命令
use crate::state::AppState;
use pensoul_llm::TaskType;
use pensoul_llm::provider::OpenAiProvider;
use pensoul_llm::{InspirationItem, generate_inspiration as gen_inspiration};

/// 生成灵感建议
///
/// # 参数
/// - `context_type`: "character" | "world" | "outline" | "writing"
/// - `context_data`: 当前项目上下文的 JSON 字符串
///
/// # 返回值
/// 灵感建议列表，每条包含 title 和 content
#[tauri::command]
pub async fn generate_inspiration(
    state: tauri::State<'_, AppState>,
    context_type: String,
    context_data: String,
) -> Result<Vec<InspirationItem>, String> {
    // 尝试获取可用模型和提供商
    let router = state.model_router.write();

    // 先获取可用模型（克隆数据，避免借用 router）
    let model = router
        .get_recommendation(TaskType::General)
        .first()
        .map(|m| (*m).clone());
    drop(router); // 释放 router 的写锁

    // 尝试从模型配置中提取 API key 并创建提供商
    let provider = model.as_ref().and_then(|m| {
        m.api_key.as_ref().and_then(|key| {
            // 根据提供商类型创建对应的 provider
            match m.provider.as_str() {
                "anthropic" => {
                    // Anthropic 提供商暂未实现，返回 None
                    None
                }
                _ => {
                    // 默认使用 OpenAI 兼容提供商
                    Some(OpenAiProvider::new(key.clone()))
                }
            }
        })
    });

    let result = gen_inspiration(
        provider
            .as_ref()
            .map(|p| p as &dyn pensoul_llm::LlmProvider),
        model.as_ref(),
        &context_type,
        &context_data,
    );

    Ok(result)
}
