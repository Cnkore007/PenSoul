export interface Chapter {
  chapter_id: string;
  chapter_no?: number; // 章节序号（后端 backfill 后必有）
  volume_id?: string;
  title: string;
  summary?: string; // 章节梗概（大纲层信息，非正文）
  content: string;
  word_count: number;
  version: number;
  status: 'Draft' | 'Reviewing' | 'Reviewed' | 'Polished' | 'Published';
  // 笔耕批注（行内 + 整章）
  annotations?: ChapterAnnotation[];
  // 版本历史（批注重写前快照 / 回滚点）
  revisions?: ChapterRevision[];
}

// 批注锚点：段落索引 + 段内偏移 + 锚定原文片段（行内）；field 为字段级锚点（细纲/描述等）
export interface AnnotationAnchor {
  paragraph_index: number;
  offset: number;
  text: string;
  field?: string | null;
}

// 全链路批注（正文行内 / 表单字段 / 实体级）
export interface ChapterAnnotation {
  annotation_id: string;
  kind: "issue" | "suggestion" | "note"; // 问题 / 修改建议 / 备注
  anchor?: AnnotationAnchor | null;
  content: string;
  status: "open" | "accepted" | "rejected"; // 待处理 / 已采纳 / 已拒绝
  created_at?: string;
  processed_in_version?: number;
  // 定位串：如 chapter:ch-1:body / location:loc-1:description
  target?: string | null;
  // 判决来源：manual=用户直接处理 / rewrite_plan=重写计划
  resolved_by?: string | null;
  // 创建时的锚定文本快照
  anchor_snapshot?: string | null;
  resolved_at?: string | null;
}

// 章节版本历史
export interface ChapterRevision {
  version: number;
  content: string;
  word_count?: number;
  created_at?: string;
  reason?: string;
}

// 项目写作经验条目
export interface WritingLesson {
  lesson_id: string;
  category: string;
  problem: string;
  fix?: string;
  example?: string;
  count?: number;
  created_at?: string;
}

export interface Volume {
  volume_id: string;
  title: string;
  chapter_count: number;
}

export interface Project {
  project_id: string;
  title: string;
  volumes: Volume[];
  total_chapters: number;
  total_words: number;
}

export interface HarnessStatus {
  current_stage: string;
  stage_status: string;
  memo: Record<string, unknown>;
  retry_count: number;
}

export interface ConsistencyViolation {
  violation_id: string;
  entity_id: string;
  entity_type: string;
  chapter_a: number;
  chapter_b: number;
  description: string;
  severity: 'Error' | 'Warning' | 'Info';
}

export interface CharacterData {
  id: string;
  name: string;
  personality_traits: Array<[string, number]>;
  current_mood?: string;
  relationships: Array<{ from: string; to: string; relation_type: string; strength: number }>;
  wants?: string;
  fears?: string;
  secret?: string;
  speech_style?: string;
  arc_stages?: Array<{ name: string; chapter_range?: string; trait_desc?: string; goal?: string }>;
  knows?: string[];
  does_not_know?: string[];
  sources?: string[];
}

export interface WorldData {
  locations: Array<{
    id: string;
    name: string;
    description: string;
    level?: string;
    region?: string;
    faction?: string;
    unlocked_chapter?: string;
    spatial_tags?: string[];
    sources?: string[];
  }>;
  timeline_events: Array<{
    event_id: string;
    story_time: string;
    description: string;
    participants?: string[];
    sources?: string[];
  }>;
  setting_rules: Array<{
    rule_id: string;
    title: string;
    description: string;
    category?: string;
    constraints?: string[];
    cost?: string;
    loophole?: string;
    sources?: string[];
  }>;
}

export interface StyleMetrics {
  avg_sentence_length: number;
  vocabulary_richness: number;
  dialogue_ratio: number;
  pace_score: number;
  ai_pattern_score: number;
  sentence_var: number;
  avg_paragraph_length: number;
  paragraph_uniformity: number;
  connector_per_1k: number;
  dash_per_1k: number;
  colon_per_1k: number;
  quote_style: string;
  sampled_chapters: number;
}

// 章节修改后的影响分析（analyze_chapter_impact 命令返回）
export interface ChapterImpact {
  chapter_no: number;
  affected: any[]; // CDA AffectedItem：node_id / chapter_id / severity / action
  consistency: any[]; // ConsistencyViolation：rule_name / description / severity
}

