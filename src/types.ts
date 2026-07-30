export interface Chapter {
  chapter_id: string;
  volume_id?: string;
  title: string;
  summary?: string; // 章节梗概（大纲层信息，非正文）
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
  // 核心概念 / 高概念种子
  concept: CoreConceptData;
  // 萌芽数据 — 想法描述 + Agent 讨论配置
  sprout: SproutData;
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
  | 'consistency'
  | 'harness'
  | 'style'
  | 'projects'
  | 'llm-settings'
  | 'plugins'
  | 'workflow'
  | 'dashboard';
