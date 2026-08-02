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
}

export interface WorldData {
  locations: Array<{ id: string; name: string; description: string }>;
  timeline_events: Array<{ event_id: string; story_time: string; description: string }>;
  setting_rules: Array<{ rule_id: string; title: string; description: string }>;
}

export interface StyleMetrics {
  avg_sentence_length: number;
  vocabulary_richness: number;
  dialogue_ratio: number;
  pace_score: number;
  ai_pattern_score: number;
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
  hits: number;
  score: number;
  max_score: number;
  examples: string[];
}

export interface AiFlavorReport {
  score: number; // 0-100，越高 AI 味越重
  level: string; // 低 / 中 / 高
  total_hits: number;
  categories: AiFlavorCategory[];
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

// LLM 提供商
export interface LlmProvider {
  provider_id: string;
  name: string;
  display_name: string;
  api_base: string;
  requires_api_key: boolean;
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
}

// 情节脉络节点 —— 大纲规划层（覆盖一个章节范围的剧情规划，展开细纲后才生成可写章节）
export interface OutlineArc {
  arc_id: string;
  title: string;
  description: string;
  chapter_start: number; // 覆盖起始章号（含，从 1 开始）
  chapter_end: number;   // 覆盖结束章号（含）
  expanded_until: number; // 已展开细纲到第几章（0 = 未展开）
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
  locations: Array<{ name: string; description: string }>;
  timeline_events: Array<{ story_time: string; description: string }>;
  setting_rules: Array<{ name: string; description: string }>;
  characters: Array<{
    name: string;
    personality_traits: Array<[string, number]>;
    current_mood?: string;
    description?: string;
    relationships?: Array<{ from: string; to: string; relation_type: string; strength: number }>;
  }>;
  outline_beats: Array<{ title: string; description: string; chapter_hint?: string }>;
  // 讨论中显式保留的分歧与裁决（含跨维度冲突）
  disagreements?: Array<{
    topic: string;
    dimension?: string;
    sides?: Array<{ agent: string; position: string; rationale?: string }>;
    status?: string; // resolved=讨论内已收敛 / open=未收敛
    resolution?: string;
    adjudicated?: boolean;
  }>;
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