// 反 AI 味检测报告（analyze_ai_flavor 命令返回）
export interface AiFlavorCategory {
  key: string;
  label: string;
  tier: number; // 1 = 单词命中即扣 / 2 = 同段聚集 / 3 = 全文密度
  hits: number;
  score: number;
  max_score: number;
  examples: string[];
}

// 节奏信号（确定性指标，信息性展示，不参与总分）
export interface RhythmSignal {
  avg_sentence_length: number;
  sentence_var: number;
  paragraph_uniformity: number;
  flagged: boolean;
  note: string;
}

export interface AiFlavorReport {
  score: number; // 0-100，越高 AI 味越重
  level: string; // 低 / 中 / 高
  total_hits: number;
  categories: AiFlavorCategory[];
  rhythm: RhythmSignal;
  suggestion: string;
}

// 项目元数据
export interface ProjectMeta {
  project_id: string;
  title: string;
  description: string;
  created_at: string;
  updated_at: string;
  total_chapters: number;
  total_words: number;
}

// 去 AI 味重写结果（rewrite_chapter_deai 命令返回）
export interface DeaiRewriteResult {
  new_version: number;
  word_count: number;
  original_word_count: number;
  suggested_deletions: Array<{ sentence: string; reason: string }>;
  fidelity_issues: string[];
  residual_issues: string[];
  repaired: boolean;
  summary: string;
}

// LLM 提供商
export interface LlmProvider {
  provider_id: string;
  name: string;
  display_name: string;
  api_base: string;
  requires_api_key: boolean;
}

// 思考模式能力
export type ThinkingMode = "none" | "always" | "toggleable";

// 模型能力档案（models.json 的 capability 字段，对应官方文档参数）
export interface ModelCapability {
  context_window: number;
  max_output_tokens: number;
  budget_field: string;
  thinking_mode: ThinkingMode;
  reasoning_effort_options: string[];
  // 深度任务默认思考强度（用户可调整）
  default_reasoning_effort: string;
  // 深度任务默认是否开启思考（仅 toggleable 生效；用户可调整）
  thinking_enabled: boolean;
  // 思考开关字段名：thinking（对象）或 enable_thinking（布尔）
  thinking_field: string;
  // 思考强度字段名：reasoning_effort（顶层）或 reasoning（嵌套对象）
  effort_field: string;
  // 采样参数固定（Kimi 系列显式传 temperature/top_p 会被拒绝）
  fixed_sampling: boolean;
  docs_url: string;
  notes: string;
  updated_at: string;
}

// LLM 模型配置
export interface LlmModel {
  model_id: string;
  provider_id: string;
  display_name: string;
  max_tokens: number;
  supports_tools: boolean;
  cost_per_1k_tokens: number;
  avg_quality_score: number;
  avg_latency_ms: number;
  is_available: boolean;
  api_key_configured: boolean;
  // 全局默认模型（模型设置页可设，各环节未手动选择时优先用它）
  is_default?: boolean;
  // 用户是否手动开关过该模型（true 时 is_available 以磁盘为准，不被 api_key 自动点亮）
  user_managed?: boolean;
  // 能力档案（官方文档参数；未收录时由后端自动补齐内置档案）
  capability?: ModelCapability;
}

// 情节脉络节点 —— 大纲规划层（覆盖一个章节范围的剧情规划，展开细纲后才生成可写章节）
export interface OutlineArc {
  arc_id: string;
  title: string;
  description: string;
  chapter_start: number; // 覆盖起始章号（含，从 1 开始）
  chapter_end: number;   // 覆盖结束章号（含）
  volume_id?: string;    // 所属卷 ID（讨论成果按卷导入时挂载）
  expanded_until: number; // 已展开细纲到第几章（0 = 未展开）
}

// ── 开书定盘 · 真相账本体系 ──

// 承诺账本条目
export interface Commitment {
  commitment_id: string;
  statement: string;
  kind: string; // theme / promise / tone / rule / no_go
  priority: number;
  scope: string;
  resolution_chapter?: number | null;
  ongoing: boolean;
  status: string; // active / fulfilled / waived / broken
  sources: string[];
}

// 结构骨架：卷蓝图
export interface VolumeBeat {
  beat_id: string;
  // hook / buildup / payoff / fall / climax / hook_end
  beat_type: string;
  chapter: number;
  note: string;
  links: string[];
}

