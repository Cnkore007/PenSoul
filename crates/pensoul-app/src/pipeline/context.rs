//! 上下文组装器 — 把备忘录、本章梗概、世界观、人物、记忆包组装成各阶段 prompt。
//!
//! 对应设计稿「工具白名单的落地形态」：不做 function calling，
//! 注入面就是能力面——写作阶段没有写设定的通道，天然改不了设定。
use pensoul_core::{Chapter, NovelOntology};
use pensoul_memory::MemoryPacket;

/// 一个阶段的完整提示词
pub struct StagePrompt {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
    /// 轻量结构任务（评审 JSON / 纪要）：按模型档案关闭或降低思考，提速省费
    pub light: bool,
}

/// 截断到指定字符数（按字符而非字节，避免切坏 UTF-8）
fn cap_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max).collect();
    format!("{truncated}…")
}

/// 世界观压缩：地点与设定规则各一行式，整体约 2000 字封顶
fn world_digest(onto: &NovelOntology) -> String {
    let mut out = String::new();
    let world = &onto.world;
    if !world.name.is_empty() {
        out.push_str(&format!("世界名：{}\n", world.name));
    }
    for loc in &world.spatial_model.locations {
        out.push_str(&format!("- 地点「{}」：{}\n", loc.name, loc.description));
    }
    for rule in &world.setting_rules {
        out.push_str(&format!("- 设定「{}」：{}\n", rule.title, rule.description));
    }
    for term in &world.glossary {
        out.push_str(&format!("- 术语「{}」：{}\n", term.term, term.definition));
    }
    cap_chars(out.trim(), 2000)
}

