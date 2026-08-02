//! 专家蒸馏 IPC 命令 —— 加载 pensoul-skill-Experts 技能，按其方法论调用 LLM
//! 将人物的思维方式提炼为可参与创作讨论的专家技能卡。
//!
//! 产物约定（与 skills/pensoul-skill-Experts 一致）：
//! - 目录：`Experts/<名字>-expert/`
//! - 模板：角色规则 / 创作讨论工作流 / 核心心智模型 / 创作决策启发式 /
//!   表达 DNA / 价值观与反模式 / 诚实边界（无身份卡、生平年表、智识谱系）
//! - 调研过程写入 `references/research/` 随产物自包含保存
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

use super::experts::{experts_base_dir, extract_section_any, parse_skill_md};

/// 蒸馏阶段事件 —— 实时推送给前端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillPhaseEvent {
    pub phase: String,
    pub status: String,
    pub message: String,
    pub detail: String,
}

/// 蒸馏事件缓冲上限（阶段事件有限，超出后丢弃最旧）
const DISTILL_EVENT_CAP: usize = 100;

/// 蒸馏控制面（书籍/方法论/专家蒸馏共用）：运行旗标 + 任务类型 + 事件缓冲，
/// 支持前端切换页面后重连恢复进度（与讨论、造化工坊同一模式）。
pub struct DistillControl {
    /// 是否有蒸馏任务正在进行（防重入）
    pub running: AtomicBool,
    /// 当前任务类型（book / methodology / expert），前端据此恢复对应面板
    pub kind: parking_lot::Mutex<Option<String>>,
    /// 阶段事件缓冲：前端重连后重放恢复进度
    pub events: parking_lot::RwLock<Vec<DistillPhaseEvent>>,
}

impl DistillControl {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            kind: parking_lot::Mutex::new(None),
            events: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// 事件入缓冲（发射前调用）
    pub(crate) fn record(&self, ev: &DistillPhaseEvent) {
        let mut buf = self.events.write();
        if buf.len() >= DISTILL_EVENT_CAP {
            buf.remove(0);
        }
        buf.push(ev.clone());
    }
}

impl Default for DistillControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 蒸馏运行守卫：独占蒸馏控制面，任务结束（成功或失败）时自动释放旗标
/// 并发射终态事件（`phase = "__distill__"`），前端据此收尾刷新。
pub struct DistillRunGuard<'a> {
    app_handle: &'a tauri::AppHandle,
    state: &'a AppState,
    event_name: &'static str,
    success: bool,
    message: String,
}

impl<'a> DistillRunGuard<'a> {
    /// 尝试独占蒸馏控制面；已有任务进行时返回错误
    pub fn begin(
        app_handle: &'a tauri::AppHandle,
        state: &'a AppState,
        kind: &str,
        event_name: &'static str,
    ) -> Result<Self, String> {
        if state.distills.running.swap(true, Ordering::SeqCst) {
            return Err("已有蒸馏任务正在进行，请等待当前任务完成".to_string());
        }
        *state.distills.kind.lock() = Some(kind.to_string());
        state.distills.events.write().clear();
        Ok(Self {
            app_handle,
            state,
            event_name,
            success: false,
            message: String::new(),
        })
    }

    /// 标记任务成功并附完成消息（终态事件在 Drop 时发射）
    pub fn finish(&mut self, message: impl Into<String>) {
        self.success = true;
        self.message = message.into();
    }
}

impl Drop for DistillRunGuard<'_> {
    fn drop(&mut self) {
        let (status, message) = if self.success {
            (
                "finished",
                if self.message.is_empty() {
                    "蒸馏完成".to_string()
                } else {
                    self.message.clone()
                },
            )
        } else {
            (
                "error",
                if self.message.is_empty() {
                    "蒸馏失败或已中断".to_string()
                } else {
                    self.message.clone()
                },
            )
        };
        let ev = DistillPhaseEvent {
            phase: "__distill__".to_string(),
            status: status.to_string(),
            message,
            detail: String::new(),
        };
        self.state.distills.record(&ev);
        let _ = self.app_handle.emit(self.event_name, ev);
        self.state.distills.running.store(false, Ordering::SeqCst);
    }
}

