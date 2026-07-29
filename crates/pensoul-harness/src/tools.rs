/// 工具白名单 — 控制每个阶段能使用哪些工具。
///
/// 工具访问决策优先级：
/// 1. 显式禁止列表（`tools_denied`）→ 拒绝
/// 2. 允许列表（`tools_allowed`）→ 放行（空列表表示不限制）
/// 3. 其余 → 拒绝
///
/// 每次拒绝都会写入 WAL 作为审计记录。
use crate::stage::Stage;
use crate::wal::{WalAction, WalManager};
use pensoul_core::PensoulError;

/// 工具白名单检查器。
#[derive(Debug, Clone)]
pub struct ToolWhitelist;

impl ToolWhitelist {
    /// 检查指定阶段是否允许使用某个工具。
    ///
    /// # 参数
    /// - `stage`: 当前阶段定义。
    /// - `tool_name`: 要检查的工具名称。
    /// - `wal`: WAL 管理器引用，拒绝时写入审计日志。
    /// - `current_stage`: 当前阶段的名称（用于 WAL 记录）。
    ///
    /// # 返回值
    /// - `true` → 允许使用。
    /// - `false` → 不允许（已写入 WAL）。
    pub fn check_access(
        stage: &Stage,
        tool_name: &str,
        wal: &WalManager,
        current_stage: &str,
    ) -> bool {
        // 优先级 1：显式禁止
        if stage.tools_denied.iter().any(|t| t == tool_name) {
            let _ = wal.write(
                WalAction::ToolBlocked,
                Some(current_stage),
                Some(&format!(
                    "工具 '{tool_name}' 在阶段 '{}' 的禁止列表中",
                    stage.name
                )),
            );
            return false;
        }

        // 优先级 2：允许列表为空表示不限制，否则必须在白名单中
        if !stage.tools_allowed.is_empty() && !stage.tools_allowed.iter().any(|t| t == tool_name) {
            let _ = wal.write(
                WalAction::ToolBlocked,
                Some(current_stage),
                Some(&format!(
                    "工具 '{tool_name}' 不在阶段 '{}' 的允许列表中",
                    stage.name
                )),
            );
            return false;
        }

        true
    }

    /// 构造被拒绝时的错误对象（用于上层处理）。
    pub fn denied_error(tool_name: &str, stage_name: &str) -> PensoulError {
        PensoulError::ToolAccessDenied {
            tool: tool_name.to_string(),
            stage: stage_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;
    use crate::wal::WalManager;
    use pensoul_core::StageName;
    use std::path::Path;

    fn temp_wal(dir: &Path) -> WalManager {
        WalManager::new(dir)
    }

    #[test]
    fn test_allowed_empty_means_all_tools_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = temp_wal(tmp.path());
        let stage = Stage {
            name: StageName::new("writing"),
            tools_allowed: vec![],
            tools_denied: vec![],
            ..Stage::default()
        };
        assert!(ToolWhitelist::check_access(
            &stage, "any_tool", &wal, "writing"
        ));
    }

    #[test]
    fn test_explicitly_denied_tool() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = temp_wal(tmp.path());
        let stage = Stage {
            name: StageName::new("writing"),
            tools_allowed: vec!["write_prose".into()],
            tools_denied: vec!["modify_settings".into()],
            ..Stage::default()
        };
        assert!(!ToolWhitelist::check_access(
            &stage,
            "modify_settings",
            &wal,
            "writing"
        ));
    }

    #[test]
    fn test_tool_not_in_allow_list() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = temp_wal(tmp.path());
        let stage = Stage {
            name: StageName::new("writing"),
            tools_allowed: vec!["write_prose".into(), "read_outline".into()],
            tools_denied: vec![],
            ..Stage::default()
        };
        assert!(!ToolWhitelist::check_access(
            &stage,
            "unknown_tool",
            &wal,
            "writing"
        ));
    }

    #[test]
    fn test_tool_in_allow_list() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = temp_wal(tmp.path());
        let stage = Stage {
            name: StageName::new("writing"),
            tools_allowed: vec!["write_prose".into(), "read_outline".into()],
            tools_denied: vec![],
            ..Stage::default()
        };
        assert!(ToolWhitelist::check_access(
            &stage,
            "write_prose",
            &wal,
            "writing"
        ));
    }

    #[test]
    fn test_deny_overrides_allow() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = temp_wal(tmp.path());
        let stage = Stage {
            name: StageName::new("writing"),
            tools_allowed: vec!["write_prose".into()],
            tools_denied: vec!["write_prose".into()],
            ..Stage::default()
        };
        // deny 优先于 allow
        assert!(!ToolWhitelist::check_access(
            &stage,
            "write_prose",
            &wal,
            "writing"
        ));
    }
}
