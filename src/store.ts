// IPC 持久化层 — 通过 Tauri IPC 与后端通信
import type { ProjectMeta, ProjectData, LlmProvider, LlmModel, Expert } from "./types";
import * as ipc from "./ipc";
import { computeEffectiveSkills } from "./workflow";

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
// 全字段透传：expert_id / skill_path / presets_dismissed / last_discussion 一个都不能丢
function transformSprout(raw: any) {
  if (!raw) return { ideaDescription: '', agents: [], lastDiscussion: undefined };
  return {
    ideaDescription: raw.idea_description ?? raw.ideaDescription ?? '',
    agents: (raw.agents ?? []).map((a: any) => ({
      id: a.id ?? '',
      name: a.name ?? '',
      model: a.model ?? '',
      prompt: a.prompt ?? '',
      perspective: a.perspective ?? '',
      enabled: a.enabled ?? true,
      expertId: a.expert_id ?? undefined,
      skillPath: a.skill_path ?? undefined,
    })),
    presetsDismissed: raw.presets_dismissed ?? false,
    // 最近一次讨论结果（发言 + 成果），切换页面/重启后恢复
    lastDiscussion: raw.last_discussion
      ? {
          turns: raw.last_discussion.turns ?? [],
          synthesis: raw.last_discussion.synthesis ?? null,
          authorFeedback: raw.last_discussion.author_feedback ?? "",
        }
      : undefined,
  };
}

// 前端 (camelCase) → 后端 SproutData (snake_case)
// last_discussion 为 undefined 时发 null，后端会保留已有讨论结果（见 save_sprout）
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
      expert_id: a.expertId ?? null,
      skill_path: a.skillPath ?? null,
    })),
    presets_dismissed: s.presetsDismissed ?? false,
    last_discussion: s.lastDiscussion
      ? {
          turns: s.lastDiscussion.turns ?? [],
          synthesis: s.lastDiscussion.synthesis ?? null,
          author_feedback: s.lastDiscussion.authorFeedback ?? "",
        }
      : null,
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
    // 顶层关系按「关系双方」分发给相关角色（旧实现把全量关系塞给每个角色，
    // 保存时 flatMap 会按角色数成倍膨胀——曾导致项目 JSON 膨胀到 93MB）
    const mine = (ch.relationships ?? []).length > 0
      ? ch.relationships
      : layerRelationships.filter((r: any) =>
          (r.from ?? "") === (ch.name ?? "") || (r.to ?? "") === (ch.name ?? "")
        );
    return {
      id: String(ch.id ?? ''),
      name: ch.name ?? '',
      personality_traits: Array.isArray(traits) ? traits : [],
      current_mood: mood,
      relationships: mine,
      wants: ch.wants ?? '',
      fears: ch.fears ?? '',
      secret: ch.secret ?? '',
      speech_style: ch.speech_style ?? '',
      arc_stages: ch.arc_stages ?? [],
      knows: ch.knows ?? [],
      does_not_know: ch.does_not_know ?? [],
      sources: ch.sources ?? [],
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
      wants: ch.wants ?? '',
      fears: ch.fears ?? '',
      secret: ch.secret ?? '',
      speech_style: ch.speech_style ?? '',
      arc_stages: ch.arc_stages ?? [],
      knows: ch.knows ?? [],
      does_not_know: ch.does_not_know ?? [],
      sources: ch.sources ?? [],
      knowledge_base: { known_facts: [], knowledge_sources: [], decay_model: { half_life_chapters: 10, min_reliability: 0.1 } },
    })),
    relationships: (chars ?? []).flatMap((ch: any) =>
      (ch.relationships ?? []).map((r: any) => ({
        from: r.from ?? "",
        to: r.to ?? "",
        relation_type: r.relation_type ?? "",
        // LLM 产物可能把强度给成字符串或中文描述，统一钳到数字
        strength: Number.isFinite(Number(r.strength)) ? Number(r.strength) : 0.5,
        history: [],
      }))
    ).filter((r, i, arr) => {
      // 去重：同一对实体 + 关系类型只保留一条（防保存链路重复膨胀）
      return arr.findIndex(x => x.from === r.from && x.to === r.to && x.relation_type === r.relation_type) === i;
    }),
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
// 注意：后端 WorldLayer 反序列化要求字段齐全，缺字段会导致整个 save_world 失败（重启后世界观丢失）
function toBackendWorld(world: any): any {
  return {
    world_id: "default",
    name: "default",
    spatial_model: {
      locations: (world.locations ?? []).map((l: any) => ({
        id: l.id,
        name: l.name ?? "",
        description: l.description ?? "",
        level: l.level ?? "",
        region: l.region ?? "",
        faction: l.faction ?? "",
        unlocked_chapter: l.unlocked_chapter ?? "",
        spatial_tags: l.spatial_tags ?? [],
        sources: l.sources ?? [],
      })),
      hierarchy: [],
    },
    timeline: {
      events: (world.timeline_events ?? []).map((e: any) => ({
        event_id: e.event_id,
        story_time: e.story_time ?? "",
        chapter_id: e.chapter_id ?? "",
        description: e.description ?? "",
        participants: e.participants ?? [],
        sources: e.sources ?? [],
      })),
      epoch_markers: [],
    },
    setting_rules: (world.setting_rules ?? []).map((r: any) => ({
      rule_id: r.rule_id,
      category: r.category ?? "",
      title: r.title ?? "",
      description: r.description ?? "",
      constraints: r.constraints ?? [],
      cost: r.cost ?? "",
      loophole: r.loophole ?? "",
      sources: r.sources ?? [],
    })),
    glossary: [],
    item_graph: [],
  };
}