/// 蒸馏状态查询：运行旗标 + 任务类型 + 事件缓冲（前端切换页面后重连恢复进度）
#[tauri::command]
pub async fn get_distill_state(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "running": state.distills.running.load(Ordering::SeqCst),
        "kind": state.distills.kind.lock().clone(),
        "events": state.distills.events.read().clone(),
    }))
}

/// 技能文件在工作区内的相对路径
const SKILL_RELATIVE_PATH: &str = "skills/pensoul-skill-Experts/SKILL.md";

/// 编译进二进制的完整蒸馏技能内容（发布版必然可用，不依赖运行时路径）。
/// 技能源文件随仓库分发，构建时用 include_str! 嵌入。
const EMBEDDED_SKILL_MD: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../skills/pensoul-skill-Experts/SKILL.md"
));

#[tauri::command]
pub async fn distill_expert(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    persona: String,
    model: Option<String>,
) -> Result<pensoul_core::Expert, String> {
    use super::llm_helper as lh;
    lh::ensure_api_keys_loaded(&state);
    // 独占蒸馏控制面：任务全程占位，结束（成功或失败）时自动发终态事件
    let mut guard = DistillRunGuard::begin(&app_handle, &state, "expert", "distill-phase")?;

    let saved_providers = lh::load_providers(&state);
    let saved_models = lh::load_models(&state);
    let api_keys = { state.api_keys.read().clone() };

    // 模型解析：指定优先（按其归属供应商取 Key）；缺省 = 全局默认模型，其次任意可用模型
    let (provider_id, api_key, api_base, model_id) = match model.filter(|m| !m.trim().is_empty()) {
        Some(m) => {
            let m2p = lh::build_model_to_provider(&saved_models);
            let bases = lh::build_provider_api_bases(&saved_providers);
            let (pid, key, base) = lh::resolve_provider(&m, &m2p, &bases, &api_keys)?;
            (pid, key, base, m)
        }
        None => {
            let mid = lh::pick_default_model(&saved_models, &api_keys)
                .ok_or_else(|| "未配置任何可用模型，请在模型设置中配置 API Key".to_string())?;
            let m2p = lh::build_model_to_provider(&saved_models);
            let bases = lh::build_provider_api_bases(&saved_providers);
            let (pid, key, base) = lh::resolve_provider(&mid, &m2p, &bases, &api_keys)?;
            (pid, key, base, mid)
        }
    };
    let model_id = model_id.as_str();

    // 加载 pensoul-skill-Experts 技能作为蒸馏方法论
    let (methodology, skill_source) = load_distill_methodology(&state);
    emit_phase(
        &app_handle,
        &state,
        "加载蒸馏技能",
        "done",
        &skill_source,
        "",
    )
    .ok();

    let auth = lh::ProviderAuth {
        provider_id: &provider_id,
        api_key: &api_key,
        api_base: &api_base,
    };

    // ── Phase 1: 人物调研（技能的六维度框架）──
    emit_phase(
        &app_handle,
        &state,
        "人物调研",
        "running",
        &format!("正在按六维度框架调研「{}」的思维方式...", persona),
        "",
    )
    .ok();

    let research_prompt = format!(
        "你是「PenSoul · 专家思维蒸馏术」的调研执行者。上述方法论是你的工作手册。\n\
         现在执行其中的 Phase 1（多源信息采集），对象为「{persona}」。\n\n\
         注意：你无法联网检索，请基于你的知识储备调研，并对不确定的信息明确标注置信度。\n\
         严格按以下六个维度输出，每个维度 3-8 条要点，每条注明（一手/二手/推测）：\n\
         1. 著作与系统思考：反复出现≥3次的核心论点、自创术语、推崇的书/作者\n\
         2. 对话与即兴思考：被追问时的回答方式、即兴类比、改变立场的瞬间\n\
         3. 表达风格 DNA：高频用词句式、幽默方式、确定性表达习惯\n\
         4. 他者视角：外部观察到的模式、批评与争议、与同行对比\n\
         5. 创作决策：重大创作决策的背景与逻辑、言行一致/不一致案例\n\
         6. 思维演变：创作观的思想转折点（不是生平年表）\n\n\
         硬性要求：发现矛盾时保留矛盾，不要调和；信息不足的维度直接标注「信息不足」。\n\
         直接以「## 维度 1」开头输出报告：不要标题、不要前言、不要执行方式说明、\n\
         不要置信度图例，置信度标注融入每条要点末尾即可。\n\
         用中文输出。",
    );
    // 调研输出量大（六维度各 3-8 条），推理型模型还要先烧 reasoning 预算
    let research =
        lh::call_llm(&auth, model_id, &methodology, &research_prompt, 0.7, 16384).await?;
    // 兜底清理：剥掉模型可能附加的执行说明/图例等前言，报告从第一个维度标题开始
    let research = match research.find("## 维度") {
        Some(idx) => research[idx..].to_string(),
        None => research,
    };
    emit_phase(
        &app_handle,
        &state,
        "人物调研",
        "done",
        "调研完成",
        &research,
    )
    .ok();

    // ── Phase 2: 技能构建（按产物模板生成 SKILL.md）──
    emit_phase(
        &app_handle,
        &state,
        "技能构建",
        "running",
        &format!("正在为「{}」提炼心智模型并构建专家技能...", persona),
        "",
    )
    .ok();

    // 目标目录名（中文保留）：frontmatter 的 name 必须与之一致
    let safe_name: String = persona
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            _ => c,
        })
        .collect();
    let dir_name = format!("{}-expert", safe_name);

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let build_prompt = format!(
        "你是「PenSoul · 专家思维蒸馏术」的提炼与构建者。上述方法论是你的工作手册。\n\
         基于以下对「{persona}」的调研，执行 Phase 2（框架提炼）与 Phase 3（专家构建），\n\
         产出最终的 SKILL.md 文件内容。\n\n\
         ═══════ 调研素材 ═══════\n{research}\n═══════ 调研素材结束 ═══════\n\n\
         构建要求：\n\
         1. 从素材中提炼 3-7 个心智模型，每个必须通过三重验证（跨域复现/生成力/排他性），\n\
            包含：名称、一句话描述、证据（≥2个场景）、应用（什么创作问题适用）、局限\n\
         2. 提炼 5-10 条创作决策启发式，每条含应用场景和案例\n\
         3. 表达 DNA：句式/词汇/节奏/幽默/确定性/引用习惯\n\
         4. 价值观与反模式：我追求的（创作价值观排序）/ 我拒绝的（创作反模式）/ 我自己也没想清楚的（内在张力，≥2对）\n\
         5. 创作讨论工作流：Step1 问题分类 → Step2 [人物]式审视（3-5 个审视维度，必须从心智模型反推，\n\
            每个维度有具体看点）→ Step3 输出讨论意见\n\
         6. 诚实边界：≥3 条具体局限，并注明「调研时间：{today}」\n\n\
         产物红线（违反即失败）：\n\
         - 禁止出现人物简介类内容：身份卡、生平介绍、生平时间线、最新动态、智识谱系\n\
         - 禁止编造此人没说过的话；禁止把通用道理包装成此人的独特见解\n\
         - 禁止堆砌金句，心智模型必须是可运行的框架\n\n\
         输出格式（严格遵守）：\n\
         - 只输出 SKILL.md 的 markdown 内容，前后不要任何解释\n\
         - 用 ===SKILL_MD_BEGIN=== 和 ===SKILL_MD_END=== 包裹全部内容\n\
         - 文件以 ---\\nname: {dir_name}\\ndescription: <一句话中文描述>\\n--- 开头\n\
           （name 字段必须原样写 {dir_name}，禁止翻译成英文或拼音）\n\
         - 正文 section 依次为：# {persona} · 创作思维系统 / ## 角色规则（第一人称参与创作讨论，\n\
           不复述生平）/ ## 创作讨论工作流 / ## 核心心智模型（### 模型N: 名称）/ ## 创作决策启发式 /\n\
           ## 表达 DNA / ## 价值观与反模式 / ## 诚实边界",
    );
    // K3 默认 reasoning_effort=max，思考先烧掉一半以上预算，
    // 直接给足 32768，避免 SKILL.md 写到一半被截断丢标记
    let raw_output = lh::call_llm(&auth, model_id, &methodology, &build_prompt, 0.7, 32768).await?;

    // 提取标记之间的 SKILL.md 内容，并强制校正 frontmatter 的 name 为目录名（防 LLM 译成英文）
    let skill_md_raw = extract_skill_md(&raw_output).ok_or_else(|| {
        let total = raw_output.chars().count();
        let tail: String = raw_output.chars().skip(total.saturating_sub(150)).collect();
        format!("LLM 输出中未找到 SKILL.md 内容标记（输出 {total} 字符，结尾：…{tail}），请重试")
    })?;
    let skill_md = normalize_frontmatter_name(&skill_md_raw, &dir_name);

    // 解析产物，提取专家卡片字段
    let (frontmatter, body) = parse_skill_md(&skill_md);
    let fm_desc = frontmatter.get("description").cloned().unwrap_or_default();
    let models_section = extract_section_any(&body, &["核心心智模型"]);
    let decision = extract_section_any(&body, &["创作决策启发式", "决策启发式"]);
    let expression = extract_section_any(&body, &["表达 DNA", "表达DNA"]);
    let boundaries = extract_section_any(&body, &["诚实边界"]);

    if models_section.is_empty() {
        return Err("生成的技能缺少「核心心智模型」章节，请重试".to_string());
    }

    let expert_name = persona.clone();

    // 保存到 Experts 文件夹：<名字>-expert/
    let experts_base = experts_base_dir(&state);
    let skill_dir = experts_base.join(&dir_name);
    let research_dir = skill_dir.join("references").join("research");
    std::fs::create_dir_all(&research_dir).map_err(|e| format!("创建技能目录失败: {e}"))?;

    // 调研过程自包含保存（方法论要求：不存文件的调研等于没做）
    let research_md = format!(
        "# 「{}」LLM 调研记录\n\n> 调研时间：{}\n> 说明：应用内蒸馏为单次 LLM 调研，\
         未经多源交叉验证，置信度以文中标注为准。\n\n{}",
        persona, today, research
    );
    std::fs::write(research_dir.join("01-llm-research.md"), research_md)
        .map_err(|e| format!("写入调研记录失败: {e}"))?;

    let skill_file = skill_dir.join("SKILL.md");
    std::fs::write(&skill_file, &skill_md).map_err(|e| format!("写入 SKILL.md 失败: {e}"))?;

    let mut prompt_parts = Vec::new();
    prompt_parts.push(format!("## 核心心智模型\n{}", models_section.trim()));
    if !decision.is_empty() {
        prompt_parts.push(format!("## 创作决策启发式\n{}", decision.trim()));
    }
    if !expression.is_empty() {
        prompt_parts.push(format!("## 表达 DNA\n{}", expression.trim()));
    }
    if !boundaries.is_empty() {
        prompt_parts.push(format!("## 诚实边界\n{}", boundaries.trim()));
    }
    let default_prompt = prompt_parts.join("\n\n");

    let desc_combined = format!("【PenSoul专家】{} - {}", persona, fm_desc.trim());

    let expert = pensoul_core::Expert {
        id: format!("distilled-{}", uuid::Uuid::new_v4()),
        name: expert_name.clone(),
        description: desc_combined,
        source_persona: persona.clone(),
        model_id: model_id.to_string(),
        // 卡片维度只显示一句话；完整心智模型在 SKILL.md 与 default_prompt 中
        perspective: format!("以{}的视角对设定及核心想法进行讨论", persona),
        default_prompt,
        created_at: today,
        skill_path: Some(skill_file.to_string_lossy().to_string()),
        skill_summary: Some(format!("PenSoul专家 · {}", persona)),
    };

    emit_phase(
        &app_handle,
        &state,
        "技能构建",
        "done",
        "技能构建完成！",
        &format!("已生成「{}」并保存到 Experts/{}", expert_name, dir_name),
    )
    .ok();
    guard.finish(format!("已生成「{}」专家技能", expert_name));
    Ok(expert)
}

