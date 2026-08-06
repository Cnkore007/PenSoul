//! 开书定盘命令：讨论成果 → 蓝图（六张账本 + 实体档案骨架）+ 确定性检查
//!
//! 一稿采用「确定性映射」：直接复用讨论提炼的结构化候选（地点/规则/人物/情节），
//! 不调 LLM；LLM 账本化转换与运行态自动结算留待后续阶段。

use std::collections::HashMap;

use pensoul_core::{
    BlueprintForeshadow, BlueprintReport, BookBlueprint, CharacterMatrixEntry, CheckIssue,
    Commitment, CurrentState, DiscussionSynthesis, EntityDossier, MatrixArcStage, NovelOntology,
    ResourceEntry, Subplot, SubplotItem, VolumeBeat, VolumeBlueprint,
};
use serde_json::json;

use crate::state::AppState;

use super::blueprint_llm::llm_convert_blueprint;
use super::llm_helper as lh;
use super::story_modules::StoryModule;

/// 开书定盘：把最近一次讨论的提炼成果确定性映射为蓝图并落盘
#[tauri::command]
pub async fn settle_blueprint(
    state: tauri::State<'_, AppState>,
    reference_modules: Option<Vec<StoryModule>>,
) -> Result<BookBlueprint, String> {
    lh::ensure_api_keys_loaded(&state);
    let (fallback, syn) = {
        let onto = state.ontology.read();
        let fallback = build_blueprint(&onto)?;
        let syn = onto
            .sprout
            .last_discussion
            .as_ref()
            .map(|r| r.synthesis.clone())
            .unwrap_or_default();
        (fallback, syn)
    };
    // LLM 账本化转换（承诺精炼/卷清洗/副线识别/伏笔锚点）；失败自动回退确定性映射
    let refs = reference_modules.unwrap_or_default();
    let bp = match llm_convert_blueprint(&state, &syn, &fallback, &refs).await {
        Ok(mut converted) => {
            converted.settled = true;
            converted.settled_at = now();
            converted.settled_from = fallback.settled_from.clone();
            converted.source_stamp = fallback.source_stamp.clone();
            converted
        }
        Err(_) => fallback,
    };
    {
        let mut onto = state.ontology.write();
        onto.blueprint = bp.clone();
    }
    state.save().map_err(|e| format!("保存蓝图失败: {e}"))?;
    Ok(bp)
}

/// 获取当前蓝图（未定盘时返回空结构，前端据 settled 显示引导）
#[tauri::command]
pub async fn get_blueprint(state: tauri::State<'_, AppState>) -> Result<BookBlueprint, String> {
    let onto = state.ontology.read();
    Ok(onto.blueprint.clone())
}

/// 保存蓝图（用户编辑后的账本内容落盘）
#[tauri::command]
pub async fn save_blueprint(
    state: tauri::State<'_, AppState>,
    blueprint: BookBlueprint,
) -> Result<(), String> {
    {
        let mut onto = state.ontology.write();
        onto.blueprint = blueprint;
    }
    state.save().map_err(|e| format!("保存蓝图失败: {e}"))
}

/// 确定性检查引擎：对蓝图跑 H/S 规则，返回问题清单
#[tauri::command]
pub async fn check_blueprint(
    state: tauri::State<'_, AppState>,
) -> Result<BlueprintReport, String> {
    let onto = state.ontology.read();
    Ok(run_checks(&onto))
}

/// 从本体构造蓝图（确定性映射，无 LLM 调用）
pub(crate) fn build_blueprint(onto: &NovelOntology) -> Result<BookBlueprint, String> {
    let syn = onto
        .sprout
        .last_discussion
        .as_ref()
        .map(|r| &r.synthesis);
    let syn = syn.ok_or("还没有讨论成果，请先在「灵魂萌芽」完成一次讨论与成果提炼")?;
    build_blueprint_from_syn(syn, onto)
}

