/// PenSoul 错误类型定义
use thiserror::Error;

/// PenSoul 核心错误枚举
#[derive(Error, Debug, Clone)]
pub enum PensoulError {
    /// 阶段未注册
    #[error("阶段未注册: {0}")]
    StageNotFound(String),

    /// 工具访问被拒绝
    #[error("工具访问被拒绝: {tool} 在阶段 {stage} 中不被允许")]
    ToolAccessDenied { tool: String, stage: String },

    /// 门控条件不满足
    #[error("门控条件不满足: {reason}")]
    GateConditionFailed { reason: String },

    /// 版本冲突
    #[error("版本冲突: 章节 {chapter_id} 期望版本 {expected}，实际版本 {actual}")]
    VersionConflict {
        chapter_id: i64,
        expected: i32,
        actual: i32,
    },

    /// 操作被拒绝
    #[error("操作被拒绝: {0}")]
    OperationRejected(String),

    /// 插件验证失败
    #[error("插件验证失败: {errors:?}")]
    PluginValidationFailed { errors: Vec<String> },

    /// 一致性违反
    #[error(
        "一致性违反: {entity_id} 在第 {chapter_a} 章和第 {chapter_b} 章之间不一致: {description}"
    )]
    ConsistencyViolation {
        entity_id: String,
        chapter_a: i64,
        chapter_b: i64,
        description: String,
    },

    /// WAL 校验失败
    #[error("WAL 校验失败: 条目 {index} checksum 不匹配")]
    WalChecksumFailed { index: usize },

    /// LLM 调用失败
    #[error("LLM 调用失败: 所有模型均不可用，尝试链: {chain:?}")]
    LlmAllModelsFailed { chain: Vec<String> },

    /// 导入失败
    #[error("导入失败: {0}")]
    ImportError(String),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    Internal(String),
}

/// PenSoul Result 类型别名
pub type Result<T> = std::result::Result<T, PensoulError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_stage_not_found() {
        let e = PensoulError::StageNotFound("萌芽阶段".to_string());
        assert_eq!(e.to_string(), "阶段未注册: 萌芽阶段");
    }

    #[test]
    fn test_display_version_conflict_includes_all_fields() {
        let e = PensoulError::VersionConflict {
            chapter_id: 7,
            expected: 3,
            actual: 5,
        };
        let msg = e.to_string();
        assert!(msg.contains("章节 7"));
        assert!(msg.contains("期望版本 3"));
        assert!(msg.contains("实际版本 5"));
    }

    #[test]
    fn test_display_wal_checksum_failed() {
        let e = PensoulError::WalChecksumFailed { index: 42 };
        assert_eq!(e.to_string(), "WAL 校验失败: 条目 42 checksum 不匹配");
    }

    #[test]
    fn test_display_tool_access_denied() {
        let e = PensoulError::ToolAccessDenied {
            tool: "write_chapter".to_string(),
            stage: "审阅".to_string(),
        };
        let msg = e.to_string();
        assert!(msg.contains("write_chapter"));
        assert!(msg.contains("审阅"));
    }

    #[test]
    fn test_display_consistency_violation() {
        let e = PensoulError::ConsistencyViolation {
            entity_id: "角色-张三".to_string(),
            chapter_a: 1,
            chapter_b: 3,
            description: "位置突变".to_string(),
        };
        let msg = e.to_string();
        assert!(msg.contains("角色-张三"));
        assert!(msg.contains("第 1 章"));
        assert!(msg.contains("第 3 章"));
        assert!(msg.contains("位置突变"));
    }

    #[test]
    fn test_error_clone_preserves_message() {
        let e = PensoulError::Internal("boom".to_string());
        let cloned = e.clone();
        assert_eq!(e.to_string(), cloned.to_string());
    }
}