/// 加载 pensoul-skill-Experts 技能内容作为蒸馏方法论。
/// 返回 (方法论文本, 来源描述)。找不到技能文件时回退到内置简版。
fn load_distill_methodology(state: &AppState) -> (String, String) {
    for candidate in skill_file_candidates(state) {
        if candidate.exists()
            && let Ok(content) = std::fs::read_to_string(&candidate)
        {
            return (content, format!("已加载蒸馏技能: {}", candidate.display()));
        }
    }
    (
        EMBEDDED_SKILL_MD.to_string(),
        "已加载内置蒸馏技能（编译进应用）".to_string(),
    )
}

/// 技能文件候选路径：沿可执行文件向上找、Experts 目录的同级、当前工作目录
fn skill_file_candidates(state: &AppState) -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();

    // 沿可执行文件路径逐级向上（覆盖 target/debug 与 .app bundle 两种形态）
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            candidates.push(ancestor.join(SKILL_RELATIVE_PATH));
        }
    }

    // Experts 目录的同级 skills/（Experts 与工作区根同级）
    if let Some(root) = experts_base_dir(state).parent().map(|p| p.to_path_buf()) {
        candidates.push(root.join(SKILL_RELATIVE_PATH));
    }

    // 当前工作目录
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(SKILL_RELATIVE_PATH));
    }

    // 用户技能目录（~/.codex/skills、~/.agents/skills）下的 PenSoul-skill-Experts：
    // 目录名可能是 nuwa-skill 等任意名字，按 SKILL.md frontmatter 的 name 匹配。
    // 固定路径与全量扫描都加入，保证换目录名后仍能找到。
    if let Some(home) = dirs::home_dir() {
        for base in [home.join(".codex").join("skills"), home.join(".agents").join("skills")] {
            candidates.push(base.join("nuwa-skill").join("SKILL.md"));
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let skill_md = entry.path().join("SKILL.md");
                    if !skill_md.is_file() {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&skill_md)
                        && content.lines().any(|l| l.trim() == "name: PenSoul-skill-Experts")
                    {
                        candidates.push(skill_md);
                    }
                }
            }
        }
    }

    candidates
}

