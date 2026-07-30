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
    // 同步模型偏好设置到内存
    pensoul_app::commands::model::init_preferences(&state);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
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
            pensoul_app::commands::harness::start_harness_stage,
            pensoul_app::commands::harness::complete_harness_stage,
            pensoul_app::commands::harness::approve_harness_stage,
            pensoul_app::commands::harness::inject_memo,
            pensoul_app::commands::harness::get_harness_status,
            pensoul_app::commands::cda::find_affected_chapters,
            pensoul_app::commands::cda::get_impact_graph,
            pensoul_app::commands::memory::build_memory_packet,
            pensoul_app::commands::memory::get_hot_memory,
            pensoul_app::commands::memory::get_warm_memory,
            pensoul_app::commands::model::get_models,
            pensoul_app::commands::model::set_model_preference,
            pensoul_app::commands::model::set_task_model,
            pensoul_app::commands::model::route_model,
            pensoul_app::commands::plugin::list_plugins,
            pensoul_app::commands::plugin::install_plugin,
            pensoul_app::commands::plugin::remove_plugin,
            pensoul_app::commands::plugin::toggle_plugin,
            pensoul_app::commands::inspiration::generate_inspiration,
            pensoul_app::commands::http::http_request,
            pensoul_app::commands::settings::save_settings,
            pensoul_app::commands::settings::load_settings,
            pensoul_app::commands::settings::save_concept,
            pensoul_app::commands::settings::load_concept,
            pensoul_app::commands::settings::save_sprout,
            pensoul_app::commands::settings::load_sprout,
            pensoul_app::commands::experts::save_experts,
            pensoul_app::commands::experts::load_experts,
            pensoul_app::commands::experts::scan_nuwa_skills,
            pensoul_app::commands::character::get_characters,
            pensoul_app::commands::character::save_characters,
            pensoul_app::commands::world::get_world,
            pensoul_app::commands::world::save_world,
            pensoul_app::commands::consistency::check_consistency,
            pensoul_app::commands::style::get_style_metrics,
            pensoul_app::commands::llm::list_providers,
            pensoul_app::commands::llm::save_providers,
            pensoul_app::commands::llm::list_models,
            pensoul_app::commands::llm::save_models,
            pensoul_app::commands::llm::save_api_key,
            pensoul_app::commands::llm::test_model,
            pensoul_app::commands::llm::load_api_keys,
            pensoul_app::commands::discussion::discuss_concept,
            pensoul_app::commands::expert_distill::distill_expert,
            pensoul_app::commands::experts::scan_experts_folder,
            pensoul_app::commands::experts::get_experts_folder,
            pensoul_app::commands::experts::delete_expert_skill,
            pensoul_app::commands::harness_exec::execute_harness_step,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
