// error.rs — 错误类型定义

use thiserror::Error;

/// PenSoul 领域错误
#[derive(Debug, Error)]
pub enum PensoulError {
    #[error("阶段未找到: {0}")]
    StageNotFound(String),

    #[error("工具访问被拒绝: 工具={tool}, 阶段={stage}")]
    ToolAccessDenied { tool: String, stage: String },

    #[error("门控条件未满足: {reason}")]
    GateConditionFailed { reason: String },

    #[error("版本冲突: 章节={chapter_id}, 期望版本={expected}, 实际版本={actual}")]
    VersionConflict {
        chapter_id: String,
        expected: i32,
        actual: i32,
    },

    #[error("操作被拒绝: {0}")]
    OperationRejected(String),

    #[error("一致性违规: 实体={entity_id}, 章节={chapter_a}-{chapter_b}: {description}")]
    ConsistencyViolation {
        entity_id: String,
        chapter_a: i64,
        chapter_b: i64,
        description: String,
    },

    #[error("WAL 校验失败: 索引={index}")]
    WalChecksumFailed { index: usize },

    #[error("LLM 所有模型失败: {chain:?}")]
    LlmAllModelsFailed { chain: Vec<String> },

    #[error("导入错误: {0}")]
    ImportError(String),

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// PenSoul Result 类型
pub type Result<T> = std::result::Result<T, PensoulError>;