/// 人物志压缩：名 + 特质 + 心境 + 位置 + 关系，约 1500 字封顶
fn character_digest(onto: &NovelOntology) -> String {
    let mut out = String::new();
    for ch in &onto.characters.characters {
        let traits: Vec<String> = ch
            .core_personality
            .traits
            .iter()
            .take(4)
            .map(|(name, intensity)| format!("{name}({intensity:.1})"))
            .collect();
        out.push_str(&format!(
            "- {}：特质[{}]，心境「{}」，位于「{}」\n",
            ch.name,
            traits.join("、"),
            ch.current_mood.primary,
            ch.current_location
        ));
    }
    // 关系：ID 映射成名字便于模型理解
    let name_of = |id: &pensoul_core::CharacterId| {
        onto.characters
            .characters
            .iter()
            .find(|c| c.id == *id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    for rel in &onto.characters.relationships {
        out.push_str(&format!(
            "- 关系：{} → {}（{}，强度 {:.1}）\n",
            name_of(&rel.from),
            name_of(&rel.to),
            rel.relation_type,
            rel.strength
        ));
    }
    cap_chars(out.trim(), 1500)
}

/// 记忆包压缩：热记忆前文 + 温记忆（卷摘要/角色状态/活跃伏笔）
fn memory_digest(packet: &MemoryPacket) -> String {
    let mut out = String::new();
    if !packet.hot.is_empty() {
        out.push_str("【前文节选（就近章节）】\n");
        for (i, text) in packet.hot.iter().enumerate() {
            out.push_str(&format!(
                "── 前文 {} ──\n{}\n",
                i + 1,
                cap_chars(text, 3000)
            ));
        }
    }
    if !packet.warm.volume_summary.is_empty() {
        out.push_str(&format!("【卷摘要】{}\n", packet.warm.volume_summary));
    }
    if let Some(ref states) = packet.warm.character_states
        && !states.is_empty()
    {
        out.push_str(&format!("【角色状态】{}\n", cap_chars(states, 800)));
    }
    if !packet.warm.active_foreshadows.is_empty() {
        out.push_str(&format!(
            "【活跃伏笔】{}\n",
            packet.warm.active_foreshadows.join("；")
        ));
    }
    cap_chars(out.trim(), 6000)
}

/// 写作阶段 prompt。`prev_issues` 非空表示本次是审查退回后的重写。
/// `cards` 为工作流绑定的写作技法卡注入块（load_writing_cards 拼接结果），可为空。
pub fn build_writing_prompt(
    onto: &NovelOntology,
    memo_ctx: &str,
    chapter: &Chapter,
    packet: &MemoryPacket,
    prev_issues: &[String],
    cards: &str,
) -> StagePrompt {
    let target_words = if onto.settings.chapter_target_words > 0 {
        onto.settings.chapter_target_words
    } else {
        3000
    };

    let mut system = "你是一位长篇小说作家，正在为一部连载小说撰写章节正文。\n\
        铁律：只输出章节正文本身——不输出章节标题、不输出大纲复述、不输出任何解释或元信息；\n\
        文风贴合给定题材与基调，严格承接前文情节与人物状态，不得与世界观设定矛盾。"
        .to_string();
    if !cards.is_empty() {
        system.push_str(&format!(
            "\n\n【写作技法卡】\n\
            以下是本书选定工作流绑定的写作技法卡，是你撰写正文的方法手册：\n\
            执行其「E · 执行步骤」，遵守其「B · 边界」，文风向其「I · 技法骨架」靠拢。\n\n{cards}"
        ));
    }

    let concept = &onto.core_concept;
    let mut user = String::new();
    if !memo_ctx.is_empty() {
        user.push_str(&format!("【创作备忘录】\n{memo_ctx}\n\n"));
    }
    if !concept.high_concept.is_empty() || !concept.premise.is_empty() {
        user.push_str(&format!(
            "【核心构思】\n高概念：{}\n前提：{}\n主角：{}\n基调：{}\n核心冲突：{}\n\n",
            concept.high_concept,
            concept.premise,
            concept.protagonist_hint,
            concept.tone,
            concept.central_conflict
        ));
    }
    user.push_str(&format!(
        "【本章任务】\n第 {} 章《{}》，目标约 {} 字。\n本章梗概：{}\n\n",
        chapter.chapter_no, chapter.title, target_words, chapter.summary
    ));
    let world = world_digest(onto);
    if !world.is_empty() {
        user.push_str(&format!("【世界观】\n{world}\n\n"));
    }
    let chars = character_digest(onto);
    if !chars.is_empty() {
        user.push_str(&format!("【人物志】\n{chars}\n\n"));
    }
    let memory = memory_digest(packet);
    if !memory.is_empty() {
        user.push_str(&format!("{memory}\n\n"));
    }
    if !prev_issues.is_empty() {
        user.push_str("【上次审查未通过，必须修正的问题】\n");
        for issue in prev_issues {
            user.push_str(&format!("- {issue}\n"));
        }
        user.push('\n');
    }
    user.push_str(&format!(
        "现在请撰写第 {} 章正文，约 {} 字，直接开始正文第一段。",
        chapter.chapter_no, target_words
    ));

    StagePrompt {
        system,
        user,
        // 中文 1 字 ≈ 1-2 token：正文预算 = 目标字数 ×2，再留 8192 推理余量，
        // 夹在 [16384, 32768]（推理型模型 reasoning 会额外消耗大量预算）
        max_tokens: (target_words * 2 + 8192).clamp(16384, 32768),
        // 章节正文是深度创作，保持模型默认思考强度
        light: false,
    }
}

/// 审查阶段 prompt（异模型判卷：本章正文 + 设定/人物/前章纪要对照）
/// `cards` 为审查环节绑定的技法卡（通常是文风卡），审查时对照其边界检查文风偏离。
pub fn build_review_prompt(
    onto: &NovelOntology,
    chapter: &Chapter,
    recent_briefs: &str,
    cards: &str,
) -> StagePrompt {
    let mut system = "你是一位极其严谨的网文编辑，负责章节一致性审查。\n\
        逐项核对：① 与世界观设定是否矛盾；② 人物性格/状态/位置是否连贯；\n\
        ③ 与前文情节（含前章纪要）是否矛盾；④ 时间线是否合理；⑤ 文笔是否基本通顺。\n\
        输出必须严格使用如下双通道格式，不得输出任何其他内容：\n\
        ===SIGNAL_BEGIN===\n\
        {\"score\": 0到100的整数, \"issues\": [\"问题1\", \"问题2\"]}\n\
        ===SIGNAL_END===\n\
        ===REPORT_BEGIN===\n\
        给作者看的中文审查报告（300 字以内，先结论后问题清单）\n\
        ===REPORT_END===\n\
        评分参考：90+ 优秀可直接通过；80-89 基本合格；低于 80 存在必须修正的硬伤。"
        .to_string();
    if !cards.is_empty() {
        system.push_str(&format!(
            "\n\n【文风技法卡】\n\
            本书工作流绑定了以下技法卡。除上述一致性核对外，还须对照技法卡的\n\
            「I · 技法骨架」与「B · 边界」检查文风是否严重偏离；偏离计入 issues 并扣分。\n\n{cards}"
        ));
    }

    let mut user = String::new();
    if !recent_briefs.is_empty() {
        user.push_str(&format!("【前章纪要】\n{recent_briefs}\n\n"));
    }
    let world = world_digest(onto);
    if !world.is_empty() {
        user.push_str(&format!("【世界观】\n{world}\n\n"));
    }
    let chars = character_digest(onto);
    if !chars.is_empty() {
        user.push_str(&format!("【人物志】\n{chars}\n\n"));
    }
    user.push_str(&format!(
        "【待审章节】第 {} 章《{}》（梗概：{}）\n正文：\n{}",
        chapter.chapter_no,
        chapter.title,
        chapter.summary,
        cap_chars(&chapter.content, 12000)
    ));

    StagePrompt {
        system,
        user,
        // 评审输出虽小，但需通读整章 + 推理，给足 reasoning 预算
        max_tokens: 8192,
        // 结构化 JSON 判定：关闭/降低思考，提速省费
        light: true,
    }
}

/// 回灌阶段 prompt：提炼本章纪要（JSON chapter_brief）
pub fn build_injection_prompt(chapter: &Chapter) -> StagePrompt {
    let system = "你是叙事纪要员。阅读章节正文，提炼 150 字以内的本章纪要，\n\
        覆盖：关键事件、人物状态变化、埋设或推进的伏笔。\n\
        只输出 JSON：{\"chapter_brief\": \"纪要内容\"}，不要输出其他内容。"
        .to_string();
    let user = format!(
        "第 {} 章《{}》正文：\n{}",
        chapter.chapter_no,
        chapter.title,
        cap_chars(&chapter.content, 6000)
    );
    StagePrompt {
        system,
        user,
        // 纪要输出虽小，推理型模型 reasoning 会占用大量预算
        max_tokens: 8192,
        // 150 字纪要 + JSON 包裹：关闭/降低思考，提速省费
        light: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pensoul_core::{ChapterId, ChapterStatus, ProjectId, VolumeId};

    fn make_chapter(no: i64, summary: &str, content: &str) -> Chapter {
        Chapter {
            chapter_id: ChapterId::new(format!("ch-{no}")),
            chapter_no: no,
            volume_id: VolumeId::new("vol-1"),
            title: format!("第{no}章标题"),
            summary: summary.to_string(),
            content: content.to_string(),
            word_count: content.chars().count() as u32,
            version: 1,
            status: ChapterStatus::Draft,
            consistency_score: 1.0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_writing_prompt_contains_key_sections() {
        let mut onto = NovelOntology::new(ProjectId::new("p"), String::new());
        onto.core_concept.high_concept = "测试高概念".to_string();
        onto.settings.chapter_target_words = 2000;
        let chapter = make_chapter(3, "本章梗概内容", "");
        let packet = MemoryPacket {
            hot: vec!["前一章的正文节选".to_string()],
            warm: Default::default(),
            cold: vec![],
            narrative: vec![],
            total_tokens: 0,
            budget_used: pensoul_memory::packet::get_budget_ratio(
                pensoul_memory::EditingMode::Drafting,
            ),
        };
        let prompt = build_writing_prompt(&onto, "memo: 测试", &chapter, &packet, &[], "");
        assert!(prompt.user.contains("测试高概念"));
        assert!(prompt.user.contains("本章梗概内容"));
        assert!(prompt.user.contains("前一章的正文节选"));
        assert!(prompt.user.contains("第 3 章"));
        assert_eq!(prompt.max_tokens, 16384); // 2000*2+8192=12192 → 夹到下限 16384
        // 重写时携带 issues
        let retry = build_writing_prompt(
            &onto,
            "",
            &chapter,
            &packet,
            &["时间线矛盾".to_string()],
            "",
        );
        assert!(retry.user.contains("时间线矛盾"));
    }

    #[test]
    fn test_writing_prompt_injects_cards() {
        let onto = NovelOntology::new(ProjectId::new("p"), String::new());
        let chapter = make_chapter(1, "梗概", "");
        let packet = MemoryPacket {
            hot: vec![],
            warm: Default::default(),
            cold: vec![],
            narrative: vec![],
            total_tokens: 0,
            budget_used: pensoul_memory::packet::get_budget_ratio(
                pensoul_memory::EditingMode::Drafting,
            ),
        };
        let prompt = build_writing_prompt(
            &onto,
            "",
            &chapter,
            &packet,
            &[],
            "── 技能卡「x/style」──\n卡内容",
        );
        assert!(prompt.system.contains("写作技法卡"));
        assert!(prompt.system.contains("卡内容"));
    }

    #[test]
    fn test_review_prompt_has_dual_channel_format() {
        let onto = NovelOntology::new(ProjectId::new("p"), String::new());
        let chapter = make_chapter(1, "梗概", "正文内容");
        let prompt = build_review_prompt(&onto, &chapter, "前一章纪要", "");
        assert!(prompt.system.contains("SIGNAL_BEGIN"));
        assert!(prompt.system.contains("REPORT_BEGIN"));
        assert!(prompt.user.contains("正文内容"));
        assert!(prompt.user.contains("前一章纪要"));
    }

    #[test]
    fn test_cap_chars_truncates() {
        let long: String = "字".repeat(5000);
        let capped = cap_chars(&long, 100);
        assert_eq!(capped.chars().count(), 101); // 100 字 + 省略号
    }
}
