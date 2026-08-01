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
    /// 内置模板（不可删除、不可改 builtin 标志）
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
            version: "1.0".to_string(),
            genre: "网文".to_string(),
            description: "面向网络小说的自动连写流：黄金三章、核心卖点、压抑-释放情绪曲线、断章钩子等网文方法论内置于阶段手册；审查按「卖点一致 + 情绪曲线 + 断章钩子」判定。".to_string(),
            builtin: true,
            enabled: true,
            review_pass_score: 80.0,
            stages: vec![
                WorkflowStageDef {
                    stage: "chapter_writing".to_string(),
                    display_name: "章节写作".to_string(),
                    prompt_hint: "根据本章梗概与创作备忘录撰写正文：开篇 300 字内建立冲突或悬念（刀架脖子式危机），金手指针对本章危机量身定制；语言干练、钩子优先，结尾断在刀尖落下前。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "卖点与一致性审查".to_string(),
                    prompt_hint: "用不同模型审查本章：① 是否兑现核心卖点（馒头不跑偏成钻石）；② 压抑→释放的情绪曲线是否成立；③ 是否给读者新期待并留断章钩子；④ 与设定/人物/前文的一致性。输出 score 与 issues。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                },
            ],
            bindings: serde_json::json!({}),
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
                    prompt_hint: "根据本章梗概与前文承接撰写正文：人物动机先行，冲突从人物目标自然生长；文风稳定克制，段落不超过五行，展示而非讲述。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 2,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "一致性审查".to_string(),
                    prompt_hint: "用不同模型审查本章：人物性格/状态/位置是否连贯、伏笔是否埋而未收、世界观设定是否矛盾、文风是否偏离基调。输出 score 与 issues。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
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
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "设定一致性审查".to_string(),
                    prompt_hint: "用不同模型审查本章：核心设定是否自洽、技术细节是否违背已建立的规则、时间线与因果链是否成立、人物在科幻压力下行为是否合理。输出 score 与 issues。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 2,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
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
                },
                WorkflowStageDef {
                    stage: "chapter_review".to_string(),
                    display_name: "快速检查".to_string(),
                    prompt_hint: "用不同模型快速检查关键一致性问题与明显硬伤，输出 score 与 issues。".to_string(),
                    gate: "conditional".to_string(),
                    on_fail: Some("chapter_writing".to_string()),
                    max_retries: 1,
                    enabled: true,
                },
                WorkflowStageDef {
                    stage: "state_injection".to_string(),
                    display_name: "状态回灌".to_string(),
                    prompt_hint: "提炼本章纪要，回灌滚动备忘录，供下一章写作携带。".to_string(),
                    gate: "auto".to_string(),
                    on_fail: None,
                    max_retries: 1,
                    enabled: true,
                },
            ],
            bindings: serde_json::json!({}),
        },
    ]
}
