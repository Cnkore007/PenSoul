// IPC 持久化层 — 通过 Tauri IPC 与后端通信
import type { ProjectMeta, ProjectData, LlmProvider, LlmModel, PluginConfig, Expert } from "./types";
import * as ipc from "./ipc";

// ── Projects ──

export async function loadProjects(): Promise<ProjectMeta[]> {
  try {
    const list = await ipc.listProjects();
    return list as ProjectMeta[];
  } catch (e) {
    console.error("加载项目列表失败:", e);
    return [];
  }
}

export async function saveProjects(_projects: ProjectMeta[]): Promise<void> {
  // 项目列表现在由后端管理，此函数保留用于兼容性
  // 实际的项目增删改通过 ipc.createProject / updateProject / deleteProject
}

// ── snake_case ↔ camelCase 转换 ──

// 后端 ProjectSettings (snake_case) → 前端 (camelCase)
function transformSettings(raw: any) {
  if (!raw) return { targetChapters: 0, targetWords: 0, chapterTargetWords: 0, genre: '', targetVolumes: 0 };
  return {
    targetChapters: raw.target_chapters ?? raw.targetChapters ?? 0,
    targetWords: raw.target_words ?? raw.targetWords ?? 0,
    chapterTargetWords: raw.chapter_target_words ?? raw.chapterTargetWords ?? 0,
    genre: raw.genre ?? '',
    targetVolumes: raw.target_volumes ?? raw.targetVolumes ?? 0,
  };
}

// 前端 (camelCase) → 后端 ProjectSettings (snake_case)
function toBackendSettings(s: any) {
  return {
    target_chapters: s.targetChapters ?? 0,
    target_words: s.targetWords ?? 0,
    chapter_target_words: s.chapterTargetWords ?? 0,
    target_volumes: s.targetVolumes ?? 0,
    genre: s.genre ?? '',
  };
}

// 后端 CoreConcept (snake_case) → 前端 (camelCase)
function transformConcept(raw: any) {
  if (!raw) return { highConcept: '', premise: '', protagonistHint: '', tone: '', centralConflict: '', inspiration: '' };
  return {
    highConcept: raw.high_concept ?? raw.highConcept ?? '',
    premise: raw.premise ?? '',
    protagonistHint: raw.protagonist_hint ?? raw.protagonistHint ?? '',
    tone: raw.tone ?? '',
    centralConflict: raw.central_conflict ?? raw.centralConflict ?? '',
    inspiration: raw.inspiration ?? '',
  };
}

// 前端 (camelCase) → 后端 CoreConcept (snake_case)
function toBackendConcept(c: any) {
  return {
    high_concept: c.highConcept ?? '',
    premise: c.premise ?? '',
    protagonist_hint: c.protagonistHint ?? '',
    tone: c.tone ?? '',
    central_conflict: c.centralConflict ?? '',
    inspiration: c.inspiration ?? '',
  };
}

// 后端 SproutData (snake_case) → 前端 (camelCase)
function transformSprout(raw: any) {
  if (!raw) return { ideaDescription: '', agents: [] };
  return {
    ideaDescription: raw.idea_description ?? raw.ideaDescription ?? '',
    agents: (raw.agents ?? []).map((a: any) => ({
      id: a.id ?? '',
      name: a.name ?? '',
      model: a.model ?? '',
      prompt: a.prompt ?? '',
      perspective: a.perspective ?? '',
      enabled: a.enabled ?? true,
    })),
  };
}

// 前端 (camelCase) → 后端 SproutData (snake_case)
function toBackendSprout(s: any) {
  return {
    idea_description: s.ideaDescription ?? '',
    agents: (s.agents ?? []).map((a: any) => ({
      id: a.id ?? '',
      name: a.name ?? '',
      model: a.model ?? '',
      prompt: a.prompt ?? '',
      perspective: a.perspective ?? '',
      enabled: a.enabled ?? true,
    })),
  };
}

// 后端 CharacterLayer → 前端 CharacterData[]
// 后端返回 { characters: [...], relationships: [...] }，前端期望扁平的 CharacterData[]
function transformCharacters(raw: any) {
  if (!raw) return [];
  // 后端返回的可能已经是 CharacterLayer 结构
  const chars = raw.characters ?? raw;
  if (!Array.isArray(chars)) return [];
  const layerRelationships = raw.relationships ?? [];
  return chars.map((ch: any) => {
    // core_personality.traits → personality_traits
    const traits = ch.core_personality?.traits ?? ch.personality_traits ?? [];
    // current_mood 可能是 Emotion 对象或字符串
    let mood: string | undefined;
    if (typeof ch.current_mood === 'string') {
      mood = ch.current_mood || undefined;
    } else if (ch.current_mood && typeof ch.current_mood === 'object') {
      mood = ch.current_mood.primary || undefined;
    }
    return {
      id: String(ch.id ?? ''),
      name: ch.name ?? '',
      personality_traits: Array.isArray(traits) ? traits : [],
      current_mood: mood,
      relationships: ch.relationships ?? layerRelationships,
    };
  });
}