/// 强制将 SKILL.md frontmatter 中的 name 校正为实际目录名。
/// LLM 有时会把中文名翻译成英文/拼音（如 luxun），此处以用户输入为准；
/// 缺少 frontmatter 时补一个最小的。
/// （pub(crate)：书籍蒸馏 book_distill.rs 复用同一校正逻辑）
pub(crate) fn normalize_frontmatter_name(content: &str, dir_name: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return format!("---\nname: {}\n---\n\n{}", dir_name, content);
    }
    // 在前两个 --- 之间查找 name 行并替换；没有则在 frontmatter 末尾插入
    let mut name_written = false;
    let mut fm_end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            fm_end = Some(i);
            break;
        }
    }
    let scan_end = fm_end.unwrap_or(lines.len());
    for line in lines[1..scan_end].iter_mut() {
        if line.starts_with("name:") {
            *line = format!("name: {}", dir_name);
            name_written = true;
        }
    }
    if !name_written {
        lines.insert(fm_end.unwrap_or(1), format!("name: {}", dir_name));
    }
    lines.join("\n")
}

/// 从 LLM 输出中提取 ===SKILL_MD_BEGIN=== 与 ===SKILL_MD_END=== 之间的内容，
/// 并剥离可能残留的 markdown 代码围栏。
/// （pub(crate)：书籍蒸馏 book_distill.rs 复用同一提取逻辑）
pub(crate) fn extract_skill_md(output: &str) -> Option<String> {
    let begin = output.find("===SKILL_MD_BEGIN===")? + "===SKILL_MD_BEGIN===".len();
    let end = output.rfind("===SKILL_MD_END===")?;
    if end <= begin {
        return None;
    }
    let mut text = output[begin..end].trim().to_string();

    // 剥离 ```markdown / ``` 围栏
    if let Some(stripped) = text.strip_prefix("```") {
        let without_open = stripped.strip_prefix("markdown").unwrap_or(stripped);
        text = without_open
            .trim_end()
            .strip_suffix("```")
            .unwrap_or(without_open.trim_end())
            .trim()
            .to_string();
    }

    if text.is_empty() { None } else { Some(text) }
}

/// 向 Tauri 前端发射蒸馏阶段事件
fn emit_phase(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    phase: &str,
    status: &str,
    message: &str,
    detail: &str,
) -> Result<(), String> {
    let event = DistillPhaseEvent {
        phase: phase.to_string(),
        status: status.to_string(),
        message: message.to_string(),
        detail: detail.to_string(),
    };
    state.distills.record(&event);
    let _ = app_handle.emit("distill-phase", event);
    Ok(())
}
