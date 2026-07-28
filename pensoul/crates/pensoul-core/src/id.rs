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
