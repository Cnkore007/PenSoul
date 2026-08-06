//! 开书定盘蓝图：六张账本 + 实体动态档案 + 状态快照
//!
//! 蓝图是讨论收敛后的正典（静态蓝图 + 运行态骨架），正文与细纲从它派生；
//! 运行态更新（实体档案自动结算）在后续阶段接入。

/// 承诺账本条目 —— 这本书对读者的承诺与铁律
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Commitment {
    pub commitment_id: String,
    pub statement: String,
    /// 承诺类型：theme=主题 / promise=卖点 / tone=基调 / rule=铁律 / no_go=禁区
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub priority: u32,
    /// 生效范围：book / volume-N / chapter-A-B
    #[serde(default)]
    pub scope: String,
    /// 兑现章节（非持续型承诺必填，否则无法检查）
    #[serde(default)]
    pub resolution_chapter: Option<i64>,
    /// true 表示持续型承诺（全书生效，不设单一兑现点）
    #[serde(default)]
    pub ongoing: bool,
    /// active / fulfilled / waived / broken
    #[serde(default = "default_active")]
    pub status: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

fn default_active() -> String {
    "active".to_string()
}

/// 结构骨架：卷蓝图 —— 正式「第一卷 / 第二卷」的规划
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VolumeBlueprint {
    pub volume_no: u32,
    pub title: String,
    #[serde(default)]
    pub one_line: String,
    /// setup / escalation / climax / resolution
    #[serde(default)]
    pub function: String,
    /// 本卷对读者的承诺（读完之后获得什么）
    #[serde(default)]
    pub reader_promise: String,
    #[serde(default)]
    pub chapter_start: i64,
    #[serde(default)]
    pub chapter_end: i64,
    #[serde(default)]
    pub central_conflict: String,
    #[serde(default)]
    pub climax_scene: String,
    #[serde(default)]
    pub climax_chapter: Option<i64>,
    /// 卷间钩子：本卷结尾留给下一卷的悬念
    #[serde(default)]
    pub volume_hook: String,
    /// 节奏/情绪曲线描述（如「起-伏-升-爆」）
    #[serde(default)]
    pub pacing: String,
    /// 节奏点规划（爽点位置提前定好，供时间线展示与 VOL-S2/S3 检查）
    #[serde(default)]
    pub beats: Vec<VolumeBeat>,
    /// 本卷推进的角色弧光（「角色名→阶段名」）
    #[serde(default)]
    pub arcs_pushed: Vec<String>,
    #[serde(default)]
    pub subplots_started: Vec<String>,
    #[serde(default)]
    pub subplots_resolved: Vec<String>,
    #[serde(default)]
    pub foreshadows_planted: Vec<String>,
    #[serde(default)]
    pub foreshadows_paid_off: Vec<String>,
    /// planned / outlined / drafting / closed
    #[serde(default = "default_planned")]
    pub status: String,
}

fn default_planned() -> String {
    "planned".to_string()
}

/// 卷内节奏点：钩子/蓄力/爽点/回落/高潮/卷末钩子的位置规划
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VolumeBeat {
    pub beat_id: String,
    /// hook / buildup / payoff / fall / climax / hook_end
    #[serde(default)]
    pub beat_type: String,
    #[serde(default)]
    pub chapter: i64,
    #[serde(default)]
    pub note: String,
    /// 关联承诺/伏笔/副线 id（cmt-001 / fs-001 / sp-001）
    #[serde(default)]
    pub links: Vec<String>,
}

/// 人物矩阵：弧光阶段
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MatrixArcStage {
    pub name: String,
    #[serde(default)]
    pub chapter_range: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub turning_point: String,
}

/// 人物矩阵条目 —— 不变内核 + 弧光 + 知情边界 + 出场纪律
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CharacterMatrixEntry {
    pub character_name: String,
    /// protagonist / mentor / antagonist / ally / love_interest / minor
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub core_values: Vec<String>,
    /// 绝不越过的行为底线
    #[serde(default)]
    pub taboo: Vec<String>,
    #[serde(default)]
    pub speech_style: String,
    #[serde(default)]
    pub wants: String,
    #[serde(default)]
    pub fears: String,
    /// 读者暂不知晓的秘密
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub arc: Vec<MatrixArcStage>,
    /// 当前知道什么 / 不知道什么（知情边界）
    #[serde(default)]
    pub knows: Vec<String>,
    #[serde(default)]
    pub does_not_know: Vec<String>,
    /// 缺席超过该章数即提示（0 = 不检查）
    #[serde(default)]
    pub max_absent_chapters: i64,
    #[serde(default)]
    pub last_appeared: i64,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 伏笔账本条目 —— 埋设-回收承诺（章号用 i64，保证规则可枚举）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BlueprintForeshadow {
    pub foreshadow_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// object / line / secret / ability / event / relationship
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub planted_chapter: i64,
    /// 预期回收章；0 = 未分配（检查 FS-H1）
    #[serde(default)]
    pub expected_payoff_chapter: i64,
    /// 回收锚点类型：chapter / volume / event（无章号时用卷/事件锚点）
    #[serde(default)]
    pub payoff_anchor_type: String,
    /// 回收锚点文本（如「第2卷」「身份揭破时」）
    #[serde(default)]
    pub payoff_anchor: String,
    #[serde(default)]
    pub actual_payoff_chapter: i64,
    /// planned / planted / progressing / resolved / abandoned / overdue
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub related_characters: Vec<String>,
    #[serde(default)]
    pub related_items: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 副线账本条目 —— 生命周期与主线关系
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Subplot {
    pub subplot_id: String,
    pub name: String,
    #[serde(default)]
    pub line_tags: Vec<String>,
    #[serde(default)]
    pub mainline_relation: String,
    /// planned / active / paused / resolved / abandoned
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub start_chapter: i64,
    #[serde(default)]
    pub end_chapter: Option<i64>,
    #[serde(default)]
    pub characters: Vec<String>,
    /// 最近一次被正文触碰的章节
    #[serde(default)]
    pub last_touched_chapter: i64,
    #[serde(default)]
    pub touch_interval_limit: i64,
    #[serde(default)]
    pub open_threads: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 资源账本条目 —— 金手指/道具/信息/势力的状态机
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ResourceEntry {
    pub resource_id: String,
    pub name: String,
    /// item / ability / info / relationship / faction / asset
    #[serde(default)]
    pub rtype: String,
    #[serde(default)]
    pub owner: String,
    /// available / consumed / lost / destroyed / transferred / revealed
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub acquired_chapter: i64,
    #[serde(default)]
    pub consumed_chapter: i64,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 实体档案：单条变更留痕
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DossierChange {
    pub chapter: i64,
    pub field: String,
    /// add / remove / update / promote / drop / resolve
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub value: serde_json::Value,
    #[serde(default)]
    pub before: serde_json::Value,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub source: String,
}

/// 实体档案：出场摘要（轻量，供回溯与矛盾检查）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DossierAppearance {
    pub chapter: i64,
    #[serde(default)]
    pub visual: String,
    #[serde(default)]
    pub state_summary: String,
}

/// 实体档案：悬置变更（等证据自动转正，不是确认区）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PendingChange {
    pub pending_id: String,
    pub field: String,
    #[serde(default)]
    pub value: serde_json::Value,
    pub chapter: i64,
    /// waiting / promoted / dropped
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub evidence: String,
}

/// 实体档案：未决冲突（自动解决器无法判定时打标，正文自然消解）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DossierConflict {
    pub conflict_id: String,
    pub field: String,
    pub chapter_a: i64,
    pub chapter_b: i64,
    #[serde(default)]
    pub note: String,
    /// open / resolved
    #[serde(default)]
    pub status: String,
}

/// 实体动态档案 —— 每实体一张卡，随剧情增删改并留痕
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntityDossier {
    /// character / location / faction
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    /// 指向静态蓝图（人物矩阵 / 世界观条目）
    #[serde(default)]
    pub static_ref: String,
    /// 当前状态组：appearance / abilities / state，结构按实体类型自由
    #[serde(default)]
    pub current: serde_json::Value,
    #[serde(default)]
    pub change_log: Vec<DossierChange>,
    #[serde(default)]
    pub appearances: Vec<DossierAppearance>,
    #[serde(default)]
    pub pending: Vec<PendingChange>,
    #[serde(default)]
    pub conflicts: Vec<DossierConflict>,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// 当前状态快照 —— 实体档案的投影，供下一章上下文组装
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CurrentState {
    #[serde(default)]
    pub as_of_chapter: i64,
    #[serde(default)]
    pub characters: Vec<serde_json::Value>,
    #[serde(default)]
    pub world_state: Vec<serde_json::Value>,
    #[serde(default)]
    pub active_plots: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<serde_json::Value>,
    #[serde(default)]
    pub loose_ends: Vec<String>,
    #[serde(default)]
    pub last_events: Vec<String>,
}

/// 开书定盘蓝图 —— 讨论收敛后的正典
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BookBlueprint {
    /// 是否已定盘
    #[serde(default)]
    pub settled: bool,
    #[serde(default)]
    pub settled_at: String,
    /// 定盘来源（讨论总结摘要前 60 字）
    #[serde(default)]
    pub settled_from: String,
    /// 来源指纹：讨论成果的轻量摘要（角色数|情节数|规则数|地点数|总结字数），
    /// 前端据此提示「讨论成果已更新，需重新定盘」
    #[serde(default)]
    pub source_stamp: String,
    /// 承诺账本
    #[serde(default)]
    pub commitments: Vec<Commitment>,
    /// 结构骨架（卷蓝图）
    #[serde(default)]
    pub volumes: Vec<VolumeBlueprint>,
    /// 人物矩阵
    #[serde(default)]
    pub character_matrix: Vec<CharacterMatrixEntry>,
    /// 伏笔账本
    #[serde(default)]
    pub foreshadows: Vec<BlueprintForeshadow>,
    /// 副线账本
    #[serde(default)]
    pub subplots: Vec<Subplot>,
    /// 资源账本
    #[serde(default)]
    pub resources: Vec<ResourceEntry>,
    /// 实体动态档案（骨架，运行态结算后增补）
    #[serde(default)]
    pub dossiers: Vec<EntityDossier>,
    /// 当前状态快照
    #[serde(default)]
    pub current_state: CurrentState,
}

/// 确定性检查结果
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CheckIssue {
    /// H=硬性（门控阻塞）/ S=软性（提示）
    pub severity: String,
    /// commitments / skeleton / characters / foreshadows / subplots / resources / dossiers / state
    pub ledger: String,
    pub rule_id: String,
    pub target_id: String,
    pub message: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// 检查报告（含汇总）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BlueprintReport {
    pub checked_at: String,
    /// 已写章节数（正文类规则用）
    pub written_chapters: i64,
    pub issues: Vec<CheckIssue>,
    pub hard_count: usize,
    pub soft_count: usize,
}
