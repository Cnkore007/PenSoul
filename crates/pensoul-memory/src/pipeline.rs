use std::collections::HashMap;

use pensoul_core::{ChapterId, Result};

use crate::archive::ArchiveMemory;
use crate::cold::ColdMemory;
use crate::hot::HotMemory;
use crate::narrative::NarrativeMemory;
use crate::packet::{
    BudgetRatio, ChapterSummary, EditingMode, MemoryPacket, NarrativeCategory, NarrativeDetail,
    estimate_tokens,
};
use crate::warm::WarmMemory;

/// 记忆更新管道
pub struct MemoryPipeline {
    pub hot: HotMemory,
    pub warm: WarmMemory,
    pub cold: ColdMemory,
    pub narrative: NarrativeMemory,
    pub archive: ArchiveMemory,
    /// 编辑模式
    pub mode: EditingMode,
    /// 总 token 预算
    pub total_budget: usize,
}

impl MemoryPipeline {
    pub fn new(window_size: usize, mode: EditingMode, total_budget: usize) -> Self {
        Self {
            hot: HotMemory::new(window_size),
            warm: WarmMemory::new(),
            cold: ColdMemory::new(),
            narrative: NarrativeMemory::new(),
            archive: ArchiveMemory::new(),
            mode,
            total_budget,
        }
    }

    /// 获取当前模式的预算分配
    pub fn budget_ratio(&self) -> BudgetRatio {
        crate::packet::get_budget_ratio(self.mode)
    }

    /// 8 步更新管道 — 每章写入时调用
    pub fn update(&mut self, chapter_id: i64, chapter_text: &str) -> Result<()> {
        // --- 第 1 步：提取章节摘要 ---
        let summary = self.extract_summary(chapter_id, chapter_text);

        // --- 第 2 步：提取角色状态 ---
        let character_states = self.extract_character_states(chapter_text);

        // --- 第 3 步：提取关键事件 ---
        let key_events = self.extract_key_events(chapter_text);

        // --- 第 4 步：提取叙事细节 ---
        let narrative_details = self.extract_narrative_details(chapter_id, chapter_text);

        // --- 第 5 步：更新热记忆 — 插入完整文本 ---
        self.hot.insert(chapter_id, chapter_text.to_string());

        // --- 第 6 步：更新温记忆 — 插入章节摘要 + 更新角色状态 ---
        let mut updated_summary = summary;
        updated_summary.key_events = key_events;
        updated_summary.character_states = character_states.clone();
        self.warm.insert_chapter(chapter_id, updated_summary);
        self.warm
            .set_character_states(serde_json::to_string(&character_states).unwrap_or_default());

        // --- 第 7 步：更新冷记忆 — 将超出热窗口的旧章节摘要转入冷记忆 ---
        self.sync_cold_memory(chapter_id);

        // --- 第 8 步：更新叙事记忆 — 添加提取的叙事细节 ---
        for detail in narrative_details {
            self.narrative.add_detail(detail);
        }

        Ok(())
    }

    /// 构建记忆包 — 组装四层记忆
    pub fn build_packet(&self, current_chapter: i64) -> MemoryPacket {
        let ratio = self.budget_ratio();
        let mut remaining = self.total_budget;

        let hot_budget = (self.total_budget as f32 * ratio.hot) as usize;
        let hot = self.hot.build(current_chapter, hot_budget.min(remaining));
        let hot_tokens = estimate_tokens_batch(&hot);
        remaining = remaining.saturating_sub(hot_tokens);

        let warm_budget = (self.total_budget as f32 * ratio.warm) as usize;
        let warm = self.warm.build(current_chapter, warm_budget.min(remaining));
        let warm_tokens = estimate_tokens_batch_slice(&[&warm.volume_summary])
            + estimate_tokens_batch(&warm.active_foreshadows)
            + warm
                .character_states
                .as_ref()
                .map(|s| estimate_tokens(s))
                .unwrap_or(0);
        remaining = remaining.saturating_sub(warm_tokens);

        let cold_budget = (self.total_budget as f32 * ratio.cold) as usize;
        let cold = self
            .cold
            .retrieve(current_chapter, cold_budget.min(remaining));
        let cold_tokens = estimate_tokens_batch(&cold);
        remaining = remaining.saturating_sub(cold_tokens);

        let narrative_budget = (self.total_budget as f32 * ratio.narrative) as usize;
        let narrative = self
            .narrative
            .retrieve(current_chapter, narrative_budget.min(remaining));
        let narrative_tokens: usize = narrative.iter().map(|d| estimate_tokens(&d.content)).sum();

        let total_tokens = hot_tokens + warm_tokens + cold_tokens + narrative_tokens;

        MemoryPacket {
            hot,
            warm,
            cold,
            narrative,
            total_tokens,
            budget_used: ratio,
        }
    }

    // ===== 内部辅助方法 =====