export interface VolumeBlueprint {
  volume_no: number;
  title: string;
  one_line: string;
  function: string; // setup / escalation / climax / resolution
  reader_promise: string;
  chapter_start: number;
  chapter_end: number;
  central_conflict: string;
  climax_scene: string;
  climax_chapter?: number | null;
  volume_hook: string;
  pacing: string;
  beats?: VolumeBeat[];
  arcs_pushed: string[];
  subplots_started: string[];
  subplots_resolved: string[];
  foreshadows_planted: string[];
  foreshadows_paid_off: string[];
  status: string; // planned / outlined / drafting / closed
}

// 人物矩阵：弧光阶段
export interface MatrixArcStage {
  name: string;
  chapter_range: string;
  goal: string;
  turning_point: string;
}

export interface CharacterMatrixEntry {
  character_name: string;
  role: string;
  core_values: string[];
  taboo: string[];
  speech_style: string;
  wants: string;
  fears: string;
  secret: string;
  arc: MatrixArcStage[];
  knows: string[];
  does_not_know: string[];
  max_absent_chapters: number;
  last_appeared: number;
  sources: string[];
}

export interface BlueprintForeshadow {
  foreshadow_id: string;
  name: string;
  description: string;
  kind: string;
  planted_chapter: number;
  expected_payoff_chapter: number;
  actual_payoff_chapter: number;
  status: string;
  related_characters: string[];
  related_items: string[];
  sources: string[];
}

export interface Subplot {
  subplot_id: string;
  name: string;
  line_tags: string[];
  mainline_relation: string;
  status: string;
  start_chapter: number;
  end_chapter?: number | null;
  characters: string[];
  last_touched_chapter: number;
  touch_interval_limit: number;
  open_threads: string[];
  sources: string[];
}

export interface ResourceEntry {
  resource_id: string;
  name: string;
  rtype: string;
  owner: string;
  status: string;
  acquired_chapter: number;
  consumed_chapter: number;
  constraints: string[];
  note: string;
  sources: string[];
}

export interface DossierChange {
  chapter: number;
  field: string;
  action: string; // add / remove / update / promote / drop / resolve
  value?: unknown;
  before?: unknown;
  reason: string;
  source: string;
}

export interface DossierAppearance {
  chapter: number;
  visual: string;
  state_summary: string;
}

export interface PendingChange {
  pending_id: string;
  field: string;
  value?: unknown;
  chapter: number;
  status: string;
  evidence: string;
}

export interface DossierConflict {
  conflict_id: string;
  field: string;
  chapter_a: number;
  chapter_b: number;
  note: string;
  status: string;
}

export interface EntityDossier {
  entity_type: string; // character / location / faction
  entity_id: string;
  name: string;
  static_ref: string;
  current?: unknown;
  change_log: DossierChange[];
  appearances: DossierAppearance[];
  pending: PendingChange[];
  conflicts: DossierConflict[];
  sources: string[];
}

export interface CurrentState {
  as_of_chapter: number;
  characters: unknown[];
  world_state: unknown[];
  active_plots: string[];
  relationships: unknown[];
  loose_ends: string[];
  last_events: string[];
}

export interface BookBlueprint {
  settled: boolean;
  settled_at: string;
  settled_from: string;
  // 来源指纹：讨论成果摘要，前端据此提示「讨论成果已更新」
  source_stamp?: string;
  commitments: Commitment[];
  volumes: VolumeBlueprint[];
  character_matrix: CharacterMatrixEntry[];
  foreshadows: BlueprintForeshadow[];
  subplots: Subplot[];
  resources: ResourceEntry[];
  dossiers: EntityDossier[];
  current_state: CurrentState;
}

export interface CheckIssue {
  severity: string; // H / S
  ledger: string;
  rule_id: string;
  target_id: string;
  message: string;
  evidence: string[];
}

export interface BlueprintReport {
  checked_at: string;
  written_chapters: number;
  issues: CheckIssue[];
  hard_count: number;
  soft_count: number;
}

// ── 书籍蒸馏 · 写作技能卡 ──

// 单张技能卡（WritingCard/<书名>-book/<维度>/SKILL.md）
export interface BookCardInfo {
  dimension: string; // style / structure / character / tension / genre
  dimension_label: string;
  name: string;
  description: string;
  skill_path: string;
  applicable_stages: string[]; // outline_expand / chapter_writing / review
}

// 技能包（一次蒸馏的产物）
export interface BookPackage {
  package: string; // 目录名 <书名>-book
  title: string;
  author: string;
  created_at: string;
  cards: BookCardInfo[];
}

