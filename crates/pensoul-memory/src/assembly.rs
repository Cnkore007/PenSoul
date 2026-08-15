// assembly.rs — 上下文组装
// 按重要性排序、去重、压缩、格式化

use crate::types::MemoryPacket;

/// 上下文组装器
pub struct ContextAssembler;

impl ContextAssembler {
    /// 将记忆包格式化为 LLM 可读的上下文字符串
    pub fn assemble(packet: &MemoryPacket) -> String {
        let mut parts = Vec::new();

        // 实体信息
        if !packet.entities.is_empty() {
            parts.push("## 相关实体".to_string());
            for entity in &packet.entities {
                parts.push(format!(
                    "- **{}** (相关度: {:.0}%): {}",
                    entity.entity.label.as_deref().unwrap_or(&entity.entity.entity_id),
                    entity.relevance_score * 100.0,
                    entity.summary
                ));
                if !entity.details.is_empty() {
                    parts.push(format!("  {}", entity.details));
                }
            }
        }

        // 时间上下文
        if !packet.temporal_context.is_empty() {
            parts.push(format!("## 时间上下文\n{}", packet.temporal_context));
        }

        // 情感上下文
        if !packet.emotional_context.is_empty() {
            parts.push(format!("## 情感上下文\n{}", packet.emotional_context));
        }

        // 预算信息
        parts.push(format!(
            "\n---\nToken 使用: {}/{}",
            packet.total_tokens, packet.budget_used.total_tokens
        ));

        parts.join("\n\n")
    }
}