/// 从讨论合成数据构造蓝图（确定性映射，供讨论定盘与续写反推共用）
pub(crate) fn build_blueprint_from_syn(
    syn: &DiscussionSynthesis,
    onto: &NovelOntology,
) -> Result<BookBlueprint, String> {
    if syn.summary.trim().is_empty()
        && syn.characters.is_empty()
        && syn.outline_beats.is_empty()
        && syn.locations.is_empty()
        && syn.setting_rules.is_empty()
    {
        return Err("讨论成果为空，无法定盘。请先完成讨论并生成成果。".to_string());
    }

    let written = onto
        .chapters
        .iter()
        .filter(|c| c.word_count > 0)
        .map(|c| c.chapter_no)
        .max()
        .unwrap_or(0);

    let commitments = build_commitments(syn);
    let volumes = build_volumes(&syn.outline_beats);
    let character_matrix = build_character_matrix(syn);
    let foreshadows = build_foreshadows(&syn.outline_beats);
    let subplots = build_subplots(&syn.outline_beats);
    let resources = onto
        .world
        .item_graph
        .iter()
        .map(|item| ResourceEntry {
            resource_id: item.item_id.clone(),
            name: item.name.clone(),
            rtype: "item".to_string(),
            owner: item.owner.clone(),
            status: "available".to_string(),
            note: item.description.clone(),
            sources: vec!["世界观·物品图".to_string()],
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let dossiers = build_dossiers(syn);
    let current_state = build_current_state(syn, &dossiers, &subplots, written);

    Ok(BookBlueprint {
        settled: true,
        settled_at: now(),
        settled_from: truncate(&syn.summary, 60),
        source_stamp: synthesis_stamp(syn),
        commitments: refine_commitments(commitments),
        volumes,
        character_matrix: character_matrix
            .into_iter()
            .filter(|m| m.role != "group")
            .collect(),
        foreshadows,
        subplots: if syn.subplots.is_empty() {
            subplots
        } else {
            map_subplots(&syn.subplots)
        },
        resources,
        dossiers,
        current_state,
    })
}

/// 承诺账本：优先用承诺维度的提炼条目；为空时回退到设定规则的禁令启发式提取
fn build_commitments(syn: &DiscussionSynthesis) -> Vec<Commitment> {
    if !syn.commitments.is_empty() {
        return syn
            .commitments
            .iter()
            .enumerate()
            .map(|(i, c)| Commitment {
                commitment_id: format!("cmt-{:03}", i + 1),
                statement: c.statement.clone(),
                kind: if c.kind.is_empty() {
                    "rule".to_string()
                } else {
                    c.kind.clone()
                },
                priority: 2,
                scope: if c.scope.is_empty() {
                    "book".to_string()
                } else {
                    c.scope.clone()
                },
                resolution_chapter: None,
                ongoing: c.ongoing,
                status: "active".to_string(),
                sources: c.sources.clone(),
            })
            .collect();
    }
    let mut out = Vec::new();
    for rule in &syn.setting_rules {
        let text = format!("{}：{}", rule.name, rule.description);
        if is_prohibition(&text) {
            out.push(new_commitment(out.len(), text, rule.sources.clone()));
        }
    }
    out
}

fn new_commitment(idx: usize, statement: String, sources: Vec<String>) -> Commitment {
    Commitment {
        commitment_id: format!("cmt-{:03}", idx + 1),
        statement,
        kind: "rule".to_string(),
        priority: 2,
        scope: "book".to_string(),
        resolution_chapter: None,
        ongoing: true,
        status: "active".to_string(),
        sources,
    }
}

/// 铁律/禁区判定词表
fn is_prohibition(text: &str) -> bool {
    [
        "不得", "不能", "禁止", "不许", "绝不", "永不", "不可", "不允许", "上限", "必须",
        "无法做到",
    ]
    .iter()
    .any(|w| text.contains(w))
}

/// 承诺精炼：截断超长文本、按主题前缀去重（裁决复读不再进入承诺）
pub(crate) fn refine_commitments(items: Vec<Commitment>) -> Vec<Commitment> {
    let mut out: Vec<Commitment> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for mut c in items {
        if c.statement.chars().count() > 60 {
            c.statement = truncate(&c.statement, 60);
        }
        let key: String = c.statement.chars().take(8).collect();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(c);
    }
    out
}

/// 副线映射：讨论提炼条目 → 账本条目
fn map_subplots(items: &[SubplotItem]) -> Vec<Subplot> {
    items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (start, end) = parse_chapter_range(&s.chapter_range)
                .map(|(a, b)| (a, Some(b)))
                .unwrap_or((0, None));
            Subplot {
                subplot_id: format!("sp-{:03}", i + 1),
                name: s.name.clone(),
                line_tags: vec![s.name.clone()],
                mainline_relation: s.mainline_relation.clone(),
                status: "active".to_string(),
                start_chapter: start,
                end_chapter: end,
                characters: s.characters.clone(),
                last_touched_chapter: start,
                touch_interval_limit: 3,
                open_threads: s.open_threads.clone(),
                sources: s.sources.clone(),
            }
        })
        .collect()
}

/// 来源指纹：轻量摘要，前端据此提示「讨论成果已更新」
pub(crate) fn synthesis_stamp(syn: &DiscussionSynthesis) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        syn.characters.len(),
        syn.outline_beats.len(),
        syn.setting_rules.len(),
        syn.locations.len(),
        syn.summary.chars().count()
    )
}

