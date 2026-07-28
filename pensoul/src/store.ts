// localStorage 持久化层 — 项目级数据隔离
import type { ProjectMeta, ProjectData, LlmProvider, LlmModel, PluginConfig } from "./types";

const KEYS = {
  projects: "pensoul_projects",
  apiKeys: "pensoul_api_keys",
  models: "pensoul_models",
  providers: "pensoul_providers",
  projectData: "pensoul_project_data", // Record<projectId, ProjectData>
  plugins: "pensoul_plugins",
} as const;

// ── Projects ──

const defaultProjects: ProjectMeta[] = [];

export function loadProjects(): ProjectMeta[] {
  try {
    const raw = localStorage.getItem(KEYS.projects);
    if (raw) return JSON.parse(raw);
  } catch {}
  saveProjects(defaultProjects);
  return defaultProjects;
}

export function saveProjects(projects: ProjectMeta[]) {
  localStorage.setItem(KEYS.projects, JSON.stringify(projects));
}

// ── Per-Project Data ──

export function loadProjectData(projectId: string): ProjectData {
  try {
    const raw = localStorage.getItem(KEYS.projectData);
    if (raw) {
      const all: Record<string, ProjectData> = JSON.parse(raw);
      if (all[projectId]) return all[projectId];
    }
  } catch {}
  // 返回空白项目数据
  return {
    project_id: projectId,
    volumes: [],
    characters: [],
    world: { locations: [], timeline_events: [], setting_rules: [] },
    workflow_id: null,
    style: null,
    settings: {
      targetChapters: 0,
      targetWords: 0,
      chapterTargetWords: 0,
      genre: '',
      targetVolumes: 0,
    },
  };
}

export function saveProjectData(data: ProjectData) {
  let all: Record<string, ProjectData> = {};
  try {
    const raw = localStorage.getItem(KEYS.projectData);
    if (raw) all = JSON.parse(raw);
  } catch {}
  all[data.project_id] = data;
  localStorage.setItem(KEYS.projectData, JSON.stringify(all));
}

// ── LLM Providers ──

const defaultProviders: LlmProvider[] = [
  { provider_id: "openai", name: "openai", display_name: "OpenAI", api_base: "https://api.openai.com/v1", requires_api_key: true },
  { provider_id: "anthropic", name: "anthropic", display_name: "Anthropic", api_base: "https://api.anthropic.com", requires_api_key: true },
  { provider_id: "deepseek", name: "deepseek", display_name: "DeepSeek", api_base: "https://api.deepseek.com", requires_api_key: true },
  { provider_id: "moonshot", name: "moonshot", display_name: "Moonshot (Kimi)", api_base: "https://api.moonshot.cn/v1", requires_api_key: true },
  { provider_id: "local", name: "local", display_name: "本地模型 (Ollama)", api_base: "http://localhost:11434/v1", requires_api_key: false },
];

export function loadProviders(): LlmProvider[] {
  try {
    const raw = localStorage.getItem(KEYS.providers);
    if (raw) return JSON.parse(raw);
  } catch {}
  saveProviders(defaultProviders);
  return defaultProviders;
}

export function saveProviders(providers: LlmProvider[]) {
  localStorage.setItem(KEYS.providers, JSON.stringify(providers));
}

// ── LLM Models ──

const defaultModels: LlmModel[] = [
  { model_id: "gpt-4o", provider_id: "openai", display_name: "GPT-4o", max_tokens: 128000, supports_tools: true, cost_per_1k_tokens: 0.005, avg_quality_score: 0.92, avg_latency_ms: 1200, is_available: false, api_key_configured: false },
  { model_id: "claude-sonnet-4-20250514", provider_id: "anthropic", display_name: "Claude Sonnet 4", max_tokens: 200000, supports_tools: true, cost_per_1k_tokens: 0.003, avg_quality_score: 0.94, avg_latency_ms: 1500, is_available: false, api_key_configured: false },
  { model_id: "deepseek-chat", provider_id: "deepseek", display_name: "DeepSeek V3", max_tokens: 64000, supports_tools: true, cost_per_1k_tokens: 0.0002, avg_quality_score: 0.88, avg_latency_ms: 2000, is_available: false, api_key_configured: false },
  { model_id: "qwen-2.5-72b", provider_id: "local", display_name: "Qwen 2.5 72B (本地)", max_tokens: 32000, supports_tools: false, cost_per_1k_tokens: 0, avg_quality_score: 0.80, avg_latency_ms: 3000, is_available: false, api_key_configured: false },
];

export function loadModels(): LlmModel[] {
  try {
    const raw = localStorage.getItem(KEYS.models);
    if (raw) return JSON.parse(raw);
  } catch {}
  saveModels(defaultModels);
  return defaultModels;
}

export function saveModels(models: LlmModel[]) {
  localStorage.setItem(KEYS.models, JSON.stringify(models));
}

// ── API Keys ──
// 安全提示：API Key 存储在 WebView localStorage 中，仅供前端直接调用 LLM API 使用。
// 密钥不会被发送到 PenSoul 后端，也不会在 Tauri IPC 日志中序列化。
// 如需更高的安全性，建议使用后端代理模式或操作系统密钥链。

export function loadApiKeys(): Record<string, string> {
  try {
    const raw = localStorage.getItem(KEYS.apiKeys);
    if (raw) return JSON.parse(raw);
  } catch {}
  return {};
}

export function saveApiKeys(keys: Record<string, string>) {
  localStorage.setItem(KEYS.apiKeys, JSON.stringify(keys));
}

// ── Plugins ──

export function loadPlugins(): PluginConfig[] {
  try {
    const raw = localStorage.getItem(KEYS.plugins);
    if (raw) return JSON.parse(raw);
  } catch {}
  return [];
}

export function savePlugins(plugins: PluginConfig[]) {
  localStorage.setItem(KEYS.plugins, JSON.stringify(plugins));
}

// ── Delete Project Data ──

export function deleteProjectData(projectId: string) {
  try {
    const raw = localStorage.getItem(KEYS.projectData);
    if (raw) {
      const all: Record<string, ProjectData> = JSON.parse(raw);
      delete all[projectId];
      localStorage.setItem(KEYS.projectData, JSON.stringify(all));
    }
  } catch {}
}