// 前端 CharacterData[] → 后端 CharacterLayer
function toBackendCharacters(chars: any[]) {
  return {
    characters: (chars ?? []).map((ch: any) => ({
      id: ch.id ?? '',
      name: ch.name ?? '',
      core_personality: { traits: ch.personality_traits ?? [] },
      current_mood: { primary: ch.current_mood ?? '', intensity: 0.5, secondary: '' },
      current_location: '',
      current_knowledge: { facts: [] },
      state_history: [],
      transition_rules: [],
      dialogue_style: { patterns: [], vocabulary_level: 'normal', sentence_length_avg: 15.0, catchphrases: [] },
      growth_curve: [],
      knowledge_base: { known_facts: [], knowledge_sources: [], decay_model: { half_life_chapters: 10, min_reliability: 0.1 } },
    })),
    relationships: (chars ?? []).flatMap((ch: any) => ch.relationships ?? []),
  };
}

// ── Per-Project Data ──

// 将后端 WorldLayer 结构转换为前端 WorldData 格式
function transformWorldData(raw: any): { locations: any[]; timeline_events: any[]; setting_rules: any[] } {
  if (!raw) {
    return { locations: [], timeline_events: [], setting_rules: [] };
  }
  return {
    locations: raw.spatial_model?.locations ?? raw.locations ?? [],
    timeline_events: raw.timeline?.events ?? raw.timeline_events ?? [],
    setting_rules: raw.setting_rules ?? [],
  };
}

// 将前端 WorldData 格式转换为后端 WorldLayer 结构
function toBackendWorld(world: any): any {
  return {
    world_id: "default",
    name: "default",
    spatial_model: {
      locations: world.locations ?? [],
      hierarchy: [],
    },
    timeline: {
      events: world.timeline_events ?? [],
    },
    setting_rules: world.setting_rules ?? [],
    glossary: [],
    item_graph: [],
  };
}

export async function loadProjectData(projectId: string): Promise<ProjectData> {
  try {
    // 先确保项目已打开
    await ipc.openProject(projectId);

    const [chapters, characters, world, settings, concept, sprout] = await Promise.all([
      ipc.listChapters(),
      ipc.getCharacters(),
      ipc.getWorld(),
      ipc.loadSettings(),
      ipc.loadConcept(),
      ipc.loadSprout(),
    ]);

    // 将 chapters 组织成 volumes 结构
    const volumeMap = new Map<string, { title: string; chapters: any[] }>();
    for (const ch of chapters) {
      const volId = ch.volume_id || "_default";
      if (!volumeMap.has(volId)) {
        volumeMap.set(volId, { title: volId === "_default" ? "默认卷" : volId, chapters: [] });
      }
      volumeMap.get(volId)!.chapters.push(ch);
    }

    const volumes = Array.from(volumeMap.entries()).map(([volId, vol]) => ({
      volume_id: volId,
      title: vol.title,
      chapter_count: vol.chapters.length,
      expanded: true,
      chapters: vol.chapters.map((ch: any) => ({
        chapter_id: ch.chapter_id,
        volume_id: ch.volume_id || volId,
        title: ch.title,
        content: ch.content || "",
        word_count: ch.word_count || 0,
        version: ch.version || 1,
        status: ch.status || "Draft",
      })),
    }));

    return {
      project_id: projectId,
      volumes,
      characters: transformCharacters(characters),
      world: transformWorldData(world),
      workflow_id: null,
      style: null,
      concept: transformConcept(concept),
      sprout: transformSprout(sprout),
      settings: transformSettings(settings),
    };
  } catch (e) {
    console.error("加载项目数据失败:", e);
    return {
      project_id: projectId,
      volumes: [],
      characters: [],
      world: { locations: [], timeline_events: [], setting_rules: [] },
      workflow_id: null,
      style: null,
      concept: {
        highConcept: '',
        premise: '',
        protagonistHint: '',
        tone: '',
        centralConflict: '',
        inspiration: '',
      },
      sprout: {
        ideaDescription: '',
        agents: [],
      },
      settings: {
        targetChapters: 0,
        targetWords: 0,
        chapterTargetWords: 0,
        genre: '',
        targetVolumes: 0,
      },
    };
  }
}