/// 结构骨架：按卷分组情节脉络，生成正式分卷蓝图。
/// 卷名先清洗（跨卷标记归入起始卷、「各卷」等非规范卷名不建卷），
/// 卷顺序按卷号排列，保证「第一卷 → 第二卷 → …」连续。
fn build_volumes(beats: &[pensoul_core::OutlineBeat]) -> Vec<VolumeBlueprint> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&pensoul_core::OutlineBeat>> = HashMap::new();
    for beat in beats {
        let Some(key) = normalize_volume(&beat.volume) else {
            // 「各卷」等跨卷标记：不建独立卷，节点保留在情节脉络中
            continue;
        };
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(beat);
    }
    // 卷号解析：按数字排序（第一卷=1），避免按出现顺序错位
    order.sort_by_key(|k| volume_no_of(k).unwrap_or(99));
    let total = order.len();
    let mut out = Vec::with_capacity(total);
    for (idx, key) in order.iter().enumerate() {
        let g = &groups[key];
        let (mut min_s, mut max_e) = (i64::MAX, 0i64);
        for b in g {
            if let Some((s, e)) = parse_chapter_range(&b.chapter_hint) {
                min_s = min_s.min(s);
                max_e = max_e.max(e);
            }
        }
        let climax = g.iter().find(|b| b.beat_type.contains("高潮"));
        let last = *g.last().unwrap_or(&g[0]);
        out.push(VolumeBlueprint {
            volume_no: (idx + 1) as u32,
            title: key.clone(),
            one_line: g
                .first()
                .map(|b| truncate(&b.description, 60))
                .unwrap_or_default(),
            function: if idx == 0 {
                "setup"
            } else if idx + 1 == total {
                "resolution"
            } else {
                "escalation"
            }
            .to_string(),
            chapter_start: if min_s == i64::MAX { 0 } else { min_s },
            chapter_end: max_e,
            climax_scene: climax
                .map(|b| truncate(&b.title, 40))
                .unwrap_or_default(),
            climax_chapter: climax
                .and_then(|b| parse_chapter_range(&b.chapter_hint).map(|(s, _)| s)),
            volume_hook: last.hook.clone(),
            beats: build_initial_beats(
                if min_s == i64::MAX { 0 } else { min_s },
                max_e,
                climax.map(|b| truncate(&b.title, 40)).unwrap_or_default(),
                climax.and_then(|b| parse_chapter_range(&b.chapter_hint).map(|(s, _)| s)),
                last.hook.clone(),
            ),
            status: "planned".to_string(),
            ..Default::default()
        });
    }
    out
}

/// 从卷已有字段投影初始节奏点（高潮 + 卷末钩子；不臆造其他节奏）
fn build_initial_beats(
    chapter_start: i64,
    chapter_end: i64,
    climax_scene: String,
    climax_chapter: Option<i64>,
    volume_hook: String,
) -> Vec<VolumeBeat> {
    let mut beats = Vec::new();
    if let Some(cc) = climax_chapter {
        beats.push(VolumeBeat {
            beat_id: format!("bt-climax-{cc}"),
            beat_type: "climax".to_string(),
            chapter: cc,
            note: climax_scene,
            links: vec![],
        });
    }
    if !volume_hook.is_empty() && chapter_end > 0 && chapter_end >= chapter_start {
        beats.push(VolumeBeat {
            beat_id: format!("bt-end-{chapter_end}"),
            beat_type: "hook_end".to_string(),
            chapter: chapter_end,
            note: volume_hook,
            links: vec![],
        });
    }
    beats
}

/// 卷名规范化：「第一卷末至第二卷」→「第一卷」；「各卷」等非规范卷名 → None
fn normalize_volume(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return Some("第1卷".to_string());
    }
    if s.contains("各卷") || s.contains("全书") || s.contains("全程") {
        return None;
    }
    // 提取「第X卷」；优先第 X 卷
    let chars: Vec<char> = s.chars().collect();
    let mut best: Option<u32> = None;
    for (i, ch) in chars.iter().enumerate() {
        if *ch == '卷' && i >= 1 {
            // 向前找数字（第X卷 / X卷，支持「第一卷」汉字数字与「第2卷」阿拉伯数字）
            let mut j = i as isize - 1;
            let mut digits = String::new();
            while j >= 0
                && (chars[j as usize].is_ascii_digit()
                    || "一二三四五六七八九十零".contains(chars[j as usize]))
            {
                digits.insert(0, chars[j as usize]);
                j -= 1;
            }
            if digits.is_empty() {
                continue;
            }
            if let Some(n) = parse_chinese_number(&digits).filter(|n| *n > 0) {
                best = Some(best.map_or(n, |b| b.min(n)));
            }
        }
    }
    best.map(|n| format!("第{n}卷"))
}

/// 卷号解析：支持「1」「一」「十二」「二十一」等（1-99）
fn parse_chinese_number(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse().ok();
    }
    let digit = |c: char| match c {
        '零' => Some(0),
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    };
    let chars: Vec<char> = s.chars().collect();
    match chars.as_slice() {
        ['十'] => Some(10),
        ['十', c] => digit(*c).map(|d| 10 + d),
        [c, '十'] => digit(*c).map(|d| d * 10),
        [a, '十', b] => digit(*a).and_then(|x| digit(*b).map(|y| x * 10 + y)),
        [c] => digit(*c),
        _ => None,
    }
}

/// 卷名 → 卷号（「第一卷」→ 1）
fn volume_no_of(name: &str) -> Option<u32> {
    let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    parse_chinese_number(&digits)
}

