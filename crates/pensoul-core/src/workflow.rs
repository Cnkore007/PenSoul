//! 工作流模板与项目引用。
//!
//! 分层模型：
//! - `WorkflowTemplate`：全局工作流模板（作品库层面定义，如网文流/标准流/科幻流），
//!   描述阶段编排、门控阈值、阶段手册与模板级环节绑定；
//! - `WorkflowRef`：项目对模板的引用 + 项目级覆盖（各环节模型/技法卡），
//!   项目内只存引用与差异，不复制整套模板。
//!
//! 造化工坊启动时按「项目引用 → 全局模板 → 覆盖」解析出实际执行的阶段与绑定。

/// 模板中的一个执行环节（对应管线固定三阶段，名称与后端阶段 key 一致）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStageDef {
    /// 阶段 key：chapter_writing / chapter_review / state_injection
    pub stage: String,
    /// 面向用户的阶段显示名
    pub display_name: String,
    /// 阶段工作手册（注入引擎 manual，指导该阶段怎么写/怎么判）
    pub prompt_hint: String,
    /// 门控类型：auto / manual / conditional
    pub gate: String,
    /// 门控失败时的回退阶段（如 chapter_writing）
    pub on_fail: Option<String>,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用（一稿仅作展示字段，管线仍按固定三阶段执行）
    pub enabled: bool,
    /// 前 3 章启用「黄金三章」硬门控（审查时钩子/爽点维度必须达标，否则拦截重写）
    #[serde(default)]
    pub golden_gate: bool,
}

impl Default for WorkflowStageDef {
    fn default() -> Self {
        Self {
            stage: "chapter_writing".to_string(),
            display_name: "章节写作".to_string(),
            prompt_hint: String::new(),
            gate: "auto".to_string(),
            on_fail: None,
            max_retries: 2,
            enabled: true,
            golden_gate: false,
        }
    }
}

/// 全局工作流模板（作品库层面定义，可被多个项目引用）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowTemplate {
    /// 模板唯一 ID（不可变）
    pub template_id: String,
    /// 模板显示名
    pub name: String,
    /// 模板版本号（项目记录引用版本，模板更新不污染进行中作品）
    pub version: String,
    /// 体裁标签：网文 / 传统 / 科幻 / 通用…
    pub genre: String,
    /// 一句话说明
    pub description: String,
    /// 内置模板（核心内置 webnovel 不可删除；其余内置可删除，恢复内置时补回；不可改 builtin 标志）
    pub builtin: bool,
    /// 是否启用（停用的模板不进入项目选择列表）
    pub enabled: bool,
    /// 审查放行阈值（0-100，默认 80）
    pub review_pass_score: f64,
    /// 执行环节定义（管线固定三阶段，此处可调门控/手册/重试）
    pub stages: Vec<WorkflowStageDef>,
    /// 模板级环节绑定：`{ outline_expand: {model, cards}, chapter_writing: {...}, review: {...} }`
    /// cards 为 WritingCard 技法卡 SKILL.md 路径，透明存储
    pub bindings: serde_json::Value,
}

impl WorkflowTemplate {
    /// 按阶段 key 查环节定义
    pub fn find_stage(&self, stage: &str) -> Option<&WorkflowStageDef> {
        self.stages.iter().find(|s| s.stage == stage)
    }

    /// 取模板级某环节绑定（缺省返回空对象）
    pub fn stage_bindings(&self, stage: &str) -> serde_json::Value {
        self.bindings
            .get(stage)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))
    }
}

/// 项目对工作流模板的引用 + 项目级覆盖。
///
/// `overrides` 结构：`{ outline_expand: {model, cards}, chapter_writing: {...}, review: {...} }`，
/// 与模板绑定的覆盖规则：项目覆盖字段优先，模板绑定兜底。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowRef {
    /// 引用的模板 ID（None = 未选择模板）
    pub template_id: Option<String>,
    /// 引用时的模板版本
    pub template_version: Option<String>,
    /// 项目级覆盖（透明存储，结构由前端定义）
    pub overrides: serde_json::Value,
}

impl Default for WorkflowRef {
    fn default() -> Self {
        Self {
            template_id: None,
            template_version: None,
            overrides: serde_json::json!({}),
        }
    }
}