export async function saveProjectData(data: ProjectData): Promise<void> {
  try {
    // 保存各部分数据到后端（先转换为后端 snake_case 格式）
    await Promise.all([
      ipc.saveCharacters(toBackendCharacters(data.characters)),
      ipc.saveWorld(toBackendWorld(data.world)),
      ipc.saveSettings(toBackendSettings(data.settings)),
      ipc.saveConcept(toBackendConcept(data.concept)),
      ipc.saveSprout(toBackendSprout(data.sprout)),
    ]);

    // 保存每个有变更的章节
    for (const vol of data.volumes) {
      for (const ch of vol.chapters) {
        if (ch.content !== undefined) {
          await ipc.saveChapter(ch.chapter_id, ch.content, ch.version - 1);
        }
      }
    }

    await ipc.saveProject();
  } catch (e) {
    console.error("保存项目数据失败:", e);
  }
}

// ── LLM Providers ──

const defaultProviders: LlmProvider[] = [
  { provider_id: "openai", name: "openai", display_name: "OpenAI", api_base: "https://api.openai.com/v1", requires_api_key: true },
  { provider_id: "anthropic", name: "anthropic", display_name: "Anthropic", api_base: "https://api.anthropic.com", requires_api_key: true },
  { provider_id: "deepseek", name: "deepseek", display_name: "DeepSeek", api_base: "https://api.deepseek.com", requires_api_key: true },
  { provider_id: "moonshot", name: "moonshot", display_name: "Moonshot (Kimi)", api_base: "https://api.moonshot.cn/v1", requires_api_key: true },
  { provider_id: "local", name: "local", display_name: "本地模型 (Ollama)", api_base: "http://localhost:11434/v1", requires_api_key: false },
];

export async function loadProviders(): Promise<LlmProvider[]> {
  try {
    const list = await ipc.listProviders();
    return list as LlmProvider[];
  } catch {
    return defaultProviders;
  }
}

export async function saveProviders(providers: LlmProvider[]): Promise<void> {
  await ipc.saveProviders(providers);
}

// ── LLM Models ──

const defaultModels: LlmModel[] = [
  { model_id: "gpt-4o", provider_id: "openai", display_name: "GPT-4o", max_tokens: 128000, supports_tools: true, cost_per_1k_tokens: 0.005, avg_quality_score: 0.92, avg_latency_ms: 1200, is_available: false, api_key_configured: false },
  { model_id: "claude-sonnet-4-20250514", provider_id: "anthropic", display_name: "Claude Sonnet 4", max_tokens: 200000, supports_tools: true, cost_per_1k_tokens: 0.003, avg_quality_score: 0.94, avg_latency_ms: 1500, is_available: false, api_key_configured: false },
  { model_id: "deepseek-chat", provider_id: "deepseek", display_name: "DeepSeek V3", max_tokens: 64000, supports_tools: true, cost_per_1k_tokens: 0.0002, avg_quality_score: 0.88, avg_latency_ms: 2000, is_available: false, api_key_configured: false },
  { model_id: "qwen-2.5-72b", provider_id: "local", display_name: "Qwen 2.5 72B (本地)", max_tokens: 32000, supports_tools: false, cost_per_1k_tokens: 0, avg_quality_score: 0.80, avg_latency_ms: 3000, is_available: false, api_key_configured: false },
];

export async function loadModels(): Promise<LlmModel[]> {
  try {
    const list = await ipc.listModels();
    return list as LlmModel[];
  } catch {
    return defaultModels;
  }
}

export async function saveModels(models: LlmModel[]): Promise<void> {
  await ipc.saveModels(models);
}

// ── API Keys ──

export async function loadApiKeys(): Promise<Record<string, string>> {
  // 实际从后端加载已保存的 API Key（用于 InspirationPanel 等前端直调 LLM 的场景）
  try {
    return await ipc.loadApiKeys();
  } catch {
    return {};
  }
}

export async function saveApiKeys(_keys: Record<string, string>): Promise<void> {
  // 保存通过 ipc.saveApiKey 逐条处理，此函数保留用于兼容
}

// ── Plugins ──

export async function loadPlugins(): Promise<PluginConfig[]> {
  try {
    const list = await ipc.listPlugins();
    return list as PluginConfig[];
  } catch {
    return [];
  }
}

export async function savePlugins(_plugins: PluginConfig[]): Promise<void> {
  // 插件管理通过 ipc.installPlugin / removePlugin / togglePlugin
}

// ── Experts ──

export async function loadExperts(): Promise<Expert[]> {
  try {
    const list = await ipc.loadExperts();
    return list as Expert[];
  } catch {
    return [];
  }
}

export async function saveExperts(experts: Expert[]): Promise<void> {
  await ipc.saveExperts(experts);
}

// ── Delete Project Data ──

export async function deleteProjectData(projectId: string): Promise<void> {
  await ipc.deleteProject(projectId);
}