// 从后端拉取项目全量数据并组装成前端 ProjectData（不含 open_project 切换）
async function fetchProjectData(projectId: string): Promise<ProjectData> {
  try {
    const [chapters, characters, world, settings, concept, sprout, volumesMeta, outlineArcs, workflowRef, workflowTemplates, blueprint] = await Promise.all([
      ipc.listChapters(),
      ipc.getCharacters(),
      ipc.getWorld(),
      ipc.loadSettings(),
      ipc.loadConcept(),
      ipc.loadSprout(),
      ipc.getVolumes(),
      ipc.listOutlineArcs().catch(() => []), // 老版本后端无此命令时降级为空
      ipc.loadWorkflowRef().catch(() => null), // 未配置过/老版本后端时为 null
      ipc.listWorkflowTemplates().catch(() => []), // 全局模板（用于合并项目有效配置）
      ipc.getBlueprint().catch(() => null), // 老版本后端无此命令时降级为空
    ]);

    // 卷元数据（标题以持久化的卷列表为准）
    const volTitleMap = new Map<string, string>(
      (volumesMeta ?? []).map((v: any) => [v.volume_id as string, (v.title as string) || ""]),
    );

    // 将 chapters 组织成 volumes 结构
    const volumeMap = new Map<string, any[]>();
    for (const ch of chapters) {
      const volId = ch.volume_id || "_default";
      if (!volumeMap.has(volId)) volumeMap.set(volId, []);
      volumeMap.get(volId)!.push(ch);
    }

    const mapChapter = (ch: any, volId: string) => ({
      chapter_id: ch.chapter_id,
      chapter_no: ch.chapter_no ?? 0,
      volume_id: ch.volume_id || volId,
      title: ch.title,
      summary: ch.summary || "",
      content: ch.content || "",
      word_count: ch.word_count || 0,
      version: ch.version || 1,
      status: ch.status || "Draft",
    });
    const volMetaMap = new Map<string, any>((volumesMeta ?? []).map((v: any) => [v.volume_id as string, v]));
    const toVolume = (volId: string, chs: any[]) => {
      const meta = volMetaMap.get(volId) as any;
      return {
        volume_id: volId,
        title: meta?.title || volTitleMap.get(volId) || (volId === "_default" ? "默认卷" : volId),
        chapter_count: chs.length,
        // 卷展开状态持久化（后端 Volume.expanded），缺省展开
        expanded: meta?.expanded ?? true,
        chapters: chs.map(ch => mapChapter(ch, volId)),
      };
    };

    // 卷顺序：先按后端卷列表，再补上只有章节没有元数据的卷
    const orderedVolIds = [
      // 保留空卷（导入的分卷大纲此时还没有章节，不能被过滤掉）
      ...(volumesMeta ?? []).map((v: any) => v.volume_id as string),
      ...Array.from(volumeMap.keys()).filter(id => !(volumesMeta ?? []).some((v: any) => v.volume_id === id)),
    ];
    const volumes = orderedVolIds.map(volId => toVolume(volId, volumeMap.get(volId) ?? []));

    // 项目工作流引用：旧项目可能没有 workflow_ref，只有遗留 workflow_skills，
    // 此时把遗留配置当作项目覆盖（模板未选，绑定照常生效）
    let ref = (workflowRef as any) ?? null;
    if (!ref) {
      const legacy = await ipc.loadWorkflowSkills().catch(() => null);
      if (legacy && typeof legacy === "object" && (legacy as any).outline_expand) {
        ref = { template_id: null, template_version: null, overrides: legacy };
      }
    }

    return {
      project_id: projectId,
      volumes,
      characters: transformCharacters(characters),
      world: transformWorldData(world),
      workflowRef: ref,
      // 派生有效配置：项目覆盖 → 模板绑定 合并（大纲展开/造化工坊直接消费）
      workflowSkills: computeEffectiveSkills(workflowTemplates as any[], ref),
      style: null,
      concept: transformConcept(concept),
      sprout: transformSprout(sprout),
      settings: transformSettings(settings),
      outlineArcs: outlineArcs ?? [],
      blueprint: blueprint ?? emptyBlueprint(),
    };
  } catch (e) {
    console.error("加载项目数据失败:", e);
    return {
      project_id: projectId,
      volumes: [],
      characters: [],
      world: { locations: [], timeline_events: [], setting_rules: [] },
      workflowRef: null,
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
      outlineArcs: [],
      blueprint: emptyBlueprint(),
    };
  }
}

