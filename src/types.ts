// types.ts — 前后端共享类型定义

/// 视图类型
export type ViewType =
  | "projects"
  | "dashboard"
  | "concept"
  | "entity-graph"
  | "constraint"
  | "writing"
  | "outline"
  | "settings";

/// 项目元数据
export interface ProjectMeta {
  project_id: string;
  title: string;
  description: string;
  genre: string;
  created_at: string;
  updated_at: string;
}

/// 作品库列表项：内部 ID + 展示标题
export interface ProjectSummary {
  project_id: string;
  title: string;
}

/// 灵魂萌芽：对话消息
export interface SproutMessage {
  role: "user" | "assistant";
  content: string;
  created_at: string;
}

/// 灵魂萌芽：提案中的世界观设定
export interface SproutSettingProposal {
  name: string;
  category: string;
  description: string;
}

/// 灵魂萌芽：提案中的大纲脉络
export interface SproutArcProposal {
  title: string;
  description: string;
  chapter_start: number;
  chapter_end: number;
}

/// 灵魂萌芽：LLM 生成的项目提案
export interface SproutProposal {
  high_concept: string;
  premise: string;
  protagonist_hint: string;
  tone: string;
  central_conflict: string;
  inspiration: string;
  genre: string;
  world_rules: string[];
  world_settings: SproutSettingProposal[];
  outline_arcs: SproutArcProposal[];
}

/// 灵魂萌芽：会话
export interface SproutSession {
  messages: SproutMessage[];
  pending_proposal: SproutProposal | null;
}

/// 章节状态
export type ChapterStatus =
  | "Draft"
  | "Reviewing"
  | "Reviewed"
  | "Polished"
  | "Published";

/// 实体类型
export type EntityType = "Character" | "Event" | "Setting" | "Foreshadow" | "Organization";

/// 实体摘要
export interface EntitySummary {
  id: string;
  type: EntityType;
  name: string;
}

/// 图谱统计
export interface GraphStats {
  total_entities: number;
  total_relations: number;
  entities_by_type: Record<string, number>;
  avg_relations_per_entity: number;
}

/// 影响预测
export interface ImpactPrediction {
  entity_id: string;
  entity_name: string;
  severity: "Direct" | "Indirect" | "Cascading";
  distance: number;
  reason: string;
  suggested_action: string;
}

/// 约束检查结果
export interface ConstraintReport {
  checked_entities: number;
  has_issues: boolean;
  error_count: number;
  warning_count: number;
}

/// 管线事件
export interface PipelineEvent {
  kind: string;
  message: string;
  timestamp: string;
}

// ---- 新增类型 ----

/// 项目概览
export interface ProjectOverview {
  title: string;
  description: string;
  character_count: number;
  event_count: number;
  setting_count: number;
  foreshadow_count: number;
  chapter_count: number;
  volume_count: number;
  outline_count: number;
  total_words: number;
  high_concept: string;
  tone: string;
  pipeline?: {
    stages: PipelineStage[];
    next_action: string;
  };
}

export interface PipelineStage {
  id: string;
  label: string;
  ready: boolean;
  detail: string;
}

/// 角色详情
export interface Character {
  id: string;
  name: string;
  age: number | null;
  occupation: string | null;
  personality: [string, number][];
  appearance: string | null;
  backstory: string | null;
  wants: string | null;
  fears: string | null;
  secret: string | null;
  // P0 档案化扩展（人物档案）
  attire: string | null;
  techniques: string[];
  realm: string | null;
  items: string[];
}

/// 组织档案（P0）
export interface Organization {
  id: string;
  name: string;
  category: string;
  structure: string;
  goals: string;
  rules: string[];
  description: string;
}

/// 地点
export interface Location {
  id: string;
  name: string;
  category: string;
  rules: string[];
  description: string;
}

/// 时间线事件
export interface TimelineEvent {
  id: string;
  name: string;
  chapter_id: number;
  story_time: string;
  description: string;
}

/// 伏笔
export interface Foreshadow {
  id: string;
  name: string;
  description: string;
  status: string;
  planted_chapter: number;
  expected_payoff: number | null;
  actual_payoff: number | null;
}