// 爆款拆解模块库条目（从蒸馏卡投影，灵感库不是正典）
export interface StoryModule {
  module_id: string;
  source_book: string;
  module_type: string; // hook / opening / transition / ending / payoff / pacing / structure
  name: string;
  technique: string;
  example: string;
  when_to_use: string;
  boundary: string;
  bound_stage: string[];
  favorite: boolean;
}

// 工作流单个环节的技能绑定：模型 + 技法卡路径列表（每维度最多一张，前端约束）
export interface StageSkillConfig {
  model: string | null;
  cards: string[];
}

// 工作流技能配置：三个可绑卡的执行环节
export interface WorkflowSkillConfig {
  outline_expand: StageSkillConfig; // 细纲展开
  chapter_writing: StageSkillConfig; // 章节写作
  review: StageSkillConfig; // 一致性审查
}

// 模板中的一个执行环节（stage 与后端管线三阶段 key 一致）
export interface WorkflowStageDef {
  stage: string; // chapter_planning / chapter_writing / chapter_review / state_injection
  display_name: string;
  prompt_hint: string; // 阶段工作手册
  gate: 'auto' | 'manual' | 'conditional';
  on_fail: string | null; // 门控失败时的回退阶段
  max_retries: number;
  enabled: boolean;
  golden_gate?: boolean; // 审查环节：前 3 章启用黄金三章硬门控（钩子/爽点必达标）
}

// 全局工作流模板（作品库层面资产，可被多个项目引用）
export interface WorkflowTemplate {
  template_id: string;
  name: string;
  version: string;
  genre: string; // 网文 / 传统 / 科幻 / 通用…
  description: string;
  builtin: boolean; // 内置模板：不可删除
  enabled: boolean; // 停用后不进入项目选择列表
  review_pass_score: number; // 审查放行阈值（0-100）
  stages: WorkflowStageDef[];
  // 模板级环节绑定：{ outline_expand: {model, cards}, chapter_writing: {...}, review: {...} }
  bindings: Record<string, StageSkillConfig> | Record<string, unknown>;
}

// 项目对工作流模板的引用 + 项目级覆盖
// 项目内只保存「引用了哪个模板 + 差异覆盖」，模板本体留在作品库
export interface WorkflowRef {
  template_id: string | null;
  template_version: string | null;
  // 项目级覆盖：{ outline_expand: {model, cards}, chapter_writing: {...}, review: {...} }
  overrides: Record<string, StageSkillConfig>;
}

// 项目工作空间数据 — 每个项目独立存储
export interface ProjectData {
  project_id: string;
  volumes: VolumeWithChapters[];
  characters: CharacterData[];
  world: WorldData;
  // 项目工作流引用：模板 ID + 版本 + 项目覆盖（undefined = 从未配置）
  workflowRef?: WorkflowRef | null;
  // 派生字段：按「项目覆盖 → 模板绑定」合并后的各环节有效配置。
  // 由 workflowRef + 全局模板计算而来，不单独持久化（保存时只写 workflowRef）
  workflowSkills?: WorkflowSkillConfig;
  style: StyleMetrics | null;
  // 核心概念 / 高概念种子
  concept: CoreConceptData;
  // 萌芽数据 — 想法描述 + Agent 讨论配置
  sprout: SproutData;
  settings: ProjectSettings;
  // 情节脉络（大纲规划层）；只读视图，增删改走专用 IPC，不随 saveProjectData 全量保存
  outlineArcs: OutlineArc[];
  // 开书定盘蓝图（讨论收敛后的正典）；只读展示，定盘/检查走专用 IPC
  blueprint: BookBlueprint;
}

export interface VolumeWithChapters extends Volume {
  chapters: Chapter[];
  expanded: boolean;
}

// 灵感建议
export interface InspirationItem {
  title: string;
  content: string;
}

// 核心概念 / 高概念 — 整部小说的"种子"
export interface CoreConceptData {
  // 核心想法 / 高概念（一句话概括）
  highConcept: string;
  // 故事前提 / 冲突前提
  premise: string;
  // 主角雏形
  protagonistHint: string;
  // 故事基调 / 风格
  tone: string;
  // 核心冲突
  centralConflict: string;
  // 灵感来源 / 创作缘由
  inspiration: string;
}