/// 人物矩阵：讨论人物条目映射（第一个角色默认主角，其余默认盟友）
fn build_character_matrix(syn: &DiscussionSynthesis) -> Vec<CharacterMatrixEntry> {
    syn.characters
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            // 群像/派系不进人物矩阵（讨论提炼应尽量只给个体，这里兜底过滤）
            if c.entity_kind == "group" || c.entity_kind == "faction" {
                return None;
            }
            Some(CharacterMatrixEntry {
            character_name: c.name.clone(),
            role: if i == 0 {
                "protagonist".to_string()
            } else {
                "ally".to_string()
            },
            core_values: c
                .personality_traits
                .iter()
                .take(2)
                .map(|(name, _)| name.clone())
                .collect(),
            speech_style: c.speech_style.clone(),
            wants: c.wants.clone(),
            fears: c.fears.clone(),
            secret: c.secret.clone(),
            arc: c
                .arc
                .iter()
                .map(|s| MatrixArcStage {
                    name: s.name.clone(),
                    chapter_range: s.chapter_range.clone(),
                    goal: s.goal.clone(),
                    turning_point: s.trait_desc.clone(),
                })
                .collect(),
            knows: c.knows.clone(),
            does_not_know: c.does_not_know.clone(),
            sources: c.sources.clone(),
            ..Default::default()
            })
        })
        .collect()
}

/// 伏笔账本：展开各情节节点中的伏笔计划
fn build_foreshadows(beats: &[pensoul_core::OutlineBeat]) -> Vec<BlueprintForeshadow> {
    let mut out = Vec::new();
    for beat in beats {
        let planted = parse_chapter_range(&beat.chapter_hint)
            .map(|(s, _)| s)
            .unwrap_or(0);
        for f in &beat.foreshadowing {
            let (anchor_type, anchor) = if !f.payoff_anchor_type.is_empty() {
                (f.payoff_anchor_type.clone(), f.payoff_anchor.clone())
            } else if !f.payoff_hint.trim().is_empty() {
                // 兜底：从文本识别「第N章」/「第N卷」，否则视为事件锚点
                let (ty, anchor) = classify_payoff_hint(&f.payoff_hint);
                (ty, anchor)
            } else {
                (String::new(), String::new())
            };
            out.push(BlueprintForeshadow {
                foreshadow_id: format!("fs-{:03}", out.len() + 1),
                name: truncate(&f.plant, 24),
                description: f.plant.clone(),
                kind: "line".to_string(),
                planted_chapter: planted,
                expected_payoff_chapter: match anchor_type.as_str() {
                    "chapter" => parse_chapter_number(&anchor).unwrap_or(0),
                    _ => 0,
                },
                payoff_anchor_type: anchor_type,
                payoff_anchor: anchor,
                status: "planned".to_string(),
                sources: beat.sources.clone(),
                ..Default::default()
            });
        }
    }
    out
}

/// 兜底识别回收锚点：含「第N章」→ chapter；含「第N卷」→ volume；否则 event
fn classify_payoff_hint(hint: &str) -> (String, String) {
    let h = hint.trim();
    if h.is_empty() {
        return (String::new(), String::new());
    }
    if h.contains("章") && let Some(n) = parse_chapter_number(h) {
        return ("chapter".to_string(), format!("第{n}章"));
    }
    if h.contains("卷") && let Some(n) = parse_chapter_number(h) {
        return ("volume".to_string(), format!("第{n}卷"));
    }
    ("event".to_string(), truncate(h, 30))
}

/// 副线账本：按情节线标签聚合（排除「主线/副线」泛标签）
fn build_subplots(beats: &[pensoul_core::OutlineBeat]) -> Vec<Subplot> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, Vec<&pensoul_core::OutlineBeat>> = HashMap::new();
    for beat in beats {
        for tag in &beat.line_tags {
            let t = tag.trim();
            if t.is_empty() || t == "主线" || t == "副线" {
                continue;
            }
            if !map.contains_key(t) {
                order.push(t.to_string());
            }
            map.entry(t.to_string()).or_default().push(beat);
        }
    }
    let mut out = Vec::new();
    for tag in order {
        let Some(g) = map.get(&tag) else { continue };
        let Some(first) = g.first() else { continue };
        let last = *g.last().unwrap_or(first);
        let start = parse_chapter_range(&first.chapter_hint)
            .map(|(s, _)| s)
            .unwrap_or(0);
        let end = parse_chapter_range(&last.chapter_hint).map(|(_, e)| e);
        out.push(Subplot {
            subplot_id: format!("sp-{:03}", out.len() + 1),
            name: tag.clone(),
            line_tags: vec![tag.clone()],
            mainline_relation: "待定：关联主线阶段".to_string(),
            status: "active".to_string(),
            start_chapter: start,
            end_chapter: end,
            last_touched_chapter: start,
            touch_interval_limit: 3,
            open_threads: Vec::new(),
            characters: Vec::new(),
            sources: Vec::new(),
        });
    }
    out
}

