/// 滚动备忘录 — 跨阶段注入的创作方向记录。
///
/// 规划阶段确认的大纲方向，存入备忘录后在后续每个阶段注入，
/// 保证 AI 始终记得最初定下的核心冲突和角色弧线。
use std::collections::HashMap;

use crate::stage::Stage;
use crate::wal::{WalAction, WalManager};
use pensoul_core::PensoulError;

/// 单条备忘录条目。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoEntry {
    /// 条目键名（如 "core_conflict"、"protagonist_arc"）。
    pub key: String,
    /// 条目内容。
    pub value: String,
}

/// 滚动备忘录，维护跨阶段共享的创作上下文。
///
/// 备忘录以 key-value 形式存储，每个 key 代表一个创作方向要素。
/// 注入时可覆盖已有条目（"滚动"语义：新信息覆盖旧信息）。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RollingMemo {
    /// 备忘录条目映射表。
    entries: HashMap<String, String>,
}

impl RollingMemo {
    /// 创建空的滚动备忘录。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入或更新一条备忘录。
    ///
    /// 如果 key 已存在则覆盖，否则新增。
    pub fn inject(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    /// 获取所有备忘录条目的引用。
    pub fn entries(&self) -> &HashMap<String, String> {
        &self.entries
    }

    /// 获取指定 key 的值。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    /// 删除指定 key。
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    /// 备忘录条目数量。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 备忘录是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 将所有条目序列化为上下文字符串，用于注入 AI 提示。
    ///
    /// 使用 JSON 编码避免 `": "` 分隔符被解析为 MemoInject 数据格式。
    /// 格式：{"创作备忘录": {"core_conflict": "...", "protagonist_arc": "..."}}
    pub fn to_context_string(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        // 使用 BTreeMap 按 key 排序，JSON 编码避免解析歧义
        let sorted: std::collections::BTreeMap<&String, &String> = self.entries.iter().collect();
        serde_json::json!({"创作备忘录": sorted}).to_string()
    }
}

/// 工具白名单 — 控制每个阶段能使用哪些工具。
///
/// 工具访问决策优先级：
/// 1. 显式禁止列表（`tools_denied`）→ 拒绝
/// 2. 允许列表（`tools_allowed`）→ 放行（空列表表示不限制）
/// 3. 其余 → 拒绝
///
/// 每次拒绝都会写入 WAL 作为审计记录。
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
    fn test_new_memo_is_empty() {
        let memo = RollingMemo::new();
        assert!(memo.is_empty());
        assert_eq!(memo.len(), 0);
    }

    #[test]
    fn test_inject_and_get() {
        let mut memo = RollingMemo::new();
        memo.inject("conflict", "主角与反派的终极对决");
        memo.inject("arc", "从懦弱到勇敢");

        assert_eq!(memo.get("conflict"), Some("主角与反派的终极对决"));
        assert_eq!(memo.get("arc"), Some("从懦弱到勇敢"));
        assert_eq!(memo.len(), 2);
    }

    #[test]
    fn test_inject_overwrites() {
        let mut memo = RollingMemo::new();
        memo.inject("key", "old_value");
        memo.inject("key", "new_value");
        assert_eq!(memo.get("key"), Some("new_value"));
        assert_eq!(memo.len(), 1);
    }

    #[test]
    fn test_context_string_sorted() {
        let mut memo = RollingMemo::new();
        memo.inject("z_last", "Z");
        memo.inject("a_first", "A");
        let ctx = memo.to_context_string();
        // JSON 格式应包含所有条目
        assert!(ctx.contains("z_last"));
        assert!(ctx.contains("a_first"));
        assert!(ctx.contains("创作备忘录"));
        // BTreeMap 排序保证 a_first 在 z_last 前面
        let a_pos = ctx.find("a_first").unwrap();
        let z_pos = ctx.find("z_last").unwrap();
        assert!(a_pos < z_pos);
    }

    #[test]
    fn test_empty_context_string() {
        let memo = RollingMemo::new();
        assert!(memo.to_context_string().is_empty());
    }

    #[test]
    fn test_remove() {
        let mut memo = RollingMemo::new();
        memo.inject("key", "value");
        let removed = memo.remove("key");
        assert_eq!(removed, Some("value".to_string()));
        assert!(memo.is_empty());
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
