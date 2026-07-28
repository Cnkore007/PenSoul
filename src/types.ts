export interface Chapter {
  chapter_id: string;
  volume_id?: string;
  title: string;
  content: string;
  word_count: number;
  version: number;
  status: 'Draft' | 'Reviewing' | 'Reviewed' | 'Polished' | 'Published';
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

// 插件/工作流配置
export interface PluginConfig {
  plugin_id: string;
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  stages: PluginStage[];
}

export interface PluginStage {
  name: string;
  display_name: string;
  tool: string;
  gate: 'auto' | 'manual' | 'conditional';
  runner: 'local' | 'delegated';
  prompt_template: string;
  allowed_tools: string[];
  denied_tools: string[];
  timeout_seconds: number;
  max_retries: number;
}

// 项目工作空间数据 — 每个项目独立存储
export interface ProjectData {
  project_id: string;
  volumes: VolumeWithChapters[];
  characters: CharacterData[];
  world: WorldData;
  workflow_id: string | null; // 关联的工作流 plugin_id
  style: StyleMetrics | null;
  settings: ProjectSettings;
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

// 创作规划设置
export interface ProjectSettings {
  // 目标总章节数（0 表示未设定）
  targetChapters: number;
  // 目标总字数（0 表示未设定）
  targetWords: number;
  // 每章目标字数（0 表示未设定）
  chapterTargetWords: number;
  // 故事类型
  genre: string;
  // 预计卷数
  targetVolumes: number;
}

export type ViewType =
  | 'writing'
  | 'outline'
  | 'character'
  | 'world'
  | 'consistency'
  | 'harness'
  | 'style'
  | 'projects'
  | 'llm-settings'
  | 'plugins'
  | 'workflow'
  | 'dashboard';