/// 实体档案骨架：人物 + 地点各一张卡（运行态结算前仅静态快照）
pub(crate) fn build_dossiers(syn: &DiscussionSynthesis) -> Vec<EntityDossier> {
    let mut out = Vec::new();
    for (i, c) in syn.characters.iter().enumerate() {
        out.push(EntityDossier {
            entity_type: "character".to_string(),
            entity_id: format!("char-{:03}", i + 1),
            name: c.name.clone(),
            static_ref: format!("人物矩阵:{}", c.name),
            current: json!({
                "appearance": {},
                "state": {
                    "mood": c.current_mood,
                    "goal": c.wants,
                    "location": "",
                    "alive": true,
                    "knowledge": c.knows
                }
            }),
            sources: c.sources.clone(),
            ..Default::default()
        });
    }
    for (i, l) in syn.locations.iter().enumerate() {
        out.push(EntityDossier {
            entity_type: "location".to_string(),
            entity_id: format!("loc-{:03}", i + 1),
            name: l.name.clone(),
            static_ref: format!("世界观地点:{}", l.name),
            current: json!({
                "state": {
                    "description": l.description,
                    "level": l.level,
                    "region": l.region,
                    "faction": l.faction,
                    "unlocked_chapter": l.unlocked_chapter
                }
            }),
            sources: l.sources.clone(),
            ..Default::default()
        });
    }
    out
}

/// 状态快照：从档案与账本投影
fn build_current_state(
    syn: &DiscussionSynthesis,
    dossiers: &[EntityDossier],
    subplots: &[Subplot],
    written: i64,
) -> CurrentState {
    let mut cs = CurrentState {
        as_of_chapter: written,
        active_plots: subplots
            .iter()
            .filter(|s| s.status == "active")
            .map(|s| s.name.clone())
            .collect(),
        ..Default::default()
    };
    for d in dossiers.iter().filter(|d| d.entity_type == "character") {
        cs.characters.push(json!({
            "name": d.name,
            "mood": d.current["state"]["mood"],
            "goal": d.current["state"]["goal"],
            "alive": true
        }));
    }
    for d in dossiers.iter().filter(|d| d.entity_type == "location") {
        cs.world_state.push(json!({
            "name": d.name,
            "description": truncate(d.current["state"]["description"].as_str().unwrap_or(""), 60)
        }));
    }
    for c in &syn.characters {
        for r in &c.relationships {
            cs.relationships.push(json!({
                "from": r.from, "to": r.to, "relation_type": r.relation_type, "strength": r.strength
            }));
        }
    }
    cs.loose_ends = syn
        .disagreements
        .iter()
        .filter(|d| d.status != "resolved" && !d.adjudicated)
        .map(|d| d.topic.clone())
        .collect();
    cs.last_events = syn
        .timeline_events
        .iter()
        .rev()
        .take(3)
        .map(|t| t.description.clone())
        .collect();
    cs
}

