pub mod ai_flavor;
pub mod book_distill;
/// 书籍文件解析（txt/md/epub/pdf → 纯文本抽样，书籍蒸馏的全文输入）
pub(crate) mod book_file;
pub mod cda;
pub mod chapter;
pub mod chapter_rewrite;
pub mod data;
pub mod discussion;
pub mod discussion_synthesis;
pub mod expert_distill;
pub mod experts;
pub mod harness;
pub mod http;
/// LLM 输出 JSON 的容错修复（供讨论成果等结构化解析使用）
pub(crate) mod json_fix;
pub mod llm;
pub(crate) mod llm_helper;
pub mod methodology_distill;
pub mod optimize;
pub mod outline;
/// IPC 命令模块
pub mod project;
pub mod workflow_templates;