// 空蓝图（未定盘）
function emptyBlueprint() {
  return {
    settled: false,
    settled_at: "",
    settled_from: "",
    commitments: [],
    volumes: [],
    character_matrix: [],
    foreshadows: [],
    subplots: [],
    resources: [],
    dossiers: [],
    current_state: {
      as_of_chapter: 0,
      characters: [],
      world_state: [],
      active_plots: [],
      relationships: [],
      loose_ends: [],
      last_events: [],
    },
  };
}

// 首次打开项目：先切换后端活跃项目（重建引擎），再拉取全量数据
export async function loadProjectData(projectId: string): Promise<ProjectData> {
  await ipc.openProject(projectId);
  return fetchProjectData(projectId);
}

// 页面切换时的轻量刷新：只重新拉取数据，不调 open_project——
// open_project 会整体重建 Harness 引擎与派生状态，写作管线/讨论运行中调用会断现场
export async function refreshProjectData(projectId: string): Promise<ProjectData> {
  return fetchProjectData(projectId);
}

export async function saveProjectData(data: ProjectData): Promise<void> {
  // 各环节独立容错：单点失败不再中断其余保存；错误汇总后抛出，
  // 让调用方有机会告知用户（此前静默 catch 导致章节/人物"假保存"）
  const errors: string[] = [];
  const names = ["人物", "世界观", "创作设定", "核心概念", "灵魂萌芽", "工作流引用"];
  const results = await Promise.allSettled([
    ipc.saveCharacters(toBackendCharacters(data.characters)),
    ipc.saveWorld(toBackendWorld(data.world)),
    ipc.saveSettings(toBackendSettings(data.settings)),
    ipc.saveConcept(toBackendConcept(data.concept)),
    ipc.saveSprout(toBackendSprout(data.sprout)),
    ipc.saveWorkflowRef(data.workflowRef ?? null),
  ]);
  results.forEach((r, i) => {
    if (r.status === "rejected") errors.push(`${names[i]}保存失败: ${r.reason}`);
  });

  // 保存每个章节（upsert：新建章节也能落盘，梗概随章节一起持久化）
  for (const vol of data.volumes) {
    for (const ch of vol.chapters) {
      try {
        await ipc.upsertChapter(
          ch.chapter_id,
          ch.volume_id || vol.volume_id,
          ch.title,
          ch.content ?? "",
          ch.summary ?? "",
          ch.status,
        );
      } catch (e) {
        errors.push(`章节「${ch.title}」保存失败: ${e}`);
      }
    }
  }
  try {
    // 保存卷元数据（标题 + 展开状态，切页后不丢）
    await ipc.saveVolumes(data.volumes.map(v => ({ volume_id: v.volume_id, title: v.title, expanded: v.expanded })));
    await ipc.saveProject();
  } catch (e) {
    errors.push(`项目落盘失败: ${e}`);
  }

  if (errors.length > 0) {
    console.error("保存项目数据失败:", errors);
    throw new Error(errors.join("\n"));
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

// 选取默认模型：优先「设为默认」且可用，其次第一个可用，最后第一个
export function pickDefaultModel(models: LlmModel[]): LlmModel | undefined {
  return (
    models.find(m => m.is_default && m.is_available)
    ?? models.find(m => m.is_available)
    ?? models[0]
  );
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