// 创作规划设置
export interface ProjectSettings {
  // 目标总章节数（0 表示未设定）
  targetChapters: number;
  // 目标总字数（自动计算：目标总章数 × 每章字数，无需手动填写）
  targetWords: number;
  // 每章目标字数（0 表示未设定）
  chapterTargetWords: number;
  // 故事类型
  genre: string;
  // 预计卷数
  targetVolumes: number;
}

// 默认的高概念示例
// 讨论 Agent 配置
export interface AgentDiscussionConfig {
  id: string;
  name: string;
  model: string;
  prompt: string;
  perspective: string;
  enabled: boolean;
  // 关联的专家 ID（可选，从专家库选择时填充）
  expertId?: string;
  // 关联专家的技能文件路径（可选，讨论时加载作为系统提示词）
  skillPath?: string;
}

// 专家 — 蒸馏自著名人物的认知框架
export interface Expert {
  id: string;
  name: string;
  description: string;
  // 来源人物/主题
  sourcePersona: string;
  // 配置的模型 ID
  modelId: string;
  // 评审维度
  perspective: string;
  // 默认评审提示词
  defaultPrompt: string;
  // 创建时间
  createdAt: string;
  // 女娲技能文件路径（可选）
  skillPath?: string;
  // 技能摘要
  skillSummary?: string;
}

// 萌芽数据 — 核心想法 + 创作设定 + 讨论 Agent
export interface SproutData {
  // 用户对故事想法的自由描述
  ideaDescription: string;
  // 讨论 Agent 配置列表
  agents: AgentDiscussionConfig[];
  // 预置 Agent 是否已被用户移除（true 时即使 agents 为空也不再回退到预置）
  presetsDismissed?: boolean;
  // 最近一次讨论的结果（发言 + 成果），切换页面后保留
  lastDiscussion?: {
    turns: DiscussionTurn[];
    synthesis: DiscussionSynthesis;
    // 作者在确认生成前填写的确认意见（同意讨论结果 / 补充意见）
    authorFeedback?: string;
  };
}

// 讨论发言记录（一轮一条）
export interface DiscussionTurn {
  agent_id: string;
  agent_name: string;
  perspective: string;
  round: number; // 1=立论 2=交锋
  content: string;
}

// 结构化讨论成果
export interface DiscussionSynthesis {
  summary: string;
  locations: Array<{
    name: string;
    description: string;
    level?: string;
    region?: string;
    faction?: string;
    unlocked_chapter?: string;
    sources?: string[];
  }>;
  timeline_events: Array<{ story_time: string; description: string; participants?: string[]; sources?: string[] }>;
  setting_rules: Array<{
    name: string;
    description: string;
    constraints?: string[];
    cost?: string;
    loophole?: string;
    sources?: string[];
  }>;
  characters: Array<{
    name: string;
    personality_traits: Array<[string, number]>;
    current_mood?: string;
    description?: string;
    relationships?: Array<{ from: string; to: string; relation_type: string; strength: number }>;
    wants?: string;
    fears?: string;
    secret?: string;
    speech_style?: string;
    entity_kind?: string; // individual / group / faction（群像不进人物矩阵）
    arc?: Array<{ name: string; chapter_range?: string; trait_desc?: string; goal?: string }>;
    knows?: string[];
    does_not_know?: string[];
    sources?: string[];
  }>;
  outline_beats: Array<{
    title: string;
    description: string;
    chapter_hint?: string;
    volume?: string;
    beat_type?: string;
    hook?: string;
    payoff?: string;
    emotion_arc?: string;
    line_tags?: string[];
    foreshadowing?: Array<{
      plant: string;
      payoff_hint?: string;
      payoff_anchor_type?: string; // chapter / volume / event
      payoff_anchor?: string;
    }>;
    sources?: string[];
  }>;
  // 副线条目（情节维度提炼）
  subplots?: Array<{
    name: string;
    description?: string;
    mainline_relation?: string;
    chapter_range?: string;
    open_threads?: string[];
    characters?: string[];
    sources?: string[];
  }>;
  // 承诺与卖点条目（承诺维度提炼）
  commitments?: Array<{
    statement: string;
    kind?: string; // theme / promise / tone / rule / no_go
    scope?: string;
    ongoing?: boolean;
    sources?: string[];
  }>;
  // 讨论中显式保留的分歧与裁决（含跨维度冲突）
  disagreements?: Array<{
    topic: string;
    dimension?: string;
    sides?: Array<{ agent: string; position: string; rationale?: string }>;
    status?: string; // resolved=讨论内已收敛 / open=未收敛
    resolution?: string;
    adjudicated?: boolean;
    alternatives?: string[];
  }>;
  // 共识复核与质量提示
  quality_notes?: string[];
}

