import { invoke } from "@tauri-apps/api/core";
import type {
  Chapter, Project, HarnessStatus, CharacterData, WorldData,
  StyleMetrics, ProjectMeta, LlmProvider, LlmModel, PluginConfig,
  InspirationItem
} from "./types";

// ── 项目管理 ──

export async function createProject(title: string): Promise<string | null> {
  try {
    return await invoke<string>("create_project", { title });
  } catch (e) {
    console.error("create_project failed:", e);
    return null;
  }
}

export async function getProject(projectId: string): Promise<Project | null> {
  try {
    return await invoke<Project>("get_project", { projectId });
  } catch (e) {
    console.error("get_project failed:", e);
    return null;
  }
}

export async function listProjects(): Promise<ProjectMeta[]> {
  try {
    return await invoke<ProjectMeta[]>("list_projects");
  } catch (e) {
    console.error("list_projects failed:", e);
    return [];
  }
}

export async function deleteProject(projectId: string): Promise<boolean> {
  try {
    return await invoke<boolean>("delete_project", { projectId });
  } catch (e) {
    console.error("delete_project failed:", e);
    return false;
  }
}

export async function updateProject(projectId: string, title: string, description: string): Promise<boolean> {
  try {
    return await invoke<boolean>("update_project", { projectId, title, description });
  } catch (e) {
    console.error("update_project failed:", e);
    return false;
  }
}

// ── 章节 ──

export async function getChapter(chapterId: string): Promise<Chapter | null> {
  try {
    return await invoke<Chapter>("get_chapter", { chapterId });
  } catch (e) {
    console.error("get_chapter failed:", e);
    return null;
  }
}

export async function saveChapter(chapterId: string, content: string, expectedVersion: number): Promise<number | null> {
  try {
    return await invoke<number>("save_chapter", { chapterId, content, expectedVersion });
  } catch (e) {
    console.error("save_chapter failed:", e);
    return null;
  }
}

// ── 角色与世界观 ──

export async function getCharacters(): Promise<CharacterData[]> {
  try {
    return await invoke<CharacterData[]>("get_characters");
  } catch (e) {
    console.error("get_characters failed:", e);
    return [];
  }
}

export async function getWorldData(): Promise<WorldData> {
  try {
    return await invoke<WorldData>("get_world_data");
  } catch (e) {
    console.error("get_world_data failed:", e);
    return { locations: [], timeline_events: [], setting_rules: [] };
  }
}

// ── 一致性与 Harness ──

export async function checkConsistency(): Promise<import("./types").ConsistencyViolation[]> {
  try {
    return await invoke<import("./types").ConsistencyViolation[]>("check_consistency");
  } catch (e) {
    console.error("check_consistency failed:", e);
    return [];
  }
}

export async function getHarnessStatus(): Promise<HarnessStatus | null> {
  try {
    return await invoke<HarnessStatus>("get_harness_status");
  } catch (e) {
    console.error("get_harness_status failed:", e);
    return null;
  }
}

export async function advanceHarnessStage(): Promise<boolean> {
  try {
    return await invoke<boolean>("advance_harness_stage");
  } catch (e) {
    console.error("advance_harness_stage failed:", e);
    return false;
  }
}

// ── 文风 ──

export async function getStyleMetrics(): Promise<StyleMetrics | null> {
  try {
    return await invoke<StyleMetrics>("get_style_metrics");
  } catch (e) {
    console.error("get_style_metrics failed:", e);
    return null;
  }
}

// ── LLM 设置 ──

export async function listProviders(): Promise<LlmProvider[]> {
  try {
    return await invoke<LlmProvider[]>("list_providers");
  } catch (e) {
    console.error("list_providers failed:", e);
    return [];
  }
}

export async function listModels(): Promise<LlmModel[]> {
  try {
    return await invoke<LlmModel[]>("list_models");
  } catch (e) {
    console.error("list_models failed:", e);
    return [];
  }
}

export async function saveApiKey(providerId: string, apiKey: string): Promise<boolean> {
  try {
    return await invoke<boolean>("save_api_key", { providerId, apiKey });
  } catch (e) {
    console.error("save_api_key failed:", e);
    return false;
  }
}

export async function testModel(modelId: string): Promise<boolean> {
  try {
    return await invoke<boolean>("test_model", { modelId });
  } catch (e) {
    console.error("test_model failed:", e);
    return false;
  }
}

export async function setModelPreference(modelId: string, enabled: boolean): Promise<boolean> {
  try {
    return await invoke<boolean>("set_model_preference", { modelId, enabled });
  } catch (e) {
    console.error("set_model_preference failed:", e);
    return false;
  }
}

// ── 灵感生成 ──

export async function generateInspiration(
  contextType: string,
  contextData: string,
): Promise<InspirationItem[]> {
  try {
    return await invoke<InspirationItem[]>("generate_inspiration", { contextType, contextData });
  } catch (e) {
    console.error("generate_inspiration failed:", e);
    return [];
  }
}

// ── 插件/工作流 ──

export async function listPlugins(): Promise<PluginConfig[]> {
  try {
    return await invoke<PluginConfig[]>("list_plugins");
  } catch (e) {
    console.error("list_plugins failed:", e);
    return [];
  }
}

export async function installPlugin(yamlContent: string): Promise<boolean> {
  try {
    return await invoke<boolean>("install_plugin", { yamlContent });
  } catch (e) {
    console.error("install_plugin failed:", e);
    return false;
  }
}

export async function removePlugin(pluginId: string): Promise<boolean> {
  try {
    return await invoke<boolean>("remove_plugin", { pluginId });
  } catch (e) {
    console.error("remove_plugin failed:", e);
    return false;
  }
}

export async function togglePlugin(pluginId: string, enabled: boolean): Promise<boolean> {
  try {
    return await invoke<boolean>("toggle_plugin", { pluginId, enabled });
  } catch (e) {
    console.error("toggle_plugin failed:", e);
    return false;
  }
}

export async function exportPlugin(pluginId: string): Promise<string | null> {
  try {
    return await invoke<string>("export_plugin", { pluginId });
  } catch (e) {
    console.error("export_plugin failed:", e);
    return null;
  }
}

// ── HTTP 代理（绕过 WebView CSP） ──

export async function httpRequest(url: string, method: string, headers?: Record<string, string>, body?: string): Promise<{status: number, statusText: string, body: string, ok: boolean}> {
  try {
    return await invoke<{status: number, statusText: string, body: string, ok: boolean}>("http_request", { request: { url, method, headers, body } });
  } catch (e) {
    console.error("http_request failed:", e);
    return { status: 0, statusText: "Error", body: String(e), ok: false };
  }
}
