/// 滚动备忘录 — 跨阶段注入的创作方向记录。
///
/// 规划阶段确认的大纲方向，存入备忘录后在后续每个阶段注入，
/// 保证 AI 始终记得最初定下的核心冲突和角色弧线。
use std::collections::HashMap;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