/// 内置模板列表（首次启动写入全局模板库；可恢复）。
pub fn builtin_workflow_templates() -> Vec<WorkflowTemplate> {
    vec![
        WorkflowTemplate {
            template_id: "webnovel".to_string(),
            name: "网文创作流".to_string(),
            version: "2.0".to_string(),
            genre: "网文".to_string(),
            description: "面向网络小说的自动连写流 v2：写前先出「章前策划」节拍表（场景级目标/冲突/钩子），写作阶段按节拍表执行并遵守黄金三章、断章钩子、反 AI 味铁律；审查按七维加权打分（卖点/钩子/情绪/节奏/一致性/文笔）。".to_string(),
            builtin: true,
            enabled: true,
            review_pass_score: 80.0,
            stages: vec![
                WorkflowStageDef {
                    stage: "chapter_planning".to_string(),
                    display_name: "章前策划".to_string(),
                    prompt_hint: "写前策划：结合本章梗概、前章纪要、世界观与人物状态，产出一张可执行的节拍表（JSON）：本章目标一句话、开场钩子、3-6 个场景（每个含目标/冲突/状态变化/建议字数）、爽点与情绪释放点、本章必须新增的未解决次要问题、结尾断章钩子（疑问型/危机型/转折型三选一）、埋设与回收的伏笔、人物状态变化。铁律：非终局章节禁止解决主线核心冲突；刚用过的套路（冲突爽点/羁绊/势力经营/风土人情/危机升级）至少在冷却期内不得作为主场景重复。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "chapter_writing".to_string(),
                    display_name: "章节写作".to_string(),
                    prompt_hint: "严格按「章前策划」节拍表撰写正文，但场景之间要用上一场留下的细节、动作或感知自然钩连，禁止逐条机械展开；开场 300 字内出现冲突或悬念；每个场景必须有目标、阻碍与状态变化；叙述、对话与动作按叙事需要交织，保留心理与氛围描写作缓冲；结尾必须断在钩子处。语言铁律（反 AI 味）：删除「不禁/仿佛/映入眼帘/心中暗道/嘴角微扬」等套话，每千字弱化副词不超过 3 个，不用排比三连，不用「与此同时/从而/一方面…另一方面」等套话级连接词但允许自然过渡（然而/随后/片刻后）；用具体动作和感知代替情绪直说；节奏有起伏，允许长句铺陈，禁止把场景拆成孤立短碎片。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "卖点与质量审查".to_string(),
                    prompt_hint: "用不同模型按七维加权审查本章：① 卖点兑现（20 分）；② 开场钩子（10 分）；③ 情绪曲线与爽点（20 分）；④ 场景与节奏（10 分）；⑤ 断章钩子（15 分）；⑥ 人物与设定一致性（15 分）；⑦ 文笔与反 AI 味（10 分）。对照节拍表检查是否按策划执行，偏差列入问题清单。输出分数与问题清单。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                    golden_gate: true,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录：关键事件、人物状态变化、埋设/推进/回收的伏笔，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                    golden_gate: false,
                },
            ],
            // 模板级绑定：直接引用「网文创作方法论」技能卡（路径相对 WritingCard/ 根目录）
            bindings: serde_json::json!({
                "outline_expand": {
                    "cards": [
                        "网文创作方法论-methodology/structure/SKILL.md",
                        "网文创作方法论-methodology/tension/SKILL.md",
                        "网文创作方法论-methodology/character/SKILL.md",
                        "网文创作方法论-methodology/genre/SKILL.md",
                    ]
                },
                "chapter_writing": {
                    "cards": [
                        "网文创作方法论-methodology/style/SKILL.md",
                        "网文创作方法论-methodology/tension/SKILL.md",
                        "网文创作方法论-methodology/character/SKILL.md",
                    ]
                },
                "review": {
                    "cards": [
                        "网文创作方法论-methodology/review/SKILL.md",
                        "网文创作方法论-methodology/style/SKILL.md",
                    ]
                }
            }),
        },
        WorkflowTemplate {
            template_id: "standard-novel".to_string(),
            name: "标准小说流".to_string(),
            version: "1.0".to_string(),
            genre: "传统".to_string(),
            description: "面向传统长篇的稳定创作流：强调人物弧光、文风质感与伏笔回收，审查更严（85 分放行）。".to_string(),
            builtin: true,
            enabled: true,
            review_pass_score: 85.0,
            stages: vec![
                WorkflowStageDef {
                    stage: "chapter_writing".to_string(),
                    display_name: "章节写作".to_string(),
                    prompt_hint: "根据本章梗概与前文承接撰写正文：人物动机先行，冲突从人物目标自然生长；文风稳定克制，段落一般不超过五行但允许成段心理与氛围描写；场景之间用细节自然钩连，展示而非讲述。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "一致性审查".to_string(),
                    prompt_hint: "用不同模型审查本章：人物性格/状态/位置是否连贯、伏笔是否埋而未收、世界观设定是否矛盾、文风是否偏离基调。输出分数与问题清单。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                    golden_gate: false,
                },
            ],
            bindings: serde_json::json!({}),
        },
        WorkflowTemplate {
            template_id: "scifi".to_string(),
            name: "科幻创作流".to_string(),
            version: "1.0".to_string(),
            genre: "科幻".to_string(),
            description: "面向科幻长篇：核心设定的链式反应优先，硬设定一致性审查严格（85 分放行）。".to_string(),
            builtin: true,
            enabled: true,
            review_pass_score: 85.0,
            stages: vec![
                WorkflowStageDef {
                    stage: "chapter_writing".to_string(),
                    display_name: "章节写作".to_string(),
                    prompt_hint: "根据本章梗概与前文承接撰写正文：核心设定是世界的发动机而非背景板，每一处技术/规则描写都要与世界观自洽并推演连带反应；悬念先行，解释随文潜入。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "设定一致性审查".to_string(),
                    prompt_hint: "用不同模型审查本章：核心设定是否自洽、技术细节是否违背已建立的规则、时间线与因果链是否成立、人物在科幻压力下行为是否合理。输出分数与问题清单。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                    golden_gate: false,
                },
            ],
            bindings: serde_json::json!({}),
        },
        WorkflowTemplate {
            template_id: "quick-novel".to_string(),
            name: "快速创作流".to_string(),
            version: "1.0".to_string(),
            genre: "通用".to_string(),
            description: "精简快速产出：审查宽松（70 分放行），适合试稿与灵感验证。".to_string(),
            builtin: true,
            enabled: true,
            review_pass_score: 70.0,
            stages: vec![
                WorkflowStageDef {
                    stage: "chapter_writing".to_string(),
                    display_name: "快速写作".to_string(),
                    prompt_hint: "根据本章梗概快速撰写正文：节奏明快，冲突直接，不追求辞藻，保证读完有情绪反应。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "快速检查".to_string(),
                    prompt_hint: "用不同模型快速检查关键一致性问题与明显硬伤，输出分数与问题清单。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 1,
                    enabled: true,
                    golden_gate: false,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                    golden_gate: false,
                },
            ],
            bindings: serde_json::json!({}),
        },
    ]
}
