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

/// 项目写作经验库压缩：历史错误经验清单，审查时重点检查是否重犯
fn lessons_digest(onto: &NovelOntology) -> String {
    if onto.writing_lessons.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::new();
    for l in onto.writing_lessons.iter().take(30) {
        let count_mark = if l.count > 1 {
            format!("（已发生 {} 次）", l.count)
        } else {
            String::new()
        };
        let example_mark = if l.example.is_empty() {
            String::new()
        } else {
            format!("（出自{}）", l.example)
        };
        let fix = if l.fix.is_empty() {
            String::new()
        } else {
            format!("；改正：{}", l.fix)
        };
        lines.push(format!(
            "- [{}]{count_mark} {}{example_mark}{fix}",
            l.category, l.problem
        ));
    }
    cap_chars(&lines.join("\n"), 3000)
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

/// 章前策划 prompt：生成本章节拍表（JSON）。
/// `manual` 为模板阶段手册，约束策划规则。
pub fn build_planning_prompt(
    onto: &NovelOntology,
    memo_ctx: &str,
    chapter: &Chapter,
    packet: &MemoryPacket,
    manual: &str,
) -> StagePrompt {
    let mut system = String::from(
        "你是一位深谙网文节奏的责编，负责在动笔前为每一章做「章前策划」。\n\
        铁律：非终局章节禁止解决主线核心冲突；每章必须新增至少一个未解决的次要问题；\n\
        结尾必须留断章钩子（疑问型：读者想知道答案；危机型：刀悬在脖子上；转折型：预期被翻转）。\n\
        产出必须是一份可直接执行的节拍表 JSON，只输出 JSON 本身，不要任何解释。\n\
        JSON 结构：\n\
        {\n\
          \"章节目标\": \"本章要达成的叙事目标（一句话）\",\n\
          \"开场钩子\": \"开场场景与第一句的钩子设计\",\n\
          \"场景节拍\": [\n\
            {\"场景\": \"场景一名称\", \"目标\": \"本场景目标\", \"冲突\": \"阻碍与对抗\", \"状态变化\": \"场景结束时人物的状态变化\", \"建议字数\": 600}\n\
          ],\n\
          \"爽点与情绪释放\": \"本章的爽点或情绪释放点\",\n\
          \"新增未解决问题\": \"本章必须新增的次要问题\",\n\
          \"结尾断章钩子\": \"疑问型/危机型/转折型：具体钩子\",\n\
          \"伏笔\": {\"埋设\": [\"...\"], \"回收\": [\"...\"]},\n\
          \"人物状态变化\": [\"人物A：变化\"]\n\
        }",
    );
    if !manual.trim().is_empty() {
        system.push_str(&format!("\n\n【策划守则】\n{manual}"));
    }

    let mut user = String::new();
    if !memo_ctx.is_empty() {
        user.push_str(&format!("【创作备忘录】\n{memo_ctx}\n\n"));
    }
    let concept = &onto.core_concept;
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
        "【本章任务】\n第 {} 章《{}》（全书目标约 {} 章）。\n本章梗概：{}\n\n",
        chapter.chapter_no, chapter.title, onto.settings.target_chapters, chapter.summary
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
    user.push_str(&format!(
        "请输出第 {} 章的节拍表 JSON。",
        chapter.chapter_no
    ));

    StagePrompt {
        system,
        user,
        // 策划需要一定推理量，但比正文小
        max_tokens: 8192,
        light: false,
    }
}

/// 写作阶段 prompt。`prev_issues` 非空表示本次是审查退回后的重写。
/// `beat_plan` 为章前策划产出的节拍表（JSON 文本，可为空表示无策划）。
/// `cards` 为工作流绑定的写作技法卡注入块（load_writing_cards 拼接结果），可为空。
#[allow(clippy::too_many_arguments)]
pub fn build_writing_prompt(
    onto: &NovelOntology,
    memo_ctx: &str,
    chapter: &Chapter,
    packet: &MemoryPacket,
    prev_issues: &[String],
    beat_plan: &str,
    cards: &str,
    anti_ai: &str,
    style_block: &str,
) -> StagePrompt {
    let target_words = if onto.settings.chapter_target_words > 0 {
        onto.settings.chapter_target_words
    } else {
        3000
    };

    let mut system = "你是一位长篇小说作家，正在为一部连载小说撰写章节正文。\n\
        铁律：只输出章节正文本身——不输出章节标题、不输出大纲复述、不输出任何解释或元信息；\n\
        文风贴合给定题材与基调，严格承接前文情节与人物状态，不得与世界观设定矛盾。\n\
        输出协议：正文必须严格包裹在 ===CHAPTER_BEGIN=== 与 ===CHAPTER_END=== 两个标记之间；\n\
        标记之外不得出现任何内容——不输出英文规划、不输出场景说明、不输出思考过程、不输出节拍表复述。"
        .to_string();
    system.push_str(&format!("\n\n{anti_ai}"));
    if !style_block.trim().is_empty() {
        system.push_str(&format!("\n\n{style_block}"));
    }
    // 开篇黄金三章：前 3 章用「立刻出事 → 给期待 → 给爽点」节奏
    if chapter.chapter_no <= 3 {
        system.push_str(
            "\n\n【开篇黄金三章】第 1 章必须在 300 字内抛出核心事件（重生/穿越/系统降临/身死危机等），\
             把读者钉住；第 2 章亮出金手指并制造期待与反转；第 3 章释放第一个爽点（用金手指解决麻烦或首次打脸）。\
             前三章人物要少，避免多线铺陈。",
        );
    }
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
    if !beat_plan.trim().is_empty() {
        user.push_str(&format!("【本章节拍表】\n{beat_plan}\n\n"));
    }
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

/// 审查阶段 prompt（异模型判卷：本章正文 + 节拍表 + 设定/人物/前章纪要对照）
/// `cards` 为审查环节绑定的技法卡（通常是文风卡），审查时对照其边界检查文风偏离。
/// `golden` 为真时启用「黄金三章」硬门控：SIGNAL 必须额外输出 hook/payoff 两个
/// 0-10 子分数，引擎按 `score >= 阈值 && hook >= 8 && payoff >= 8` 判定。
#[allow(clippy::too_many_arguments)] // prompt 组装函数参数多属正常，保持显式便于调用点对齐
pub fn build_review_prompt(
    onto: &NovelOntology,
    chapter: &Chapter,
    recent_briefs: &str,
    beat_plan: &str,
    cards: &str,
    golden: bool,
    anti_ai: &str,
    style_block: &str,
) -> StagePrompt {
    let signal_schema = if golden {
        "{\"score\": 0到100的整数, \"hook\": 0到10的整数, \"payoff\": 0到10的整数, \"issues\": [\"问题1\", \"问题2\"], \"diagnosis\": [{\"family\": \"问题族\", \"trigger\": \"触发点（命中的词/句子）\", \"action\": \"建议动作\", \"rewrite\": true}]}"
    } else {
        "{\"score\": 0到100的整数, \"issues\": [\"问题1\", \"问题2\"], \"diagnosis\": [{\"family\": \"问题族\", \"trigger\": \"触发点（命中的词/句子）\", \"action\": \"建议动作\", \"rewrite\": true}]}"
    };
    let mut system = format!(
        "你是一位极其严谨的网文编辑，负责章节质量审查。按七维加权打分：\n\
        ① 卖点兑现（20 分）：本章是否兑现作品核心卖点，还是跑偏成另一本书；\n\
        ② 开场钩子（10 分）：前 300 字是否出现冲突或悬念；\n\
        ③ 情绪曲线与爽点（20 分）：压抑→释放是否成立，爽点是否前置且具体；\n\
        ④ 场景与节奏（10 分）：每个场景是否有目标/冲突/状态变化，节奏是否拖沓；\n\
        ⑤ 断章钩子（15 分）：结尾是否停在疑问/危机/转折钩子上；\n\
        ⑥ 人物与设定一致性（15 分）：人物性格/状态/位置、世界观、时间线是否连贯；\n\
        ⑦ 文笔与反 AI 味（10 分）：从 10 分起按标准扣分——\n\
        a) AI 套话（不禁/仿佛/映入眼帘/心中暗道/嘴角微扬/勾起一抹/目光如炬/此时此刻等）每处扣 0.5 分；\n\
        b) 弱化副词（微微/淡淡/缓缓/轻轻/悄然/默默/隐隐）每千字超过 3 个后每处扣 0.5 分；\n\
        c) 书面连接词（与此同时/从而/诚然/由此可见/值得注意的是）每处扣 0.5 分；\n\
        d) 意义膨胀词（意义深远/前所未有/可谓/未来可期）每处扣 0.5 分；\n\
        e) 情绪直说（他感到…/心中涌起…/一股寒意…）每处扣 0.5 分；\n\
        f) 排比三连（三个词一组堆「全面感」）每处扣 0.5 分；\n\
        扣完为止最低 0 分；若用具体细节代替判断、长短句有节奏变化，可加回 0-2 分。\n\
        若有章前策划节拍表，须核对是否按策划执行，明显偏离计入问题清单并扣分。\n\
        输出必须严格使用如下双通道格式，不得输出任何其他内容：\n\
        ===SIGNAL_BEGIN===\n\
        {signal_schema}\n\
        ===SIGNAL_END===\n\
        ===REPORT_BEGIN===\n\
        给作者看的中文审查报告（300 字以内，先结论后问题清单）\n\
        ===REPORT_END===\n\
        评分参考：90+ 优秀可直接通过；80-89 基本合格；低于 80 存在必须修正的问题。"
    );
    system.push_str(
        "\n\n【诊断报告要求】\n\
        issues 只写问题本身（一句一个）；diagnosis 给每条实质性问题附四字段诊断：\n\
        - family：问题族（如 人物一致性 / 设定矛盾 / 断章钩子 / 结构骨架 / 翻译腔 / 文笔套话 / 节奏问题）；\n\
        - trigger：触发点，必须引用原文具体词、句式或局部句子（不超过 40 字），不得泛泛而谈；\n\
        - action：建议动作（删掉 / 换成具体表达 / 补充前文信息 / 调整结构 / 保持不动等），必须可执行；\n\
        - rewrite：是否建议改写（布尔）。\n\
        没有实质问题可留空数组；每章诊断最多 8 条，按严重度排序。",
    );
    if golden {
        system.push_str(
            "\n\n【黄金三章硬门控（前 3 章强制）】\n\
            本阶段引擎按「总分达标 且 开场钩子 ≥ 8 且 爽点 ≥ 8」放行，任一项不达标即拦截重写：\n\
            - hook（0-10）：前 300 字是否出现核心事件/危机/悬念，把读者钉住；\n\
            - payoff（0-10）：本章是否有具体可感的爽点或情绪释放（打脸/反杀/奇遇/危机解决/情感击中）。\n\
            必须诚实给分，不得因总分高而虚高子分数。",
        );
    }
    if !beat_plan.trim().is_empty() {
        system.push_str(&format!("\n\n【本章节拍表】\n{beat_plan}"));
    }
    if !cards.is_empty() {
        system.push_str(&format!(
            "\n\n【文风技法卡】\n\
            本书工作流绑定了以下技法卡。除上述一致性核对外，还须对照技法卡的\n\
            「I · 技法骨架」与「B · 边界」检查文风是否严重偏离；偏离计入 issues 并扣分。\n\n{cards}"
        ));
    }
    if !anti_ai.trim().is_empty() {
        system.push_str(&format!(
            "\n\n【语言铁律（反 AI 味，项目配置）】\n{anti_ai}"
        ));
    }
    if !style_block.trim().is_empty() {
        system.push_str(&format!("\n\n{style_block}"));
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
    let lessons = lessons_digest(onto);
    if !lessons.is_empty() {
        user.push_str(&format!(
            "【本书历史写作经验（必须重点检查本章是否重犯同类错误，发现即计入 issues）】\n{lessons}\n\n"
        ));
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
            annotations: Vec::new(),
            revisions: Vec::new(),
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
        let prompt = build_writing_prompt(
            &onto,
            "memo: 测试",
            &chapter,
            &packet,
            &[],
            "",
            "",
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(prompt.user.contains("测试高概念"));
        assert!(prompt.user.contains("本章梗概内容"));
        assert!(prompt.user.contains("前一章的正文节选"));
        assert!(prompt.user.contains("第 3 章"));
        assert!(prompt.system.contains("反 AI 味"));
        assert_eq!(prompt.max_tokens, 16384); // 2000*2+8192=12192 → 夹到下限 16384
        // 重写时携带 issues
        let retry = build_writing_prompt(
            &onto,
            "",
            &chapter,
            &packet,
            &["时间线矛盾".to_string()],
            "",
            "",
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(retry.user.contains("时间线矛盾"));
        // 携带节拍表
        let planned = build_writing_prompt(
            &onto,
            "",
            &chapter,
            &packet,
            &[],
            "{\"章节目标\": \"脱困\"}",
            "",
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(planned.user.contains("【本章节拍表】"));
        // 前 3 章注入黄金三章规则
        assert!(planned.system.contains("开篇黄金三章"));
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
            "",
            "── 技能卡「x/style」──\n卡内容",
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(prompt.system.contains("写作技法卡"));
        assert!(prompt.system.contains("卡内容"));
    }

    #[test]
    fn test_planning_prompt_contains_schema() {
        let mut onto = NovelOntology::new(ProjectId::new("p"), String::new());
        onto.settings.target_chapters = 300;
        let chapter = make_chapter(1, "本章梗概", "");
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
        let prompt = build_planning_prompt(&onto, "", &chapter, &packet, "策划守则内容");
        assert!(prompt.user.contains("本章梗概"));
        assert!(prompt.system.contains("场景节拍"));
        assert!(prompt.system.contains("结尾断章钩子"));
        assert!(prompt.system.contains("策划守则内容"));
        assert!(prompt.system.contains("非终局章节禁止解决主线核心冲突"));
    }

    #[test]
    fn test_review_prompt_has_dual_channel_format() {
        let onto = NovelOntology::new(ProjectId::new("p"), String::new());
        let chapter = make_chapter(1, "梗概", "正文内容");
        let prompt = build_review_prompt(
            &onto,
            &chapter,
            "前一章纪要",
            "{\"章节目标\": \"脱困\"}",
            "",
            false,
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(prompt.system.contains("SIGNAL_BEGIN"));
        assert!(prompt.system.contains("REPORT_BEGIN"));
        assert!(prompt.system.contains("卖点兑现"));
        assert!(prompt.system.contains("【本章节拍表】"));
        assert!(prompt.user.contains("正文内容"));
        assert!(prompt.user.contains("前一章纪要"));
    }

    #[test]
    fn test_review_prompt_golden_gate() {
        let onto = NovelOntology::new(ProjectId::new("p"), String::new());
        let chapter = make_chapter(1, "梗概", "正文内容");
        let golden = build_review_prompt(
            &onto,
            &chapter,
            "",
            "",
            "",
            true,
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(golden.system.contains("黄金三章硬门控"));
        assert!(golden.system.contains("\"hook\""));
        assert!(golden.system.contains("\"payoff\""));
        let normal = build_review_prompt(
            &onto,
            &chapter,
            "",
            "",
            "",
            false,
            crate::anti_ai::DEFAULT_PROMPT,
            "",
        );
        assert!(!normal.system.contains("黄金三章硬门控"));
        assert!(!normal.system.contains("\"hook\""));
    }

    #[test]
    fn test_cap_chars_truncates() {
        let long: String = "字".repeat(5000);
        let capped = cap_chars(&long, 100);
        assert_eq!(capped.chars().count(), 101); // 100 字 + 省略号
    }
}