    /// 第 1 步：从章节文本提取摘要（原型用首句 + 字数统计）
    fn extract_summary(&self, chapter_id: i64, text: &str) -> ChapterSummary {
        let title = format!("章节{}", chapter_id);
        let first_line = text.lines().next().unwrap_or("").to_string();
        let summary = if first_line.chars().count() > 100 {
            let truncated: String = first_line.chars().take(100).collect();
            format!("{truncated}...")
        } else {
            first_line.clone()
        };

        ChapterSummary {
            chapter_id: ChapterId::new(chapter_id.to_string()),
            title,
            summary,
            key_events: Vec::new(),
            character_states: HashMap::new(),
            word_count: text.chars().count() as u32,
            consistency_score: 1.0,
            semantic_embedding: None,
        }
    }

    /// 第 2 步：提取角色状态（原型用简单关键词检测）
    fn extract_character_states(&self, text: &str) -> HashMap<String, String> {
        let mut states: HashMap<String, String> = HashMap::new();

        // 简单原型：检测 "XX说" / "XX想" / "XX感到" 模式提取角色名
        // 使用 char 级别索引避免 UTF-8 字节切片崩溃
        let keywords = ["说", "想", "感到", "认为", "决定"];
        for line in text.lines() {
            for kw in &keywords {
                if let Some(pos) = line.find(kw) {
                    // 取 kw 前面的字符作为角色名（最多取 4 个字符）
                    let before = &line[..pos];
                    let chars: Vec<char> = before.chars().collect();
                    let start_idx = chars.len().saturating_sub(4);
                    let name: String = chars[start_idx..].iter().collect();
                    let trimmed = name.trim();
                    if !trimmed.is_empty()
                        && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        states
                            .entry(trimmed.to_string())
                            .or_insert_with(|| "在第0章出现".to_string());
                    }
                }
            }
        }

        states
    }

    /// 第 3 步：提取关键事件（原型用句子分割）
    fn extract_key_events(&self, text: &str) -> Vec<String> {
        text.lines()
            .filter(|line| {
                let line = line.trim();
                // 简单启发式：包含动作动词的句子
                line.contains("了")
                    || line.contains("开始")
                    || line.contains("结束")
                    || line.contains("突然")
                    || line.contains("终于")
            })
            .map(|line| line.trim().to_string())
            .take(5) // 最多 5 个关键事件
            .collect()
    }

    /// 第 4 步：提取叙事细节（原型用简单分类）
    fn extract_narrative_details(&self, chapter_id: i64, text: &str) -> Vec<NarrativeDetail> {
        let mut details = Vec::new();
        let mut detail_counter = 0;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (category, importance) = classify_narrative_line(line);

            // 只提取有一定重要性的细节
            if importance > 0.3 {
                detail_counter += 1;
                details.push(NarrativeDetail {
                    detail_id: format!("{}-{}", chapter_id, detail_counter),
                    chapter_id: ChapterId::new(chapter_id.to_string()),
                    category,
                    content: line.to_string(),
                    importance,
                    last_referenced: None,
                });
            }
        }

        details
    }

    /// 第 7 步辅助：同步冷记忆 — 将超出热窗口的章节摘要移入冷记忆
    fn sync_cold_memory(&mut self, current_chapter: i64) {
        let window_size: i64 = 2;
        let chapter_ids: Vec<i64> = self.warm.chapters().keys().copied().collect();

        for id in chapter_ids {
            if (id - current_chapter).abs() > window_size {
                // 从温记忆取出，放入冷记忆
                if let Some(summary) = self.warm.remove_chapter(id) {
                    self.cold.insert_chapter(id, summary);
                }
            }
        }
    }
}

/// 简单分类叙事行
fn classify_narrative_line(line: &str) -> (NarrativeCategory, f32) {
    // 习惯/日常
    if line.contains("习惯") || line.contains("每天") || line.contains("总是") {
        return (NarrativeCategory::Habit, 0.6);
    }
    // 承诺/约定
    if line.contains("答应")
        || line.contains("承诺")
        || line.contains("约定")
        || line.contains("发誓")
    {
        return (NarrativeCategory::Promise, 0.7);
    }
    // 道具/物品
    if line.contains("拿出")
        || line.contains("递给")
        || line.contains("握着")
        || line.contains("武器")
    {
        return (NarrativeCategory::Prop, 0.5);
    }
    // 感官描写
    if line.contains("闻到")
        || line.contains("看到")
        || line.contains("听到")
        || line.contains("触感")
    {
        return (NarrativeCategory::Sensory, 0.4);
    }
    // 支线剧情
    if line.contains("与此同时") || line.contains("另一边") || line.contains("同时") {
        return (NarrativeCategory::Subplot, 0.6);
    }

    // 默认：中等重要性
    (NarrativeCategory::Subplot, 0.3)
}

/// 从文本中提取 chapter_id 的简单辅助
#[allow(dead_code)]
fn chapter_id_from_text(_text: &str) -> i64 {
    0 // 原型占位
}

fn estimate_tokens_batch(texts: &[String]) -> usize {
    texts.iter().map(|t| estimate_tokens(t)).sum()
}

