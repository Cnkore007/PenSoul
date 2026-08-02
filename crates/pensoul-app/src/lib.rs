pub mod commands;
pub mod edits;
/// PenSoul App — Tauri 桌面应用后端
///
/// 提供 IPC 命令、全局状态管理与章节连写管线编排。
pub mod integration;
/// 模型档案：按模型名自动匹配 API 参数体系（推理开关/预算字段/输出上限）
pub mod llm_profile;
/// 章节连写管线编排器（写作 → 审查 → 回灌闭环）
pub mod pipeline;
pub mod state;

// 重新导出公共类型
pub use state::AppState;
