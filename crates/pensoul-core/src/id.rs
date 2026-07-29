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