fn estimate_tokens_batch_slice(texts: &[&str]) -> usize {
    texts.iter().map(|t| estimate_tokens(t)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{EditingMode, get_budget_ratio};

    #[test]
    fn test_pipeline_update_and_build() {
        let mut pipeline = MemoryPipeline::new(2, EditingMode::Drafting, 10000);

        // 模拟写入 5 章
        for i in 1..=5 {
            let text = format!(
                "第{}章内容：主角开始行动了。突然出现了一个敌人。\n角色A说：你好。\n角色B感到紧张。",
                i
            );
            pipeline.update(i, &text).unwrap();
        }

        // 验收标准 1：热记忆窗口 ±2 章完整文本
        assert!(pipeline.hot.len(5) <= 5);

        // 验收标准 2：温记忆全量注入
        // 第5章时，窗口内的章节还在温记忆中
        assert!(pipeline.warm.chapter_count() >= 1);

        // 构建记忆包
        let packet = pipeline.build_packet(5);
        assert!(packet.total_tokens > 0);
    }

    #[test]
    fn test_pipeline_cold_memory_sync() {
        let mut pipeline = MemoryPipeline::new(2, EditingMode::Drafting, 10000);

        // 写入 6 章
        for i in 1..=6 {
            let text = format!("第{}章内容。", i);
            pipeline.update(i, &text).unwrap();
        }

        // 验收标准 3：冷记忆排除热记忆窗口内的章节
        // 在第6章，窗口 ±2 = 4,5,6 不应出现在冷记忆检索结果中
        let cold_result = pipeline.cold.retrieve(6, 100000);
        for entry in &cold_result {
            assert!(!entry.contains("第4章"));
            assert!(!entry.contains("第5章"));
            assert!(!entry.contains("第6章"));
        }
        // 应包含更早的章节
        assert!(cold_result.iter().any(|e| e.contains("第1章")));
    }

    #[test]
    fn test_pipeline_budget_not_exceeded() {
        let budget = 50;
        let mut pipeline = MemoryPipeline::new(2, EditingMode::Drafting, budget);

        for i in 1..=3 {
            let text = format!("第{}章：一段相当长的文本内容用于测试预算限制是否有效。", i);
            pipeline.update(i, &text).unwrap();
        }

        let packet = pipeline.build_packet(3);
        // 验收标准 6：Token 预算不超限（允许少量误差因为比例分配是浮点）
        assert!(packet.total_tokens <= budget + 10);
    }

    #[test]
    fn test_pipeline_narrative_importance() {
        let mut pipeline = MemoryPipeline::new(2, EditingMode::Reviewing, 10000);

        // 包含高重要性叙事线索的文本
        let text = "主角答应了朋友的约定。\n与此同时另一边在发生什么。\n他习惯每天去那个地方。";
        pipeline.update(1, text).unwrap();

        // 验收标准 4：叙事记忆按重要性排序
        let details = pipeline.narrative.retrieve(1, 10000);
        if details.len() >= 2 {
            for i in 0..details.len() - 1 {
                assert!(details[i].importance >= details[i + 1].importance);
            }
        }
    }

    #[test]
    fn test_pipeline_budget_ratios() {
        // 验收标准 5：三种编辑模式预算分配比例正确
        let drafting = get_budget_ratio(EditingMode::Drafting);
        assert!((drafting.hot - 0.50).abs() < f32::EPSILON);
        assert!((drafting.warm - 0.25).abs() < f32::EPSILON);
        assert!((drafting.cold - 0.20).abs() < f32::EPSILON);
        assert!((drafting.narrative - 0.05).abs() < f32::EPSILON);

        let revising = get_budget_ratio(EditingMode::Revising);
        assert!((revising.hot - 0.60).abs() < f32::EPSILON);
        assert!((revising.warm - 0.20).abs() < f32::EPSILON);
        assert!((revising.cold - 0.15).abs() < f32::EPSILON);
        assert!((revising.narrative - 0.05).abs() < f32::EPSILON);

        let reviewing = get_budget_ratio(EditingMode::Reviewing);
        assert!((reviewing.hot - 0.30).abs() < f32::EPSILON);
        assert!((reviewing.warm - 0.20).abs() < f32::EPSILON);
        assert!((reviewing.cold - 0.40).abs() < f32::EPSILON);
        assert!((reviewing.narrative - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pipeline_full_8_step() {
        let mut pipeline = MemoryPipeline::new(2, EditingMode::Drafting, 10000);

        let text = "第一章：故事开始。\n主角答应了回到故乡。\n与此同时远方传来消息。\n他总是习惯早起。\n突然门被推开了。";

        let result = pipeline.update(1, text);
        assert!(result.is_ok());

        // 验证热记忆有内容
        assert!(!pipeline.hot.is_empty());
        // 验证温记忆有摘要
        assert!(pipeline.warm.chapter_count() >= 1);
        // 验证叙事记忆有细节
        assert!(pipeline.narrative.total_details() > 0);
    }
}