/// 大纲脉络
export interface OutlineArc {
  arc_id: string;
  title: string;
  description: string;
  chapter_start: number;
  chapter_end: number;
  chapter_count: number;
  expanded_until: number;
}

/// 章节
export interface Chapter {
  chapter_id: string;
  chapter_no: number;
  title: string;
  summary: string;
  word_count: number;
  status: string;
  version: number;
  consistency_score: number;
}

/// 蓝图数据
export interface BlueprintData {
  settled: boolean;
  settled_at: string;
  commitment_count: number;
  volume_count: number;
  character_count: number;
  foreshadow_count: number;
  subplot_count: number;
  resource_count: number;
  commitments: { id: string; statement: string; kind: string; priority: number; status: string }[];
  volumes: { volume_no: number; title: string; one_line: string; function: string; chapter_start: number; chapter_end: number; status: string }[];
}

/// 核心概念
export interface CoreConcept {
  high_concept: string;
  premise: string;
  protagonist_hint: string;
  tone: string;
  central_conflict: string;
  inspiration: string;
}

/// 章节正文详情
export interface ChapterContent {
  chapter_id: string;
  content: string;
  word_count: number;
  version: number;
  consistency_score: number;
  revision_count: number;
  /** P2 批注（章节派生数据） */
  annotations?: ChapterAnnotation[];
}

export interface ChapterAnnotation {
  annotation_id: string;
  kind: string;
  content: string;
  status: string;
  created_at: string;
}

/// 记忆检索结果
export interface EntityMemory {
  entity: { entity_id: string; entity_type: string; label: string | null };
  relevance_score: number;
  summary: string;
  details: string;
}

export interface MemoryPacket {
  entities: EntityMemory[];
  temporal_context: string;
  emotional_context: string;
  total_tokens: number;
  budget_used: {
    total_tokens: number;
    entity_tokens: number;
    temporal_tokens: number;
    emotional_tokens: number;
  };
}

/// LLM 配置条目（对外视图，密钥脱敏）
export interface ProviderConfig {
  id: string;
  name: string;
  provider: string;
  model_id: string;
  base_url: string;
  has_key: boolean;
  api_key_masked: string;
  context_window: number;
  max_output_tokens: number;
  input_budget: number;
  thinking_mode: "None" | "Always" | "Toggleable";
  supports_streaming: boolean;
  temperature: number | null;
  top_p: number | null;
  frequency_penalty: number | null;
  presence_penalty: number | null;
  stop_sequences: string | null;
  json_mode: boolean | null;
  thinking_budget: number | null;
  timeout_seconds: number;
  doc_url: string | null;
  notes: string | null;
  enabled: boolean;
}

export interface LlmConfigs {
  default_provider_id: string | null;
  providers: ProviderConfig[];
  config_file: string;
}

/// LLM 配置状态概览
export interface LlmStatus {
  configured_count: number;
  total_count: number;
  has_default: boolean;
  config_file: string;
}

/// LLM 测试结果
export interface LlmTestResult {
  config_id: string;
  model: string;
  content: string;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  } | null;
}

/// 上下文检测结果
export interface ContextCheckResult {
  chars: number;
  cjk_chars: number;
  other_chars: number;
  estimated_tokens: number;
  context_window: number;
  input_budget: number;
  usage_percent: number;
  fits: boolean;
}

/// 抓取的官方文档
export interface RemoteDoc {
  model_id: string;
  url: string;
  title: string;
  description: string;
  text_preview: string;
  saved_file: string;
  fetched_at: string;
}

/// 供应商模型条目
export interface RemoteModel {
  id: string;
  display_name: string | null;
  owned_by: string | null;
}

/// 模型列表拉取结果（含缓存信息）
export interface PullModelsResult {
  config_id: string;
  fetched_at: string;
  models: RemoteModel[];
}

/// 文档中提取的模型参数
export interface ModelDocParams {
  context_window: number | null;
  max_output_tokens: number | null;
  thinking_supported: boolean | null;
  notes: string[];
  sources: { title: string; url: string }[];
}

/// 模型文档与参数提取结果
export interface ModelDocResult {
  suggested_url: string;
  doc: RemoteDoc;
  params: ModelDocParams;
}
