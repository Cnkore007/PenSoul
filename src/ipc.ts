import { invoke } from "@tauri-apps/api/core";
import type {
  ChapterAnnotation,
  ChapterRevision,
  DiscussionOutput,
  PipelineState,
  WritingLesson,
} from "./types";

// ── 全链路批注 ──

export async function annotationAdd(
  target: string,
  kind: ChapterAnnotation["kind"],
  content: string,
  anchor?: ChapterAnnotation["anchor"] | null
): Promise<ChapterAnnotation> {
  return await invoke<ChapterAnnotation>("annotation_add", { target, kind, content, anchor });
}

export async function annotationUpdate(
  target: string,
  annotationId: string,
  patch: { kind?: string; content?: string; status?: string }
): Promise<void> {
  await invoke("annotation_update", { target, annotationId, patch });
}

export async function annotationRemove(target: string, annotationId: string): Promise<void> {
  await invoke("annotation_remove", { target, annotationId });
}

export async function annotationResolve(
  target: string,
  decisions: Array<{ annotation_id: string; accept: boolean }>
): Promise<ChapterAnnotation[]> {
  return await invoke<ChapterAnnotation[]>("annotation_resolve", { target, decisions });
}

export async function annotationsList(target: string): Promise<ChapterAnnotation[]> {
  return await invoke<ChapterAnnotation[]>("annotations_list", { target });
}

export async function annotationsAll(): Promise<
  Array<{ target: string; label: string; annotations: ChapterAnnotation[] }>
> {
  return await invoke<Array<{ target: string; label: string; annotations: ChapterAnnotation[] }>>(
    "annotations_all"
  );
}

export async function annotationsExport(): Promise<string> {
  return await invoke<string>("annotations_export");
}

// ── 编辑经验沉淀（修改 → 样本 → WritingLesson） ──

export interface EditSample {
  sample_id: string;
  scope: "chapter" | "outline" | "world" | "character" | string;
  label: string;
  before: string;
  after: string;
  created_at?: string;
}

export async function getPendingEdits(): Promise<EditSample[]> {
  return await invoke<EditSample[]>("get_pending_edits");
}

export async function distillPendingLessons(): Promise<WritingLesson[]> {
  return await invoke<WritingLesson[]>("distill_pending_lessons");
}

// ── 页面受控保存（保存并审核） ──

export interface ReviewItem {
  source: "annotation" | "edit";
  id: string;
  label: string;
  content: string;
  verdict: "valid" | "invalid" | "uncertain" | string;
  reason: string;
}

export interface PageReview {
  items: ReviewItem[];
  impact: string;
}

export async function reviewPageChanges(
  page: "world" | "character",
  contentJson: string
): Promise<PageReview> {
  return await invoke<PageReview>("review_page_changes", { page, contentJson });
}

export async function applyPageReview(
  page: "world" | "character",
  contentJson: string,
  confirmations: Array<{ id: string; verdict: string }>
): Promise<{ applied: boolean; page: string; lessons: WritingLesson[]; can_undo: boolean }> {
  return await invoke("apply_page_review", { page, contentJson, confirmations });
}

export async function undoPageChange(page: "world" | "character"): Promise<unknown> {
  return await invoke("undo_page_change", { page });
}

export async function pageUndoAvailable(page: "world" | "character"): Promise<boolean> {
  return await invoke<boolean>("page_undo_available", { page });
}

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

export async function saveChapter(
  chapterId: string,
  content: string,
  expectedVersion: number,
  annotations?: ChapterAnnotation[] | null
): Promise<number> {
  return await invoke<number>("save_chapter", { chapterId, content, expectedVersion, annotations });
}

// 按批注重写本章：修改计划 → 重写正文 → 沉淀写作经验（新版本，旧版进历史可回滚）
export async function rewriteChapterWithAnnotations(
  chapterId: string,
  modelId?: string | null,
  skillCards?: string[] | null
): Promise<{
  new_version: number;
  accepted: string[];
  rejected: string[];
  untouched: string[];
  plan_summary: string;
  lessons: WritingLesson[];
}> {
  return await invoke("rewrite_chapter_with_annotations", {
    chapterId,
    model: modelId,
    skillCards,
  });
}

