/// PenSoul App — Tauri 桌面应用后端
///
/// 提供 IPC 命令、全局状态管理和视图状态。
pub mod state;
pub mod commands;
pub mod views;

// 重新导出公共类型
pub use state::AppState;
pub use views::*;
