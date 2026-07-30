import { invoke } from "@tauri-apps/api/core";
import type { DiscussionOutput } from "./types";

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

// 新建或更新章节（含标题/卷归属/梗概），新建章节必须走这里才能落盘
export async function upsertChapter(
  chapterId: string,
  volumeId: string,
  title: string,
  content: string,
  summary: string,
  status: string
): Promise<void> {
  await invoke("upsert_chapter", { chapterId, volumeId, title, content, summary, status });
}

// 持久化卷列表（卷名等元数据）
export async function saveVolumes(volumes: Array<{ volume_id: string; title: string; summary?: string }>): Promise<void> {
  await invoke("save_volumes", { volumes });
}

export async function getVolumes(): Promise<any[]> {
  return await invoke<any[]>("get_volumes");
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

export async function loadApiKeys(): Promise<Record<string, string>> {
  return await invoke<Record<string, string>>("load_api_keys");
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

// ── 概念讨论（真实 LLM 调用，两轮交锋 + 结构化成果） ──

export interface DiscussAgent {
  id: string;
  name: string;
  model: string;
  prompt: string;
  perspective: string;
  enabled: boolean;
  skill_path?: string | null;
}

export async function discussConcept(ideaDescription: string, settingsContext: string, agents: DiscussAgent[]): Promise<DiscussionOutput> {
  return await invoke<DiscussionOutput>("discuss_concept", { ideaDescription, settingsContext, agents });
}

// ── 页面内容优化（世界观/人物志） ──

export async function optimizeContent(contentType: string, contentJson: string, modelId: string | null): Promise<string> {
  return await invoke<string>("optimize_content", { contentType, contentJson, modelId });
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
export async function scanExpertsFolder(path: string): Promise<any[]> {
  return await invoke<any[]>("scan_experts_folder", { path });
}

export async function deleteExpertSkill(skillPath: string): Promise<void> {
  await invoke("delete_expert_skill", { skillPath });
}

export async function getExpertsFolder(): Promise<string> {
  return await invoke<string>("get_experts_folder");
}

// 女娲蒸馏：调用 LLM 对名人进行蒸馏，返回专家结果
// 通过 Tauri 事件 "distill-phase" 接收实时进度
export async function distillExpert(persona: string): Promise<any> {
  return await invoke<any>("distill_expert", { persona });
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