// 章节版本历史（批注重写前快照 / 回滚点）
export async function listChapterRevisions(chapterId: string): Promise<ChapterRevision[]> {
  return await invoke<ChapterRevision[]>("list_chapter_revisions", { chapterId });
}

// 回滚到指定版本（当前版进历史）
export async function rollbackChapter(chapterId: string, targetVersion: number): Promise<number> {
  return await invoke<number>("rollback_chapter", { chapterId, targetVersion });
}

// 项目写作经验库
export async function getWritingLessons(): Promise<WritingLesson[]> {
  return await invoke<WritingLesson[]>("get_writing_lessons");
}

export async function saveWritingLessons(lessons: WritingLesson[]): Promise<void> {
  await invoke("save_writing_lessons", { lessons });
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

// ── 情节脉络（大纲规划层） ──

export async function listOutlineArcs(): Promise<import("./types").OutlineArc[]> {
  return await invoke<import("./types").OutlineArc[]>("list_outline_arcs");
}

export async function saveOutlineArcs(arcs: import("./types").OutlineArc[]): Promise<void> {
  await invoke("save_outline_arcs", { arcs });
}

// 展开脉络节点的下一批细纲（默认每批 20 章），返回生成范围与完成状态
// skillCards：工作流为细纲展开环节绑定的技法卡 SKILL.md 路径（可空）
export async function expandOutlineArc(
  arcId: string,
  modelId: string | null,
  batch?: number,
  skillCards?: string[] | null
): Promise<{ created: number; from: number; to: number; arc_done: boolean }> {
  return await invoke("expand_outline_arc", {
    arcId,
    model: modelId,
    batch: batch ?? null,
    skillCards: skillCards ?? null,
  });
}

export async function deleteChapter(chapterId: string): Promise<void> {
  await invoke("delete_chapter", { chapterId });
}

export async function deleteVolume(volumeId: string): Promise<void> {
  await invoke("delete_volume", { volumeId });
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

// 章节修改后的影响分析：受影响章节 + 本章相关一致性违规（笔耕保存后展示）
export async function analyzeChapterImpact(chapterId: string): Promise<import("./types").ChapterImpact> {
  return await invoke<import("./types").ChapterImpact>("analyze_chapter_impact", { chapterId });
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

// 讨论状态查询（运行旗标 + 事件缓冲），切换页面后重连恢复进度用
export async function getDiscussionState(): Promise<import("./types").DiscussionState> {
  return await invoke<import("./types").DiscussionState>("get_discussion_state");
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

// ── 连写管线（造化工坊，引擎驱动的自动连写） ──
// 进度通过 Tauri 事件 "harness-event" 实时推送

export async function runChapterPipeline(
  chapterIds: string[] | null,
  writingModel: string | null,
  reviewModel: string | null,
  writingCards?: string[] | null,
  reviewCards?: string[] | null
): Promise<{ completed: number; failed: string[]; stopped: boolean; total: number }> {
  return await invoke("run_chapter_pipeline", {
    chapterIds,
    writingModel,
    reviewModel,
    writingCards: writingCards ?? null,
    reviewCards: reviewCards ?? null,
  });
}

export async function pausePipeline(): Promise<void> {
  await invoke("pause_pipeline");
}

export async function resumePipeline(): Promise<void> {
  await invoke("resume_pipeline");
}

export async function stopPipeline(): Promise<void> {
  await invoke("stop_pipeline");
}

export async function getPipelineState(): Promise<PipelineState> {
  return await invoke<PipelineState>("get_pipeline_state");
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

// 专家蒸馏：调用 LLM 对名人进行蒸馏，返回专家结果
// 通过 Tauri 事件 "distill-phase" 接收实时进度；model 为空时后端自动选模型
export async function distillExpert(persona: string, model?: string | null): Promise<any> {
  return await invoke<any>("distill_expert", { persona, model: model ?? null });
}

// ── 书籍蒸馏 · 写作技能卡 ──
// 进度通过 Tauri 事件 "book-distill-phase" 实时推送

// 蒸馏一本书为写作技能卡组（dimensions 为空 = 全 5 维；filePath 上传书籍文件优先，sampleText 手动样章次之）
export async function distillBook(
  title: string,
  author: string | null,
  filePath: string | null,
  sampleText: string | null,
  dimensions: string[] | null,
  model: string | null
): Promise<import("./types").BookPackage> {
  return await invoke<import("./types").BookPackage>("distill_book", {
    title,
    author,
    filePath,
    sampleText,
    dimensions,
    model,
  });
}

// 列出 WritingCard/ 下全部技能包
export async function listBookPackages(): Promise<import("./types").BookPackage[]> {
  return await invoke<import("./types").BookPackage[]>("list_book_packages");
}

// 删除整个技能包目录（不可逆）
export async function deleteBookPackage(packageDir: string): Promise<void> {
  return await invoke("delete_book_package", { package: packageDir });
}

// 蒸馏一段方法论为写作技能卡组（dimensions 为空 = 全 6 维）
export async function distillMethodology(
  title: string,
  methodologyText: string,
  dimensions: string[] | null,
  model: string | null
): Promise<import("./types").BookPackage> {
  return await invoke<import("./types").BookPackage>("distill_methodology", {
    title,
    methodologyText,
    dimensions,
    model,
  });
}

// 反 AI 味检测：对章节正文按五类模式统计，返回 0-100 的 AI 痕迹报告
export async function analyzeAiFlavor(content: string): Promise<import("./types").AiFlavorReport> {
  return await invoke<import("./types").AiFlavorReport>("analyze_ai_flavor", { content });
}

// ── 工作流技能配置（环节 → 模型 + 技法卡绑定，随项目持久化） ──

export async function saveWorkflowSkills(config: import("./types").WorkflowSkillConfig | null): Promise<void> {
  await invoke("save_workflow_skills", { config });
}

export async function loadWorkflowSkills(): Promise<import("./types").WorkflowSkillConfig | null> {
  return await invoke<import("./types").WorkflowSkillConfig | null>("load_workflow_skills");
}

// ── 全局工作流模板（作品库层面，data/workflows/templates.json） ──

// 列出全部模板（后端每次重新加载磁盘，跨页面/进程一致）
export async function listWorkflowTemplates(): Promise<import("./types").WorkflowTemplate[]> {
  return await invoke<import("./types").WorkflowTemplate[]>("list_workflow_templates");
}

// 整体保存模板列表（内置模板保护：缺失自动补回、builtin 标志不可篡改）
export async function saveWorkflowTemplates(templates: import("./types").WorkflowTemplate[]): Promise<void> {
  await invoke("save_workflow_templates", { templates });
}

// 恢复内置模板（用户自定义模板保留）
export async function resetWorkflowTemplates(): Promise<import("./types").WorkflowTemplate[]> {
  return await invoke<import("./types").WorkflowTemplate[]>("reset_workflow_templates");
}

// 一键清空所有项目的项目级覆盖（覆盖层退役，项目只保留模板引用）
export async function clearAllProjectOverrides(): Promise<number> {
  return await invoke<number>("clear_all_project_overrides");
}

// ── 项目工作流引用（模板 ID + 版本 + 项目覆盖，随项目文件持久化） ──

export async function saveWorkflowRef(config: import("./types").WorkflowRef | null): Promise<void> {
  await invoke("save_workflow_ref", { config: config ?? null });
}

export async function loadWorkflowRef(): Promise<import("./types").WorkflowRef | null> {
  return await invoke<import("./types").WorkflowRef | null>("load_workflow_ref");
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

// ── 蒸馏状态（书籍/方法论/专家共用，页面切换后重连恢复进度） ──

export interface DistillState {
  running: boolean;
  kind: string | null;
  events: Array<{ phase: string; status: string; message: string; detail: string }>;
}

export async function getDistillState(): Promise<DistillState> {
  return await invoke<DistillState>("get_distill_state");
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