/// 确定性检查引擎（一稿实现核心 H 规则 + 两条 S 规则）
fn run_checks(onto: &NovelOntology) -> BlueprintReport {
    let bp = &onto.blueprint;
    let written = onto
        .chapters
        .iter()
        .filter(|c| c.word_count > 0)
        .map(|c| c.chapter_no)
        .max()
        .unwrap_or(0);
    let mut issues = Vec::new();

    // CMT-H1：active 承诺必须能检查兑现（有兑现章或持续型）
    for c in &bp.commitments {
        if c.status == "active" && c.resolution_chapter.is_none() && !c.ongoing {
            issues.push(new_issue(
                "H",
                "commitments",
                "CMT-H1",
                &c.commitment_id,
                "承诺未设兑现章节且非持续型，无法检查兑现",
                vec![],
            ));
        }
    }

    // VOL-H1：卷章节范围连续无重叠
    let mut vols: Vec<&VolumeBlueprint> = bp.volumes.iter().collect();
    vols.sort_by_key(|v| v.volume_no);
    let mut prev_end = 0i64;
    for v in &vols {
        if v.chapter_start == 0 {
            continue;
        }
        if v.chapter_start <= prev_end {
            issues.push(new_issue(
                "H",
                "skeleton",
                "VOL-H1",
                &v.title,
                format!("卷章节范围与上一卷重叠或未连续（起点 {} ≤ 上卷终点 {prev_end}）", v.chapter_start),
                vec![],
            ));
        }
        prev_end = v.chapter_end;
    }

    // VOL-H2：每卷必须有高潮
    for v in &bp.volumes {
        if v.climax_chapter.is_none() || v.climax_scene.trim().is_empty() {
            issues.push(new_issue(
                "H",
                "skeleton",
                "VOL-H2",
                &v.title,
                "卷缺少高潮场景或高潮章节",
                vec![],
            ));
        }
    }

    // VOL-S2：卷首 10% 章节内没有钩子/高潮节奏点（beats 为空的老数据不误报）
    for v in &bp.volumes {
        if v.beats.is_empty() || v.chapter_start <= 0 || v.chapter_end <= v.chapter_start {
            continue;
        }
        let span = v.chapter_end - v.chapter_start + 1;
        let threshold = v.chapter_start + span / 10;
        let has_early_beat = v.beats.iter().any(|b| {
            matches!(b.beat_type.as_str(), "hook" | "climax")
                && b.chapter > 0
                && b.chapter <= threshold
        });
        if !has_early_beat {
            issues.push(new_issue(
                "S",
                "skeleton",
                "VOL-S2",
                &v.title,
                format!("卷首第 {threshold} 章之前没有钩子或高潮节奏点，开篇可能缺少抓点"),
                vec![],
            ));
        }
    }

    // VOL-S3：相邻爽点（payoff/climax）间隔超过卷长 40%，提示可能流失读者
    for v in &bp.volumes {
        if v.beats.is_empty() || v.chapter_end <= v.chapter_start {
            continue;
        }
        let mut payoff_chapters: Vec<i64> = v
            .beats
            .iter()
            .filter(|b| matches!(b.beat_type.as_str(), "payoff" | "climax"))
            .filter(|b| b.chapter > 0)
            .map(|b| b.chapter)
            .collect();
        payoff_chapters.sort_unstable();
        if payoff_chapters.len() < 2 {
            continue;
        }
        let span = v.chapter_end - v.chapter_start + 1;
        for pair in payoff_chapters.windows(2) {
            if pair[1] - pair[0] > span * 4 / 10 {
                issues.push(new_issue(
                    "S",
                    "skeleton",
                    "VOL-S3",
                    &v.title,
                    format!(
                        "爽点间隔过长：第 {} 章到第 {} 章（超过本卷 40%），读者可能流失",
                        pair[0], pair[1]
                    ),
                    vec![],
                ));
            }
        }
    }

    // FS-H1：planned 伏笔必须分配回收锚点（章/卷/事件）；完全无锚点才是硬违规，
    // 只有章级锚点无章号时降级为软性提示
    for f in &bp.foreshadows {
        if f.status != "planned" {
            continue;
        }
        let has_anchor = f.expected_payoff_chapter > 0
            || !f.payoff_anchor_type.is_empty()
            || !f.payoff_anchor.is_empty();
        if !has_anchor {
            issues.push(new_issue(
                "H",
                "foreshadows",
                "FS-H1",
                &f.foreshadow_id,
                format!("伏笔「{}」未分配任何回收锚点（章/卷/事件），永远收不了", f.name),
                vec![],
            ));
        } else if f.expected_payoff_chapter == 0 {
            issues.push(new_issue(
                "S",
                "foreshadows",
                "FS-H1",
                &f.foreshadow_id,
                format!(
                    "伏笔「{}」回收锚点为{}/「{}」，建议卷定盘时细化为具体章号",
                    f.name, f.payoff_anchor_type, f.payoff_anchor
                ),
                vec![],
            ));
        }
    }

    // FS-H3：已写章节超过预期回收章仍未解决
    for f in &bp.foreshadows {
        if f.status != "resolved" && f.expected_payoff_chapter > 0 && f.expected_payoff_chapter < written
        {
            issues.push(new_issue(
                "H",
                "foreshadows",
                "FS-H3",
                &f.foreshadow_id,
                format!(
                    "伏笔「{}」预期第 {} 章回收，已写到第 {written} 章仍未解决",
                    f.name, f.expected_payoff_chapter
                ),
                vec![],
            ));
        }
    }

    // CHR-H1：重要角色必须有核心欲望
    for m in &bp.character_matrix {
        if matches!(m.role.as_str(), "protagonist" | "mentor" | "antagonist" | "ally")
            && m.wants.trim().is_empty()
        {
            issues.push(new_issue(
                "H",
                "characters",
                "CHR-H1",
                &m.character_name,
                format!("重要角色「{}」缺核心欲望（wants）", m.character_name),
                vec![],
            ));
        }
    }

    // SP-H1：active 副线闲置超限
    for s in &bp.subplots {
        if s.status == "active"
            && s.last_touched_chapter > 0
            && s.touch_interval_limit > 0
            && written - s.last_touched_chapter > s.touch_interval_limit
        {
            issues.push(new_issue(
                "H",
                "subplots",
                "SP-H1",
                &s.subplot_id,
                format!(
                    "副线「{}」自第 {} 章后未再触碰（闲置 {} 章，上限 {}）",
                    s.name,
                    s.last_touched_chapter,
                    written - s.last_touched_chapter,
                    s.touch_interval_limit
                ),
                vec![],
            ));
        }
    }

    // DOS-H2：档案 current 与 change_log 一致性（简化：add/remove 的 value 是否出现在 current）
    for d in &bp.dossiers {
        let cur = serde_json::to_string(&d.current).unwrap_or_default();
        for ch in &d.change_log {
            let v = serde_json::to_string(&ch.value).unwrap_or_default();
            let present = !v.is_empty() && cur.contains(&v);
            let bad = (ch.action == "add" && !present) || (ch.action == "remove" && present);
            if bad {
                issues.push(new_issue(
                    "S",
                    "dossiers",
                    "DOS-H2",
                    &d.entity_id,
                    format!(
                        "档案「{}」字段 {} 的变更与当前状态不一致（action={}）",
                        d.name, ch.field, ch.action
                    ),
                    vec![],
                ));
            }
        }
    }

    let hard_count = issues.iter().filter(|i| i.severity == "H").count();
    BlueprintReport {
        checked_at: now(),
        written_chapters: written,
        hard_count,
        soft_count: issues.len() - hard_count,
        issues,
    }
}

