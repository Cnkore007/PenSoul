import { invoke } from "@tauri-apps/api/core";

// ── 项目管理 ──

export async function createProject(title: string): Promise<string> {
  return await invoke<string>("create_project", { title });
}

export async function listProjects(): Promise<any[]> {
  return await invoke<any[]>("list_projects");
}

export async function getProject(projectId: string): Promise<any> {
  return await invoke<any>("get_project", { projectId });
}

export async function updateProject(projectId: string, title: string, description: string): Promise<void> {
  await invoke("update_project", { projectId, title, description });
}

export async function deleteProject(projectId: string): Promise<void> {
  await invoke("delete_project", { projectId });
}

export async function openProject(projectId: string): Promise<void> {
  await invoke("open_project", { projectId });
}

export async function saveProject(): Promise<void> {
  await invoke("save_project");
}

// ── 章节 ──

export async function listChapters(): Promise<any[]> {
  return await invoke<any[]>("list_chapters");
}

export async function getChapter(chapterId: string): Promise<any> {
  return await invoke<any>("get_chapter", { chapterId });
}

export async function saveChapter(chapterId: string, content: string, expectedVersion: number): Promise<number> {
  return await invoke<number>("save_chapter", { chapterId, content, expectedVersion });
}

// ── 角色 ──

export async function getCharacters(): Promise<any> {
  return await invoke<any>("get_characters");
}

export async function saveCharacters(characters: any): Promise<void> {
  await invoke("save_characters", { characters });
}

// ── 世界观 ──

export async function getWorld(): Promise<any> {
  return await invoke<any>("get_world");
}

export async function saveWorld(world: any): Promise<void> {
  await invoke("save_world", { world });
}

// ── 一致性 ──

export async function checkConsistency(): Promise<any[]> {
  return await invoke<any[]>("check_consistency");
}

// ── CDA 影响图 ──

export async function findAffected(chapterId: string, changedEntities: string[]): Promise<any[]> {
  return await invoke<any[]>("find_affected_chapters", { chapterId, changedEntities });
}

export async function getImpactGraph(): Promise<any> {
  return await invoke<any>("get_impact_graph");
}

// ── Harness 流程引擎 ──

export async function getHarnessStatus(): Promise<any> {
  return await invoke<any>("get_harness_status");
}

export async function startHarnessStage(): Promise<string> {
  return await invoke<string>("start_harness_stage");
}

export async function completeHarnessStage(result: any): Promise<void> {
  await invoke("complete_harness_stage", { result });
}

export async function injectMemo(key: string, value: any): Promise<void> {
  await invoke("inject_memo", { key, value });
}

// ── 记忆系统 ──

export async function buildMemoryPacket(chapterId: string): Promise<any> {
  return await invoke<any>("build_memory_packet", { chapterId });
}

export async function getHotMemory(): Promise<any> {
  return await invoke<any>("get_hot_memory");
}

export async function getWarmMemory(): Promise<any> {
  return await invoke<any>("get_warm_memory");
}

// ── 文风 ──

export async function getStyleMetrics(): Promise<any> {
  return await invoke<any>("get_style_metrics");
}

// ── LLM 管理 ──

export async function listProviders(): Promise<any[]> {
  return await invoke<any[]>("list_providers");
}

export async function saveProviders(providers: any[]): Promise<void> {
  await invoke("save_providers", { providers });
}

export async function listModels(): Promise<any[]> {
  return await invoke<any[]>("list_models");
}

export async function saveModels(models: any[]): Promise<void> {
  await invoke("save_models", { models });
}

export async function saveApiKey(providerId: string, apiKey: string): Promise<void> {
  await invoke("save_api_key", { providerId, apiKey });
}

export async function testModel(modelId: string): Promise<boolean> {
  return await invoke<boolean>("test_model", { modelId });
}

export async function setModelPreference(modelId: string, enabled: boolean): Promise<void> {
  await invoke("set_model_preference", { modelId, enabled });
}

export async function routeModel(taskType: string): Promise<any> {
  return await invoke<any>("route_model", { taskType });
}

// ── 灵感生成 ──

export async function generateInspiration(contextType: string, contextData: string): Promise<any[]> {
  return await invoke<any[]>("generate_inspiration", { contextType, contextData });
}

// ── 概念讨论（真实 LLM 调用） ──

export interface DiscussAgent {
  id: string;
  name: string;
  model: string;
  prompt: string;
  perspective: string;
  enabled: boolean;
}

export interface DiscussResult {
  agent_id: string;
  agent_name: string;
  perspective: string;
  response: string;
}

export async function discussConcept(ideaDescription: string, agents: DiscussAgent[]): Promise<DiscussResult[]> {
  return await invoke<DiscussResult[]>("discuss_concept", { ideaDescription, agents });
}

// ── 造化工坊执行（真实 LLM 调用） ──

export interface HarnessStepResult {
  stage_name: string;
  thinking: string;
  output: string;
}

export async function executeHarnessStep(stageName: string, projectContext: string, stagePrompt: string): Promise<HarnessStepResult> {
  return await invoke<HarnessStepResult>("execute_harness_step", { stageName, projectContext, stagePrompt });
}

// ── 插件/工作流 ──

export async function listPlugins(): Promise<any[]> {
  return await invoke<any[]>("list_plugins");
}

export async function installPlugin(yamlContent: string): Promise<void> {
  await invoke("install_plugin", { yamlContent });
}

export async function removePlugin(pluginId: string): Promise<void> {
  await invoke("remove_plugin", { pluginId });
}

export async function togglePlugin(pluginId: string, enabled: boolean): Promise<void> {
  await invoke("toggle_plugin", { pluginId, enabled });
}

// ── 专家库 ──

export async function saveExperts(experts: any[]): Promise<void> {
  await invoke("save_experts", { experts });
}

export async function loadExperts(): Promise<any[]> {
  return await invoke<any[]>("load_experts");
}

export async function scanNuwaSkills(): Promise<any[]> {
  return await invoke<any[]>("scan_nuwa_skills");
}

// ── 创作设定 ──

export async function saveSettings(settings: any): Promise<void> {
  await invoke("save_settings", { settings });
}

export async function loadSettings(): Promise<any> {
  return await invoke<any>("load_settings");
}

export async function saveConcept(concept: any): Promise<void> {
  await invoke("save_concept", { concept });
}

export async function loadConcept(): Promise<any> {
  return await invoke<any>("load_concept");
}

export async function saveSprout(sprout: any): Promise<void> {
  await invoke("save_sprout", { sprout });
}

export async function loadSprout(): Promise<any> {
  return await invoke<any>("load_sprout");
}

// ── HTTP 代理（绕过 WebView CSP） ──

export async function httpRequest(
  url: string,
  method: string,
  headers?: Record<string, string>,
  body?: string,
): Promise<{ status: number; statusText: string; body: string; ok: boolean }> {
  return await invoke<{ status: number; statusText: string; body: string; ok: boolean }>(
    "http_request",
    { request: { url, method, headers, body } },
  );
}
