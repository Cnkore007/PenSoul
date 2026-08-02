#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pensoul_app::state::AppState;
/// PenSoul App — Tauri 桌面应用入口
use std::path::PathBuf;

fn main() {
    // 使用可靠路径：优先取 Tauri 数据目录，回退到当前目录
    let base_dir = if cfg!(debug_assertions) {
        // 开发模式：项目根目录下的 data。
        // 沿可执行文件路径向上找含 Cargo.toml 的工作区根目录，
        // 兼容 cargo tauri dev（target/debug/pensoul-app）与
        // .app bundle（target/debug/bundle/macos/PenSoul.app/Contents/MacOS/...）两种形态。
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let workspace_root = exe
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists())
            .map(|p| p.to_path_buf());
        match workspace_root {
            Some(root) => root.join("data"),
            None => {
                // 回退：exe_dir 向上两级
                let exe_dir = exe
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                let project_root = exe_dir
                    .parent() // target
                    .and_then(|p| p.parent()) // project_root
                    .unwrap_or(&exe_dir);
                project_root.join("data")
            }
        }
    } else {
        // 生产模式：用户文档目录下的 PenSoul 目录
        let home = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("PenSoul")
    };
    std::fs::create_dir_all(&base_dir).ok();

    let state = AppState::new(base_dir);

    // 尝试加载 API 密钥
    let _ = state.load_api_keys();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            pensoul_app::commands::annotations::annotation_add,
            pensoul_app::commands::annotations::annotation_update,
            pensoul_app::commands::annotations::annotation_remove,
            pensoul_app::commands::annotations::annotation_resolve,
            pensoul_app::commands::annotations::annotations_list,
            pensoul_app::commands::annotations::annotations_all,
            pensoul_app::commands::annotations::annotations_export,
            pensoul_app::edits::get_pending_edits,
            pensoul_app::edits::distill_pending_lessons,
            pensoul_app::page_review::review_page_changes,
            pensoul_app::page_review::apply_page_review,
            pensoul_app::page_review::undo_page_change,
            pensoul_app::page_review::page_undo_available,
            pensoul_app::chapter_review::review_chapter_changes,
            pensoul_app::chapter_review::apply_chapter_review,
            pensoul_app::commands::project::create_project,
            pensoul_app::commands::project::list_projects,
            pensoul_app::commands::project::get_project,
            pensoul_app::commands::project::update_project,
            pensoul_app::commands::project::delete_project,
            pensoul_app::commands::project::open_project,
            pensoul_app::commands::project::save_project,
            pensoul_app::commands::chapter::get_chapter,
            pensoul_app::commands::chapter::save_chapter,
            pensoul_app::commands::chapter::list_chapters,
            pensoul_app::commands::chapter::upsert_chapter,
            pensoul_app::commands::chapter_rewrite::rewrite_chapter_with_annotations,
            pensoul_app::commands::chapter_rewrite::list_chapter_revisions,
            pensoul_app::commands::chapter_rewrite::rollback_chapter,
            pensoul_app::commands::chapter_rewrite::get_writing_lessons,
            pensoul_app::commands::chapter_rewrite::save_writing_lessons,
            pensoul_app::commands::chapter::save_volumes,
            pensoul_app::commands::chapter::get_volumes,
            pensoul_app::commands::chapter::delete_chapter,
            pensoul_app::commands::chapter::delete_volume,
            pensoul_app::commands::harness::start_harness_stage,
            pensoul_app::commands::harness::complete_harness_stage,
            pensoul_app::commands::harness::approve_harness_stage,
            pensoul_app::commands::harness::inject_memo,
            pensoul_app::commands::harness::get_harness_status,
            pensoul_app::commands::harness::execute_harness_step,
            pensoul_app::commands::harness::run_chapter_pipeline,
            pensoul_app::commands::harness::pause_pipeline,
            pensoul_app::commands::harness::resume_pipeline,
            pensoul_app::commands::harness::stop_pipeline,
            pensoul_app::commands::harness::get_pipeline_state,
            pensoul_app::commands::harness::build_memory_packet,
            pensoul_app::commands::harness::get_hot_memory,
            pensoul_app::commands::harness::get_warm_memory,
            pensoul_app::commands::cda::find_affected_chapters,
            pensoul_app::commands::cda::analyze_chapter_impact,
            pensoul_app::commands::cda::get_impact_graph,
            pensoul_app::commands::optimize::optimize_content,
            pensoul_app::commands::http::http_request,
            pensoul_app::commands::data::save_settings,
            pensoul_app::commands::data::load_settings,
            pensoul_app::commands::data::save_concept,
            pensoul_app::commands::data::load_concept,
            pensoul_app::commands::data::save_sprout,
            pensoul_app::commands::data::load_sprout,
            pensoul_app::commands::data::save_workflow_skills,
            pensoul_app::commands::data::load_workflow_skills,
            pensoul_app::commands::experts::save_experts,
            pensoul_app::commands::experts::load_experts,
            pensoul_app::commands::experts::scan_nuwa_skills,
            pensoul_app::commands::data::get_characters,
            pensoul_app::commands::data::save_characters,
            pensoul_app::commands::data::get_world,
            pensoul_app::commands::data::save_world,
            pensoul_app::commands::data::check_consistency,
            pensoul_app::commands::data::get_style_metrics,
            pensoul_app::commands::llm::list_providers,
            pensoul_app::commands::llm::save_providers,
            pensoul_app::commands::llm::list_models,
            pensoul_app::commands::llm::save_models,
            pensoul_app::commands::llm::save_api_key,
            pensoul_app::commands::llm::load_api_keys,
            pensoul_app::commands::discussion::discuss_concept,
            pensoul_app::commands::discussion::get_discussion_state,
            pensoul_app::commands::expert_distill::get_distill_state,
            pensoul_app::commands::expert_distill::distill_expert,
            pensoul_app::commands::experts::scan_experts_folder,
            pensoul_app::commands::experts::get_experts_folder,
            pensoul_app::commands::experts::delete_expert_skill,
            pensoul_app::commands::outline::list_outline_arcs,
            pensoul_app::commands::outline::save_outline_arcs,
            pensoul_app::commands::outline::expand_outline_arc,
            pensoul_app::commands::book_distill::distill_book,
            pensoul_app::commands::book_distill::list_book_packages,
            pensoul_app::commands::book_distill::delete_book_package,
            pensoul_app::commands::methodology_distill::distill_methodology,
            pensoul_app::commands::ai_flavor::analyze_ai_flavor,
            pensoul_app::commands::ai_flavor::get_anti_ai_rules,
            pensoul_app::commands::ai_flavor::save_anti_ai_rules,
            pensoul_app::style_fingerprint::get_style_fingerprint,
            pensoul_app::commands::workflow_templates::list_workflow_templates,
            pensoul_app::commands::workflow_templates::save_workflow_templates,
            pensoul_app::commands::workflow_templates::reset_workflow_templates,
            pensoul_app::commands::workflow_templates::clear_all_project_overrides,
            pensoul_app::commands::workflow_templates::save_workflow_ref,
            pensoul_app::commands::workflow_templates::load_workflow_ref,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
