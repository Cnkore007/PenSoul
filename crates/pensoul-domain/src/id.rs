// id.rs — ID 类型定义
// 使用宏生成类型安全的 ID 新类型

use serde::{Deserialize, Serialize};
use std::fmt;

/// 宏：定义类型安全的 ID 新类型
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

// 实体 ID 类型
define_id!(/// 项目 ID
    ProjectId);
define_id!(/// 角色 ID
    CharacterId);
define_id!(/// 事件 ID
    EventId);
define_id!(/// 设定 ID
    SettingId);
define_id!(/// 组织 ID
    OrganizationId);
define_id!(/// 伏笔 ID
    ForeshadowId);
define_id!(/// 位置 ID
    LocationId);
define_id!(/// 世界 ID
    WorldId);
define_id!(/// 章节 ID
    ChapterId);
define_id!(/// 卷 ID
    VolumeId);
define_id!(/// 关系 ID
    RelationId);
define_id!(/// 约束/规则 ID
    RuleId);
define_id!(/// Agent ID
    AgentId);
define_id!(/// 阶段名
    StageName);
define_id!(/// 技能 ID
    SkillId);
define_id!(/// 模板 ID
    TemplateId);

/// 章节 ID 的排序实现：数字优先，非数字排在后面
impl Ord for ChapterId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.as_i64(), other.as_i64()) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.0.cmp(&other.0),
        }
    }
}

impl PartialOrd for ChapterId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ChapterId {
    /// 尝试解析为 i64（数字章节号）
    pub fn as_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }

    /// 从 i64 创建章节 ID
    pub fn from_i64(n: i64) -> Self {
        Self(n.to_string())
    }
}