// 讨论完整输出
export interface DiscussionOutput {
  turns: DiscussionTurn[];
  synthesis: DiscussionSynthesis;
}

// 讨论进度事件（后端实时推送）
export interface DiscussionEvent {
  agent_id: string;
  agent_name: string;
  round: number; // 1=立论 2=交锋 3=成果
  status: string; // running / done / error
  content: string;
}

// 预置讨论 Agent
export const DEFAULT_DISCUSSION_AGENTS: AgentDiscussionConfig[] = [
  {
    id: 'agent-market',
    name: '市场分析师',
    model: 'gpt-4o',
    perspective: '商业与市场',
    prompt: '从商业潜力和市场受众角度分析这个构思的可行性和市场定位。考虑目标读者群、题材热度、商业变现路径。',
    enabled: true,
  },
  {
    id: 'agent-logic',
    name: '逻辑审查员',
    model: 'gpt-4o',
    perspective: '设定与逻辑',
    prompt: '审查故事设定的内部一致性、情节逻辑的合理性、世界观规则的严谨性。指出潜在的逻辑漏洞和设定冲突。',
    enabled: true,
  },
  {
    id: 'agent-character',
    name: '角色顾问',
    model: 'gpt-4o',
    perspective: '角色与情感',
    prompt: '评估角色弧光的完整度、人物关系的张力、情感驱动的合理性。建议如何让角色更立体、更有代入感。',
    enabled: true,
  },
  {
    id: 'agent-style',
    name: '文风品鉴师',
    model: 'gpt-4o',
    perspective: '文风与表达',
    prompt: '分析构思所适合的文风基调、叙事视角选择、节奏把控建议。考虑如何用语言风格增强故事感染力。',
    enabled: true,
  },
  {
    id: 'agent-creative',
    name: '创意激发师',
    model: 'gpt-4o',
    perspective: '创意延伸',
    prompt: '基于现有构思进行创意延伸，提出意想不到的情节发展方向、设定亮点和叙事技巧，帮助构思更具原创性。',
    enabled: true,
  },
];

export function createDefaultConcept(): CoreConceptData {
  return {
    highConcept: '',
    premise: '',
    protagonistHint: '',
    tone: '',
    centralConflict: '',
    inspiration: '',
  };
}

export type ViewType =
  | 'experts'
  | 'concept'
  | 'blueprint'
  | 'writing'
  | 'outline'
  | 'character'
  | 'world'
  | 'annotations'
  | 'consistency'
  | 'harness'
  | 'style'
  | 'projects'
  | 'llm-settings'
  | 'workflow-library'
  | 'dashboard';

// ── 连写管线（造化工坊） ──

// 管线实时事件（后端 harness-event 推送）
export interface PipelineEvent {
  seq?: number; // 后端单调序号（快照与实时事件去重用）
  chapter_id: string;
  chapter_title: string;
  stage: string;
  kind:
    | 'chapter_start'
    | 'stage_start'
    | 'llm_output'
    | 'review_report'
    | 'review_diagnosis'
    | 'gate'
    | 'effect'
    | 'chapter_done'
    | 'chapter_failed'
    | 'paused'
    | 'resumed'
    | 'pipeline_done';
  status: string;
  content: string;
  score?: number;
  attempt: number;
}

// 管线状态快照（含事件缓冲与模型选择，页面切换后恢复现场用）
export interface PipelineState {
  running: boolean;
  paused: boolean;
  current_chapter: string | null;
  events?: PipelineEvent[];
  writing_model?: string | null;
  review_model?: string | null;
}

// 讨论状态快照（后端 DiscussionControl，页面重连用）
export interface DiscussionState {
  running: boolean;
  events?: DiscussionEvent[];
}

// 文风指纹（本书已有章节的确定性统计基线，注入写作/审查 prompt）
export interface StyleFingerprint {
  sampled_chapters: number;
  sampled_chars: number;
  avg_sentence_length: number;
  sentence_var: number;
  avg_paragraph_length: number;
  paragraph_uniformity: number;
  connector_per_1k: number;
  dash_per_1k: number;
  colon_per_1k: number;
  quote_style: string;
  dialogue_ratio: number;
  vocabulary_richness: number;
}
