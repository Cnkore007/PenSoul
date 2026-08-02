/// PenSoul Memory — 四层记忆系统
///
/// - 热记忆 (hot)：当前章 ± window_size 完整文本
/// - 温记忆 (warm)：结构化摘要 + 角色状态 + 伏笔
/// - 冷记忆 (cold)：向量检索（原型用关键词）
/// - 冰记忆 (archive)：归档已完成章节
/// - 叙事记忆 (narrative)：重要性排序的叙事细节
/// - 记忆包 (packet)：构建最终产物
/// - 记忆管道 (pipeline)：8 步更新流程
pub mod archive;
pub mod layers;
pub mod narrative;
pub mod packet;
pub mod pipeline;
pub mod warm;

// Re-export 所有公有类型
pub use archive::ArchiveMemory;
pub use layers::{ColdMemory, HotMemory};
pub use narrative::NarrativeMemory;
pub use packet::{
    BudgetRatio, ChapterSummary, EditingMode, MemoryPacket, NarrativeCategory, NarrativeDetail,
    WarmMemoryData, estimate_tokens,
};
pub use pipeline::MemoryPipeline;
pub use warm::WarmMemory;
