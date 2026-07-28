#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// PenSoul App — Tauri 桌面应用入口
use std::path::PathBuf;
use pensoul_app::state::AppState;

fn main() {
    let project_dir = PathBuf::from("./pensoul-project");
    let state = AppState::new(project_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            pensoul_app::commands::project::create_project,
            pensoul_app::commands::project::open_project,
            pensoul_app::commands::project::save_project,
            pensoul_app::commands::chapter::get_chapter,
            pensoul_app::commands::chapter::save_chapter,
            pensoul_app::commands::chapter::list_chapters,
            pensoul_app::commands::harness::start_harness_stage,
            pensoul_app::commands::harness::complete_harness_stage,
            pensoul_app::commands::harness::inject_memo,
            pensoul_app::commands::harness::get_harness_status,
            pensoul_app::commands::cda::find_affected_chapters,
            pensoul_app::commands::cda::get_impact_graph,
            pensoul_app::commands::memory::build_memory_packet,
            pensoul_app::commands::memory::get_hot_memory,
            pensoul_app::commands::memory::get_warm_memory,
            pensoul_app::commands::model::get_models,
            pensoul_app::commands::model::set_task_model,
            pensoul_app::commands::model::route_model,
            pensoul_app::commands::plugin::list_plugins,
            pensoul_app::commands::plugin::register_plugin,
            pensoul_app::commands::plugin::unregister_plugin,
            pensoul_app::commands::inspiration::generate_inspiration,
            pensoul_app::commands::http::http_request,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