fn new_issue(
    severity: &str,
    ledger: &str,
    rule_id: &str,
    target_id: &str,
    message: impl Into<String>,
    evidence: Vec<String>,
) -> CheckIssue {
    CheckIssue {
        severity: severity.to_string(),
        ledger: ledger.to_string(),
        rule_id: rule_id.to_string(),
        target_id: target_id.to_string(),
        message: message.into(),
        evidence,
    }
}

/// 解析「第1-3章 / 第1章 / 1-3」→ (起始, 结束)
fn parse_chapter_range(hint: &str) -> Option<(i64, i64)> {
    let s = hint.trim();
    // 兼容「第1-3章」与「31-80」两种写法
    let s = match s.strip_prefix("第") {
        Some(rest) => rest.trim(),
        None => s,
    };
    let s = s.split('章').next()?.trim();
    if s.is_empty() {
        return None;
    }
    for sep in ["-", "–", "—", "~", "至", "到"] {
        if let Some(pos) = s.find(sep) {
            let a: i64 = s[..pos].trim().parse().ok()?;
            let b: i64 = s[pos + sep.len()..].trim().parse().ok()?;
            return Some((a, b));
        }
    }
    let n: i64 = s.parse().ok()?;
    Some((n, n))
}

/// 提取文本中第一个数字（用于伏笔回收章等松散字段）
fn parse_chapter_number(text: &str) -> Option<i64> {
    let mut num = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else if !num.is_empty() {
            break;
        }
    }
    if num.is_empty() {
        None
    } else {
        num.parse().ok()
    }
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        t.chars().take(max).collect::<String>() + "…"
    }
}

