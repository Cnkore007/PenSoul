// api.rs — HTTP 路由定义

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::catch_panic::CatchPanicLayer;

use crate::commands;
use crate::state::AppState;

/// 全局 panic 兜底：任何 handler panic 只返回 500 并记录日志，不拖垮整个服务进程
fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let message = if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        "未知 panic".to_string()
    };
    eprintln!("[PenSoul] handler panic 已捕获: {message}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("内部错误（panic）: {message}"),
    )
        .into_response()
}

pub fn create_router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        // 项目管理（写操作一律 POST/PUT/DELETE，禁用 GET 副作用）
        .route("/api/projects", get(commands::project::list_projects))
        .route("/api/projects/create", post(commands::project::create_project))
        .route("/api/projects/open", post(commands::project::open_project))
        .route("/api/projects/save", post(commands::project::save_project))
        .route("/api/projects/delete", delete(commands::project::delete_project))
        // 实体管理
        .route("/api/entities", get(commands::entity::list_entities))
        .route("/api/entities/character", post(commands::entity::add_character))
        .route("/api/entities/character/update", put(commands::entity::update_character))
        .route("/api/entities/character/delete", delete(commands::entity::delete_character))
        .route("/api/entities/event", post(commands::entity::add_event))
        .route("/api/entities/event/update", put(commands::entity::update_event))
        .route("/api/entities/event/delete", delete(commands::entity::delete_event))
        .route("/api/entities/setting", post(commands::entity::add_setting))
        .route("/api/entities/setting/update", put(commands::entity::update_setting))
        .route("/api/entities/setting/delete", delete(commands::entity::delete_setting))
        // 组织档案（P0）
        .route("/api/entities/organizations", get(commands::entity::list_organizations))
        .route("/api/entities/organization", post(commands::entity::add_organization))
        .route("/api/entities/organization/update", put(commands::entity::update_organization))
        .route("/api/entities/organization/delete", delete(commands::entity::delete_organization))
        // 图谱查询
        .route("/api/graph/stats", get(commands::graph::graph_stats))
        .route("/api/graph/predict", post(commands::graph::predict_impact))
        .route("/api/constraints/check", get(commands::graph::check_constraints))
        // 仪表盘
        .route("/api/dashboard/overview", get(commands::dashboard::project_overview))
        // 世界管理
        .route("/api/world/characters", get(commands::world::list_characters))
        .route("/api/world/locations", get(commands::world::list_locations))
        .route("/api/world/timeline", get(commands::world::list_timeline))
        .route("/api/world/foreshadows", get(commands::world::list_foreshadows))
        .route("/api/world/foreshadows/add", post(commands::world::add_foreshadow))
        .route("/api/world/foreshadows/update", put(commands::world::update_foreshadow))
        .route("/api/world/foreshadows/delete", delete(commands::world::delete_foreshadow))
        .route("/api/world/rules", get(commands::world::list_rules))
        .route("/api/world/rules/add", put(commands::world::add_rule))
        .route("/api/world/rules/update", put(commands::world::update_rule))
        .route("/api/world/rules/delete", delete(commands::world::delete_rule))
        .route("/api/world/concept", get(commands::world::get_concept))
        .route("/api/world/concept/update", put(commands::world::update_concept))
        // 写作风格笔记（正典 AestheticLayer，F13）
        .route("/api/world/style", get(commands::world::get_style))
        .route("/api/world/style", put(commands::world::update_style))
        // 萌芽（对话式创作工作台）
        .route("/api/sprout/session", get(commands::sprout::get_session))
        .route("/api/sprout/start", post(commands::sprout::start))
        .route("/api/sprout/chat", post(commands::sprout::chat))
        .route("/api/sprout/generate", post(commands::sprout::generate))
        .route("/api/sprout/apply", post(commands::sprout::apply))
        .route("/api/sprout/discard", post(commands::sprout::discard))
        .route("/api/sprout/clear", post(commands::sprout::clear))
        // 大纲管理
        .route("/api/outline/arcs", get(commands::outline::list_arcs))
        .route("/api/outline/arcs/create", post(commands::outline::create_arc))
        .route("/api/outline/arcs/update", put(commands::outline::update_arc))
        .route("/api/outline/arcs/delete", delete(commands::outline::delete_arc))
        .route("/api/outline/chapters", get(commands::outline::list_chapters))
        .route("/api/outline/chapters/create", post(commands::outline::create_chapter))
        .route("/api/outline/chapters/update", put(commands::outline::update_chapter))
        .route("/api/outline/chapters/delete", delete(commands::outline::delete_chapter))
        .route("/api/outline/chapters/content", put(commands::outline::save_chapter_content))
        .route("/api/outline/chapters/content", get(commands::outline::get_chapter_content))
        // AI 辅助写作（建议制，保存走章节集成层）
        .route("/api/writing/generate", post(commands::writing::generate))
        // 叙事技巧库（F12/F15）
        .route("/api/writing/techniques", get(commands::techniques::list_techniques))
        // AI 章节审校（F3 完整版 / F4 / F8，建议制）
        .route("/api/writing/review", post(commands::review::review))
        // 批注 CRUD（P2）
        .route("/api/writing/annotations", post(commands::rewrite::add_annotation))
        .route("/api/writing/annotations/status", put(commands::rewrite::update_annotation))
        .route("/api/writing/annotations/delete", delete(commands::rewrite::delete_annotation))
        // 书籍蒸馏（P3：语料摄取 txt/md/epub/pdf → 风格配方）
        .route("/api/distill/corpus", post(commands::distill::add_corpus))
        .route("/api/distill/corpus/list", get(commands::distill::list_corpus))
        .route("/api/distill/corpus/delete", delete(commands::distill::delete_corpus))
        .route("/api/distill/analyze", post(commands::distill::analyze))
        .route("/api/distill/recipe", get(commands::distill::get_recipe))
        .route("/api/distill/recipe/update", put(commands::distill::update_recipe))
        .route("/api/distill/recipe/delete", delete(commands::distill::delete_recipe))
        // 归档与操作日志（P6）
        .route("/api/log/operations", get(commands::archive::list_operations))
        .route("/api/log/rollback", post(commands::archive::rollback))
        .route("/api/archive/compress", post(commands::archive::compress))
        .route("/api/archive/list", get(commands::archive::list_archive))
        .route("/api/log/cost", get(commands::archive::cost_report))
        // 细纲化与批量写作（P5）
        .route("/api/outline/detail/generate", post(commands::batch::detail_generate))
        .route("/api/outline/detail/import", post(commands::batch::detail_import))
        .route("/api/writing/batch", post(commands::batch::batch_write))
        // 级联同步（P4：影响分析 + 仅向后受控级联）
        .route("/api/writing/cascade/analyze", post(commands::cascade::cascade_analyze))
        .route("/api/writing/cascade/apply", post(commands::cascade::cascade_apply))
        // AI 审核改写 / 消痕改写（P2，建议制）
        .route("/api/writing/rewrite", post(commands::rewrite::rewrite))
        // 事实提取（P1，全自动）
        .route("/api/writing/extract-facts", post(commands::extract::extract_facts))
        // Agent 注册表：按角色选模型（P0b）
        .route("/api/agent/configs", get(commands::agent::list_agent_configs))
        .route("/api/agent/configs", put(commands::agent::update_agent_config))
        // 全局 LLM 配置管理
        .route("/api/llm/configs", get(commands::llm::list_configs))
        .route("/api/llm/configs", post(commands::llm::create_config))
        .route("/api/llm/configs", put(commands::llm::update_config))
        .route("/api/llm/configs", delete(commands::llm::delete_config))
        .route("/api/llm/default", post(commands::llm::set_default))
        .route("/api/llm/status", get(commands::llm::get_status))
        .route("/api/llm/models/pull", post(commands::llm::pull_models))
        .route("/api/llm/docs/model", post(commands::llm::fetch_model_doc))
        .route("/api/llm/test", post(commands::llm::test_llm))
        .route("/api/llm/context-check", post(commands::llm::context_check))
        // 记忆检索
        .route("/api/memory/retrieve", post(commands::memory::retrieve_memory))
        // 蓝图
        .route("/api/blueprint", get(commands::blueprint::get_blueprint))
        .with_state(state)
        // 语料上传支持大文件（默认 2MB 不够）
        .layer(DefaultBodyLimit::max(128 * 1024 * 1024))
        // 全局 panic 兜底：任何 handler panic 只返回 500 并记录日志，不拖垮服务进程
        .layer(CatchPanicLayer::custom(handle_panic))
}
