/// ID 类型定义宏
#[macro_export]
macro_rules! define_id {
    ($name:ident) => {
        /// 唯一标识符类型
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(String);

        impl $name {
            /// 创建新的 ID
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// 获取 ID 的字符串引用
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }
        }
    };
}

// ===== 世界层 ID =====
define_id!(WorldId);
define_id!(LocationId);
define_id!(EventId);
define_id!(SettingId);

// ===== 角色层 ID =====
define_id!(CharacterId);
define_id!(NodeId);
define_id!(EdgeId);
define_id!(EntityId);

// ===== 叙事层 ID =====
define_id!(ForeshadowId);
define_id!(ChapterId);
define_id!(VolumeId);

// ===== 智能体层 ID =====
define_id!(SkillId);
define_id!(RuleId);
define_id!(AgentId);
define_id!(StageName);
define_id!(PluginId);
define_id!(ProjectId);

// ===== 审美层 ID =====
define_id!(AntiAiRuleId);

/// 向后兼容的 ID 反序列化辅助模块。
///
/// 接受 JSON 字符串或数字，统一反序列化为对应的 ID 类型。
/// 用于兼容旧版数据中 chapter_id 以 i64 数字存储的格式。
pub mod flexible_id {
    use super::ChapterId;
    use serde::{Deserialize, Deserializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FlexibleId {
        Num(i64),
        Str(String),
    }

    /// 从字符串或数字反序列化为 `ChapterId`。
    pub fn deserialize_chapter_id<'de, D>(deserializer: D) -> Result<ChapterId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = match FlexibleId::deserialize(deserializer)? {
            FlexibleId::Num(n) => n.to_string(),
            FlexibleId::Str(s) => s,
        };
        Ok(ChapterId(raw))
    }
}
// ===== ChapterId 数值转换统一入口 =====
// chapter_id 在业务上是数字字符串。所有 i64 互转必须走这两个方法，
// 非数字 ID 返回 None，由调用方显式跳过，禁止静默 fallback 为 0。
impl ChapterId {
    /// 将数字字符串形式的章节 ID 转为 i64；非数字 ID 返回 None。
    pub fn as_i64(&self) -> Option<i64> {
        self.0.parse::<i64>().ok()
    }

    /// 从 i64 构造章节 ID。
    pub fn from_i64(n: i64) -> Self {
        Self::new(n.to_string())
    }
}

// ===== ChapterId 自定义排序 =====
// chapter_id 在业务上是数字，但存储为字符串。
// 默认的字典序会把 "10" 排在 "2" 前面，所以需要数值优先比较。
impl PartialOrd for ChapterId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChapterId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.0.parse::<i64>(), other.0.parse::<i64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            // 解析失败时 fallback 到字典序
            _ => self.0.cmp(&other.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── define_id! 宏行为 ─────────────────────────────────────────────

    #[test]
    fn test_id_new_and_as_str() {
        let id = WorldId::new("world-1");
        assert_eq!(id.as_str(), "world-1");
    }

    #[test]
    fn test_id_display() {
        let id = CharacterId::new("char-1");
        assert_eq!(format!("{id}"), "char-1");
    }

    #[test]
    fn test_id_from_str_and_string() {
        let a: ChapterId = "1".into();
        let b: ChapterId = String::from("1").into();
        assert_eq!(a, b);
    }

    #[test]
    fn test_id_default_generates_unique_uuid() {
        let a = ProjectId::default();
        let b = ProjectId::default();
        // 两个默认 ID 不应相同（UUID 随机）
        assert_ne!(a, b);
        assert!(!a.as_str().is_empty());
    }

    #[test]
    fn test_id_equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ChapterId::new("1"));
        assert!(set.contains(&ChapterId::new("1")));
        assert!(!set.contains(&ChapterId::new("2")));
    }

    // ── ChapterId 数值转换 ────────────────────────────────────────────

    #[test]
    fn test_chapter_id_as_i64_numeric() {
        assert_eq!(ChapterId::new("42").as_i64(), Some(42));
        assert_eq!(ChapterId::new("0").as_i64(), Some(0));
        assert_eq!(ChapterId::new("-3").as_i64(), Some(-3));
    }

    #[test]
    fn test_chapter_id_as_i64_non_numeric() {
        assert_eq!(ChapterId::new("ch_001").as_i64(), None);
        assert_eq!(ChapterId::new("").as_i64(), None);
        assert_eq!(ChapterId::new("1.5").as_i64(), None);
    }

    #[test]
    fn test_chapter_id_from_i64_round_trip() {
        let id = ChapterId::from_i64(7);
        assert_eq!(id.as_str(), "7");
        assert_eq!(id.as_i64(), Some(7));
    }

    // ── ChapterId 数值优先排序 ────────────────────────────────────────

    #[test]
    fn test_chapter_id_numeric_ordering() {
        // 字典序会把 "10" 排在 "2" 前，业务上应按数值排序
        assert!(ChapterId::new("2") < ChapterId::new("10"));
        assert!(ChapterId::new("10") > ChapterId::new("2"));
        assert!(ChapterId::new("1") < ChapterId::new("2"));
    }

    #[test]
    fn test_chapter_id_non_numeric_fallback_lexicographic() {
        assert!(ChapterId::new("abc") < ChapterId::new("abd"));
    }

    #[test]
    fn test_chapter_id_sorting_mixed() {
        let mut ids = [
            ChapterId::new("10"),
            ChapterId::new("2"),
            ChapterId::new("1"),
        ];
        ids.sort();
        let ordered: Vec<&str> = ids.iter().map(|i| i.as_str()).collect();
        assert_eq!(ordered, ["1", "2", "10"]);
    }

    // ── flexible_id 兼容层 ────────────────────────────────────────────

    #[test]
    fn test_flexible_id_from_number() {
        let json = serde_json::json!(5);
        let id = serde_json::from_value::<serde_json::Value>(json).ok();
        assert!(id.is_some());

        // 通过结构体反序列化验证数字兼容
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "flexible_id::deserialize_chapter_id")]
            chapter_id: ChapterId,
        }
        let w: Wrapper = serde_json::from_str(r#"{"chapter_id": 5}"#).unwrap();
        assert_eq!(w.chapter_id.as_str(), "5");
    }

    #[test]
    fn test_flexible_id_from_string() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "flexible_id::deserialize_chapter_id")]
            chapter_id: ChapterId,
        }
        let w: Wrapper = serde_json::from_str(r#"{"chapter_id": "7"}"#).unwrap();
        assert_eq!(w.chapter_id.as_str(), "7");
    }

    #[test]
    fn test_flexible_id_rejects_invalid_type() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(deserialize_with = "flexible_id::deserialize_chapter_id")]
            chapter_id: ChapterId,
        }
        let result = serde_json::from_str::<Wrapper>(r#"{"chapter_id": true}"#);
        assert!(result.map(|w| w.chapter_id).is_err());
    }

    // ── 序列化 round-trip ─────────────────────────────────────────────

    #[test]
    fn test_id_serde_round_trip() {
        let id = ForeshadowId::new("fs-1");
        let json = serde_json::to_string(&id).unwrap();
        let back: ForeshadowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