pub(crate) fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chapter_range_variants() {
        assert_eq!(parse_chapter_range("第1-3章"), Some((1, 3)));
        assert_eq!(parse_chapter_range("第1章"), Some((1, 1)));
        assert_eq!(parse_chapter_range("第1至3章"), Some((1, 3)));
        assert_eq!(parse_chapter_range("31-80"), Some((31, 80)));
        assert_eq!(parse_chapter_range("无章节信息"), None);
    }

    #[test]
    fn test_parse_chapter_number() {
        assert_eq!(parse_chapter_number("第45章"), Some(45));
        assert_eq!(parse_chapter_number("45-50"), Some(45));
        assert_eq!(parse_chapter_number("卷末揭示"), None);
    }

    #[test]
    fn test_is_prohibition() {
        assert!(is_prohibition("力量体系不得超过九阶"));
        assert!(is_prohibition("主角不得天降救场"));
        assert!(!is_prohibition("主角性格冷静"));
    }

    #[test]
    fn test_build_volumes_groups_by_volume() {
        let beats = vec![
            pensoul_core::OutlineBeat {
                title: "开篇".into(),
                description: "废材登场".into(),
                chapter_hint: "第1-10章".into(),
                volume: "第一卷·风起".into(),
                beat_type: "铺垫".into(),
                ..Default::default()
            },
            pensoul_core::OutlineBeat {
                title: "大比夺魁".into(),
                description: "高潮".into(),
                chapter_hint: "第28-30章".into(),
                volume: "第一卷·风起".into(),
                beat_type: "高潮".into(),
                ..Default::default()
            },
            pensoul_core::OutlineBeat {
                title: "离宗".into(),
                description: "远行".into(),
                chapter_hint: "第31-60章".into(),
                volume: "第二卷·远行".into(),
                beat_type: "转折".into(),
                ..Default::default()
            },
        ];
        let vols = build_volumes(&beats);
        assert_eq!(vols.len(), 2);
        assert_eq!(vols[0].volume_no, 1);
        assert_eq!(vols[0].chapter_start, 1);
        assert_eq!(vols[0].chapter_end, 30);
        assert_eq!(vols[0].climax_scene, "大比夺魁");
        assert_eq!(vols[0].climax_chapter, Some(28));
        assert_eq!(vols[1].function, "resolution");
    }

    #[test]
    fn test_normalize_volume_handles_chinese_and_cross_volume() {
        assert_eq!(normalize_volume("第一卷"), Some("第1卷".to_string()));
        assert_eq!(normalize_volume("第二卷·远行"), Some("第2卷".to_string()));
        assert_eq!(
            normalize_volume("第一卷末至第二卷"),
            Some("第1卷".to_string())
        );
        assert_eq!(normalize_volume("各卷"), None);
        assert_eq!(normalize_volume(""), Some("第1卷".to_string()));
    }

    #[test]
    fn test_parse_chinese_number() {
        assert_eq!(parse_chinese_number("1"), Some(1));
        assert_eq!(parse_chinese_number("一"), Some(1));
        assert_eq!(parse_chinese_number("十"), Some(10));
        assert_eq!(parse_chinese_number("十二"), Some(12));
        assert_eq!(parse_chinese_number("二十"), Some(20));
        assert_eq!(parse_chinese_number("二十一"), Some(21));
    }

    #[test]
    fn test_refine_commitments_dedup_and_truncate() {
        let items = vec![
            Commitment {
                commitment_id: "a".into(),
                statement: "主角不得天降救场，一切胜利必须来自主角自己的选择与积累".into(),
                ..Default::default()
            },
            Commitment {
                commitment_id: "b".into(),
                statement: "主角不得天降救场（重复主题，应被去重）".into(),
                ..Default::default()
            },
        ];
        let out = refine_commitments(items);
        assert_eq!(out.len(), 1);
        assert!(out[0].statement.chars().count() <= 60);
    }

    #[test]
    fn test_fs_h1_anchor_downgrade() {
        let mut onto = NovelOntology::new(
            pensoul_core::ProjectId::new("t"),
            "测试".to_string(),
        );
        onto.blueprint.foreshadows = vec![
            // 有事件锚点：软性提示
            BlueprintForeshadow {
                foreshadow_id: "fs-a".into(),
                name: "有锚点".into(),
                status: "planned".into(),
                payoff_anchor_type: "event".into(),
                payoff_anchor: "身份揭破时".into(),
                ..Default::default()
            },
            // 完全无锚点：硬违规
            BlueprintForeshadow {
                foreshadow_id: "fs-b".into(),
                name: "无锚点".into(),
                status: "planned".into(),
                ..Default::default()
            },
        ];
        let report = run_checks(&onto);
        let fs_a = report
            .issues
            .iter()
            .find(|i| i.target_id == "fs-a")
            .unwrap();
        assert_eq!(fs_a.severity, "S");
        let fs_b = report
            .issues
            .iter()
            .find(|i| i.target_id == "fs-b")
            .unwrap();
        assert_eq!(fs_b.severity, "H");
    }

    #[test]
    fn test_run_checks_finds_core_violations() {
        let mut onto = NovelOntology::new(
            pensoul_core::ProjectId::new("t"),
            "测试".to_string(),
        );
        onto.blueprint.commitments = vec![Commitment {
            commitment_id: "cmt-001".into(),
            statement: "主角必须靠自己赢".into(),
            kind: "rule".into(),
            resolution_chapter: None,
            ongoing: false,
            status: "active".into(),
            ..Default::default()
        }];
        onto.blueprint.volumes = vec![VolumeBlueprint {
            volume_no: 1,
            title: "第一卷".into(),
            chapter_start: 1,
            chapter_end: 10,
            ..Default::default()
        }];
        onto.blueprint.foreshadows = vec![BlueprintForeshadow {
            foreshadow_id: "fs-001".into(),
            name: "玉坠".into(),
            status: "planned".into(),
            expected_payoff_chapter: 0,
            ..Default::default()
        }];
        let report = run_checks(&onto);
        assert!(report
            .issues
            .iter()
            .any(|i| i.rule_id == "CMT-H1"));
        assert!(report.issues.iter().any(|i| i.rule_id == "VOL-H2"));
        assert!(report.issues.iter().any(|i| i.rule_id == "FS-H1"));
    }

    #[test]
    fn test_vol_s2_and_s3_rhythm_checks() {
        let mut onto = NovelOntology::new(
            pensoul_core::ProjectId::new("t"),
            "测试".to_string(),
        );
        // 卷 1-30 章：只有高潮（28 章）与卷末钩子（30 章），卷首无 hook → VOL-S2
        onto.blueprint.volumes = vec![VolumeBlueprint {
            volume_no: 1,
            title: "第一卷".into(),
            chapter_start: 1,
            chapter_end: 30,
            beats: vec![
                VolumeBeat {
                    beat_id: "bt-1".into(),
                    beat_type: "climax".into(),
                    chapter: 28,
                    note: "夺魁".into(),
                    links: vec![],
                },
                VolumeBeat {
                    beat_id: "bt-2".into(),
                    beat_type: "hook_end".into(),
                    chapter: 30,
                    note: "玉坠异象".into(),
                    links: vec![],
                },
            ],
            ..Default::default()
        }];
        let report = run_checks(&onto);
        assert!(report.issues.iter().any(|i| i.rule_id == "VOL-S2"));

        // 补卷首 hook（第1章）+ 小爽点（第5章）：S2 消失；第5章→第28章间隔 23 >
        // 卷长 40%（12 章）→ VOL-S3 触发
        onto.blueprint.volumes[0].beats.push(VolumeBeat {
            beat_id: "bt-0".into(),
            beat_type: "hook".into(),
            chapter: 1,
            note: "开篇异常".into(),
            links: vec![],
        });
        onto.blueprint.volumes[0].beats.push(VolumeBeat {
            beat_id: "bt-3".into(),
            beat_type: "payoff".into(),
            chapter: 5,
            note: "小爽点".into(),
            links: vec![],
        });
        let report = run_checks(&onto);
        assert!(!report.issues.iter().any(|i| i.rule_id == "VOL-S2"));
        assert!(report.issues.iter().any(|i| i.rule_id == "VOL-S3"));

        // 节奏合理时（爽点 5→16 章，间隔 11 ≤ 12）两条规则都不触发
        onto.blueprint.volumes[0].beats[3].chapter = 16;
        let report = run_checks(&onto);
        assert!(!report.issues.iter().any(|i| i.rule_id == "VOL-S2"));
        assert!(!report.issues.iter().any(|i| i.rule_id == "VOL-S3"));
    }
}
