// ipc.ts — HTTP API 调用层
// 前端调用后端 API 的唯一入口

import type {
  EntitySummary,
  GraphStats,
  ImpactPrediction,
  ConstraintReport,
  Character,
  Organization,
  Location,
  TimelineEvent,
  Foreshadow,
  OutlineArc,
  Chapter,
  ProjectOverview,
  CoreConcept,
  ChapterContent,
  LlmStatus,
  LlmTestResult,
  LlmConfigs,
  ProjectSummary,
  SproutSession,
  SproutProposal,
  ContextCheckResult,
  PullModelsResult,
  ModelDocResult,
} from "./types";

const API_BASE = "/api";

async function api<T>(path: string, params?: Record<string, string>, method?: string): Promise<T> {
  let url = `${API_BASE}${path}`;
  const options: RequestInit = { method: method || "GET" };

  if (params) {
    if (method === "GET" || method === "DELETE" || !method) {
      const qs = new URLSearchParams(params).toString();
      url += `?${qs}`;
    } else {
      // 对于 PUT/POST，发送 form-urlencoded body
      options.headers = { "Content-Type": "application/x-www-form-urlencoded" };
      options.body = new URLSearchParams(params).toString();
    }
  }

  const resp = await fetch(url, options);
  if (!resp.ok) {
    const text = await resp.text();
    // 后端统一返回 {"error":"可读原因"}；解析失败时回退原始文本/状态码
    let message = text || `HTTP ${resp.status}`;
    try {
      const parsed = JSON.parse(text) as { error?: unknown };
      if (typeof parsed.error === "string" && parsed.error) message = parsed.error;
    } catch {
      /* 保留原始响应文本 */
    }
    throw new Error(message);
  }
  const text = await resp.text();
  try {
    return JSON.parse(text) as T;
  } catch {
    return text as unknown as T;
  }
}

// ---- 项目管理 ----

export async function createProject(projectId: string, title: string): Promise<void> {
  await api("/projects/create", { project_id: projectId, title }, "POST");
}

export async function openProject(projectId: string): Promise<void> {
  await api("/projects/open", { project_id: projectId }, "POST");
}

export async function listProjects(): Promise<ProjectSummary[]> {
  return await api<ProjectSummary[]>("/projects");
}

export async function deleteProject(projectId: string): Promise<void> {
  await api("/projects/delete", { project_id: projectId }, "DELETE");
}

// ---- 灵魂萌芽（对话式创作工作台） ----

export async function getSproutSession(): Promise<SproutSession> {
  return await api<SproutSession>("/sprout/session");
}

export async function sproutStart(): Promise<{ content: string; model: string }> {
  return await api<{ content: string; model: string }>("/sprout/start", {}, "POST");
}

export async function sproutChat(
  message: string,
  perspective?: string,
): Promise<{ content: string; model: string }> {
  const params: Record<string, string> = { message };
  if (perspective) params.perspective = perspective;
  return await api<{ content: string; model: string }>("/sprout/chat", params, "POST");
}

export async function sproutGenerate(): Promise<SproutProposal> {
  return await api<SproutProposal>("/sprout/generate", {}, "POST");
}

export async function sproutApply(): Promise<void> {
  await api("/sprout/apply", {}, "POST");
}

export async function sproutDiscard(): Promise<void> {
  await api("/sprout/discard", {}, "POST");
}

export async function sproutClear(): Promise<void> {
  await api("/sprout/clear", {}, "POST");
}

// ---- 实体管理 ----

export async function addCharacter(name: string): Promise<string> {
  return await api<string>("/entities/character", { name }, "POST");
}

export async function addEvent(name: string, chapterId: number): Promise<string> {
  return await api<string>("/entities/event", { name, chapter_id: String(chapterId) }, "POST");
}

export async function addSetting(name: string, category: string): Promise<string> {
  return await api<string>("/entities/setting", { name, category }, "POST");
}

export async function listEntities(): Promise<EntitySummary[]> {
  return await api<EntitySummary[]>("/entities");
}

// ---- 实体更新/删除 ----

export async function updateCharacter(id: string, data: Partial<Character>): Promise<void> {
  const params: Record<string, string> = { id };
  if (data.name !== undefined) params.name = data.name;
  if (data.age !== undefined && data.age !== null) params.age = String(data.age);
  if (data.occupation !== undefined && data.occupation !== null) params.occupation = data.occupation;
  if (data.appearance !== undefined && data.appearance !== null) params.appearance = data.appearance;
  if (data.backstory !== undefined && data.backstory !== null) params.backstory = data.backstory;
  if (data.wants !== undefined && data.wants !== null) params.wants = data.wants;
  if (data.fears !== undefined && data.fears !== null) params.fears = data.fears;
  if (data.secret !== undefined && data.secret !== null) params.secret = data.secret;
  if (data.attire !== undefined && data.attire !== null) params.attire = data.attire;
  if (data.techniques !== undefined) params.techniques = data.techniques.join(",");
  if (data.realm !== undefined && data.realm !== null) params.realm = data.realm;
  if (data.items !== undefined) params.items = data.items.join(",");
  await api("/entities/character/update", params, "PUT");
}

export async function addOrganization(name: string, category: string): Promise<string> {
  return await api<string>("/entities/organization", { name, category }, "POST");
}

export async function updateOrganization(id: string, data: Partial<Organization>): Promise<void> {
  const params: Record<string, string> = { id };
  if (data.name !== undefined) params.name = data.name;
  if (data.category !== undefined) params.category = data.category;
  if (data.structure !== undefined) params.structure = data.structure;
  if (data.goals !== undefined) params.goals = data.goals;
  if (data.rules !== undefined) params.rules = data.rules.join(",");
  if (data.description !== undefined) params.description = data.description;
  await api("/entities/organization/update", params, "PUT");
}

export async function deleteOrganization(id: string): Promise<void> {
  await api("/entities/organization/delete", { id }, "DELETE");
}

export async function deleteCharacter(id: string): Promise<void> {
  await api("/entities/character/delete", { id }, "DELETE");
}

export async function updateEvent(id: string, data: Partial<TimelineEvent>): Promise<void> {
  const params: Record<string, string> = { id };
  if (data.name !== undefined) params.name = data.name;
  if (data.chapter_id !== undefined) params.chapter_id = String(data.chapter_id);
  if (data.description !== undefined) params.description = data.description;
  await api("/entities/event/update", params, "PUT");
}

export async function deleteEvent(id: string): Promise<void> {
  await api("/entities/event/delete", { id }, "DELETE");
}

export async function updateSetting(id: string, data: Partial<Location>): Promise<void> {
  const params: Record<string, string> = { id };
  if (data.name !== undefined) params.name = data.name;
  if (data.category !== undefined) params.category = data.category;
  if (data.description !== undefined) params.description = data.description;
  await api("/entities/setting/update", params, "PUT");
}

export async function deleteSetting(id: string): Promise<void> {
  await api("/entities/setting/delete", { id }, "DELETE");
}

// ---- 图谱查询 ----

export async function getGraphStats(): Promise<GraphStats> {
  return await api<GraphStats>("/graph/stats");
}

export async function predictImpact(entityId: string, entityType: string, maxDepth: number): Promise<ImpactPrediction[]> {
  return await api<ImpactPrediction[]>("/graph/predict", { entity_id: entityId, entity_type: entityType, max_depth: String(maxDepth) }, "POST");
}

export async function checkConstraints(): Promise<ConstraintReport> {
  return await api<ConstraintReport>("/constraints/check");
}

// ---- 仪表盘 ----

export async function getProjectOverview(): Promise<ProjectOverview> {
  return await api<ProjectOverview>("/dashboard/overview");
}

// ---- 世界管理 ----

export async function listCharacters(): Promise<Character[]> {
  return await api<Character[]>("/world/characters");
}

export async function listLocations(): Promise<Location[]> {
  return await api<Location[]>("/world/locations");
}

export async function listTimeline(): Promise<TimelineEvent[]> {
  return await api<TimelineEvent[]>("/world/timeline");
}

export async function listForeshadows(): Promise<Foreshadow[]> {
  return await api<Foreshadow[]>("/world/foreshadows");
}

export async function addForeshadow(name: string, plantedChapter: number): Promise<string> {
  return await api<string>("/world/foreshadows/add", { name, planted_chapter: String(plantedChapter) }, "POST");
}

export async function updateForeshadow(id: string, data: Partial<Foreshadow>): Promise<void> {
  const params: Record<string, string> = { id };
  if (data.name !== undefined) params.name = data.name;
  if (data.status !== undefined) params.status = data.status;
  if (data.description !== undefined) params.description = data.description;
  if (data.planted_chapter !== undefined && data.planted_chapter !== null) {
    params.planted_chapter = String(data.planted_chapter);
  }
  // 空值传空字符串：后端将回收章节清空
  if (data.expected_payoff !== undefined) {
    params.expected_payoff = data.expected_payoff == null ? "" : String(data.expected_payoff);
  }
  if (data.actual_payoff !== undefined) {
    params.actual_payoff = data.actual_payoff == null ? "" : String(data.actual_payoff);
  }
  await api("/world/foreshadows/update", params, "PUT");
}

export async function deleteForeshadow(id: string): Promise<void> {
  await api("/world/foreshadows/delete", { id }, "DELETE");
}

export async function listRules(): Promise<string[]> {
  return await api<string[]>("/world/rules");
}

export async function addRule(content: string): Promise<void> {
  await api("/world/rules/add", { content }, "PUT");
}

export async function updateRule(index: number, content: string): Promise<void> {
  await api("/world/rules/update", { index: String(index), content }, "PUT");
}

export async function deleteRule(index: number): Promise<void> {
  await api("/world/rules/delete", { index: String(index) }, "DELETE");
}

export async function getConcept(): Promise<CoreConcept> {
  return await api<CoreConcept>("/world/concept");
}

export async function updateConcept(data: Partial<CoreConcept>): Promise<void> {
  const params: Record<string, string> = {};
  if (data.high_concept !== undefined) params.high_concept = data.high_concept;
  if (data.premise !== undefined) params.premise = data.premise;
  if (data.protagonist_hint !== undefined) params.protagonist_hint = data.protagonist_hint;
  if (data.tone !== undefined) params.tone = data.tone;
  if (data.central_conflict !== undefined) params.central_conflict = data.central_conflict;
  if (data.inspiration !== undefined) params.inspiration = data.inspiration;
  await api("/world/concept/update", params, "PUT");
}

// ---- 大纲管理 ----

export async function listOutlineArcs(): Promise<OutlineArc[]> {
  return await api<OutlineArc[]>("/outline/arcs");
}

export async function createOutlineArc(title: string, chapterStart: number, chapterEnd: number): Promise<string> {
  return await api<string>("/outline/arcs/create", { title, chapter_start: String(chapterStart), chapter_end: String(chapterEnd) }, "POST");
}

export async function updateOutlineArc(arcId: string, data: Partial<OutlineArc>): Promise<void> {
  const params: Record<string, string> = { arc_id: arcId };
  if (data.title !== undefined) params.title = data.title;
  if (data.description !== undefined) params.description = data.description;
  if (data.chapter_start !== undefined) params.chapter_start = String(data.chapter_start);
  if (data.chapter_end !== undefined) params.chapter_end = String(data.chapter_end);
  await api("/outline/arcs/update", params, "PUT");
}

export async function deleteOutlineArc(arcId: string): Promise<void> {
  await api("/outline/arcs/delete", { arc_id: arcId }, "DELETE");
}

// ---- AI 辅助写作（建议制，保存走章节集成层） ----

export interface WritingMemoryStats {
  entity_count: number;
  total_tokens: number;
  budget_total: number;
  budget_entity: number;
  budget_temporal: number;
  budget_emotional: number;
}

export interface WritingResult {
  content: string;
  model: string;
  /** 动态记忆检索统计（F1） */
  memory_stats?: WritingMemoryStats;
  /** 注入的硬约束 id 列表（F3） */
  constraints_applied?: string[];
  /** 注入的叙事技巧 id 列表（F12/F15） */
  techniques_applied?: string[];
  /** 生成结果中检测到的高频 AI 味表达（F14，建议制） */
  anti_slop_warnings?: string[];
}

export async function generateWriting(
  chapterId: string,
  mode: "draft" | "continue",
  existingContent?: string,
  techniqueIds?: string[],
): Promise<WritingResult> {
  const params: Record<string, string> = { chapter_id: chapterId, mode };
  if (existingContent) params.existing_content = existingContent;
  if (techniqueIds && techniqueIds.length > 0) params.technique_ids = techniqueIds.join(",");
  return await api<WritingResult>("/writing/generate", params, "POST");
}

// ---- 叙事技巧库（F12/F15） ----

export interface Technique {
  id: string;
  name: string;
  category: string;
  description: string;
  guidance: string;
  check_items: string[];
}

export async function listTechniques(): Promise<Technique[]> {
  return await api<Technique[]>("/writing/techniques");
}

// ---- AI 章节审校（F3 完整版 / F4 / F8，建议制） ----

export interface ReviewLocalReport {
  char_count: number;
  meta_narration_hits: string[];
  anti_slop_hits: string[];
  tell_density: number;
  tell_counts: { word: string; count: number }[];
}

export interface ReviewResult {
  mode: "local" | "full";
  chapter_id: string;
  local: ReviewLocalReport;
  llm: {
    hard_constraint_issues: string[];
    entity_conflicts: string[];
    failure_modes: { dimension: string; severity: string; detail: string }[];
    suggestions: string[];
  } | null;
  techniques_checked: string[];
  /** P2-6：LLM 未配置/调用失败/解析失败时降级本地模式的显式原因 */
  llm_error?: string | null;
}

export async function reviewWriting(
  chapterId: string,
  content?: string,
  techniqueIds?: string[],
): Promise<ReviewResult> {
  const params: Record<string, string> = { chapter_id: chapterId };
  if (content) params.content = content;
  if (techniqueIds && techniqueIds.length > 0) params.technique_ids = techniqueIds.join(",");
  return await api<ReviewResult>("/writing/review", params, "POST");
}

// ---- 写作风格笔记（正典 AestheticLayer，F13） ----

export interface StyleNotes {
  style_notes: string;
  pacing_notes: string;
}

export async function getWorldStyle(): Promise<StyleNotes> {
  return await api<StyleNotes>("/world/style");
}

export async function updateWorldStyle(data: Partial<StyleNotes>): Promise<void> {
  const params: Record<string, string> = {};
  if (data.style_notes !== undefined) params.style_notes = data.style_notes;
  if (data.pacing_notes !== undefined) params.pacing_notes = data.pacing_notes;
  await api("/world/style", params, "PUT");
}

export async function listChapters(): Promise<Chapter[]> {
  return await api<Chapter[]>("/outline/chapters");
}

export async function createChapter(title: string): Promise<string> {
  return await api<string>("/outline/chapters/create", { title }, "POST");
}

export async function updateChapter(chapterId: string, data: Partial<Chapter>): Promise<void> {
  const params: Record<string, string> = { chapter_id: chapterId };
  if (data.title !== undefined) params.title = data.title;
  if (data.summary !== undefined) params.summary = data.summary;
  if (data.status !== undefined) params.status = data.status;
  await api("/outline/chapters/update", params, "PUT");
}

export async function deleteChapter(chapterId: string): Promise<void> {
  await api("/outline/chapters/delete", { chapter_id: chapterId }, "DELETE");
}

export async function saveChapterContent(chapterId: string, content: string): Promise<void> {
  await api("/outline/chapters/content", { chapter_id: chapterId, content }, "PUT");
}

export async function getChapterContent(chapterId: string): Promise<ChapterContent> {
  return await api<ChapterContent>("/outline/chapters/content", { chapter_id: chapterId });
}

// ---- 全局 LLM 配置管理 ----

export async function listLlmConfigs(): Promise<LlmConfigs> {
  return await api<LlmConfigs>("/llm/configs");
}

export async function createLlmConfig(data: Record<string, string>): Promise<string> {
  return await api<string>("/llm/configs", data, "POST");
}

export async function updateLlmConfig(data: Record<string, string>): Promise<string> {
  return await api<string>("/llm/configs", data, "PUT");
}

export async function deleteLlmConfig(configId: string): Promise<void> {
  await api("/llm/configs", { config_id: configId }, "DELETE");
}

export async function setDefaultLlmConfig(configId: string): Promise<void> {
  await api("/llm/default", { config_id: configId }, "POST");
}

export async function getLlmStatus(): Promise<LlmStatus> {
  return await api<LlmStatus>("/llm/status");
}

export async function testLlm(configId: string, prompt: string): Promise<LlmTestResult> {
  return await api<LlmTestResult>("/llm/test", { config_id: configId, prompt }, "POST");
}

export async function contextCheck(
  configId: string | null,
  modelId: string | null,
  text: string
): Promise<ContextCheckResult> {
  const params: Record<string, string> = { text };
  if (configId) params.config_id = configId;
  if (modelId) params.model_id = modelId;
  return await api<ContextCheckResult>("/llm/context-check", params, "POST");
}

export async function pullLlmModels(configId: string): Promise<PullModelsResult> {
  return await api<PullModelsResult>("/llm/models/pull", { config_id: configId }, "POST");
}

export async function fetchModelDoc(
  configId: string,
  modelId: string,
  docUrl?: string
): Promise<ModelDocResult> {
  const params: Record<string, string> = { config_id: configId, model_id: modelId };
  if (docUrl) params.doc_url = docUrl;
  return await api<ModelDocResult>("/llm/docs/model", params, "POST");
}

// ---- Agent 注册表：按角色选模型（P0b） ----

export interface AgentConfigEntry {
  role_id: string;
  display_name: string;
  llm_config_id: string | null;
  bound_model?: { name: string; model_id: string } | null;
  project_overrides: Record<string, string>;
}

export interface AgentConfigs {
  agents: AgentConfigEntry[];
  config_file: string;
  note: string;
}

export async function listAgentConfigs(): Promise<AgentConfigs> {
  return await api<AgentConfigs>("/agent/configs");
}

export async function updateAgentConfig(
  roleId: string,
  llmConfigId: string | null,
): Promise<void> {
  const params: Record<string, string> = { role_id: roleId };
  if (llmConfigId) params.llm_config_id = llmConfigId;
  await api("/agent/configs", params, "PUT");
}

export async function listOrganizations(): Promise<Organization[]> {
  return await api<Organization[]>("/entities/organizations");
}

// ---- 事实提取（P1，全自动） ----

export interface ExtractReport {
  applied: string[];
  skipped: string[];
  warnings: string[];
  low_confidence: string[];
}

export async function extractFacts(chapterId: string): Promise<ExtractReport> {
  return await api<ExtractReport>("/writing/extract-facts", { chapter_id: chapterId }, "POST");
}

// ---- 批注 + AI 审核改写（P2） ----

export interface ChapterAnnotation {
  annotation_id: string;
  kind: string;
  content: string;
  status: string;
  created_at: string;
}

export interface RewriteChange {
  what: string;
  why: string;
}

export interface DiffEntry {
  kind: "equal" | "modified" | "added" | "removed";
  text: string;
}

export interface RewriteResult {
  mode: string;
  chapter_id: string;
  rewritten: string;
  changes: RewriteChange[];
  diff: DiffEntry[];
  de_slop_hits: string[];
  model: string;
}

export async function addAnnotation(
  chapterId: string,
  kind: string,
  content: string,
): Promise<string> {
  const params: Record<string, string> = { chapter_id: chapterId, kind, content };
  return await api<string>("/writing/annotations", params, "POST");
}

export async function updateAnnotationStatus(
  chapterId: string,
  annotationId: string,
  status: string,
): Promise<void> {
  const params: Record<string, string> = {
    chapter_id: chapterId,
    annotation_id: annotationId,
    status,
  };
  await api("/writing/annotations/status", params, "PUT");
}

export async function deleteAnnotation(
  chapterId: string,
  annotationId: string,
): Promise<void> {
  await api("/writing/annotations/delete", { chapter_id: chapterId, annotation_id: annotationId }, "DELETE");
}

export async function aiRewrite(
  chapterId: string,
  content: string,
  instructions?: string,
  mode: "audit" | "de-slop" = "audit",
): Promise<RewriteResult> {
  const params: Record<string, string> = { chapter_id: chapterId, content, mode };
  if (instructions) params.instructions = instructions;
  return await api<RewriteResult>("/writing/rewrite", params, "POST");
}

// ---- 书籍蒸馏（P3：语料摄取 → 风格配方） ----

export interface BookSource {
  id: string;
  title: string;
  format: string;
  chars: number;
  weight: number;
}

export interface DimFeature {
  dimension: string;
  features: string[];
}

export interface StyleRecipe {
  books: BookSource[];
  strength: number;
  dimensions: DimFeature[];
  genes: string[];
  bans: string[];
  generated_at: string;
  model: string;
}

export interface DistillAnalyzeResult {
  ok: boolean;
  books: string[];
  dimension_count: number;
  gene_count: number;
  ban_count: number;
  strength: number;
  model: string;
  /** P2-5：正文缺失被跳过的语料（读盘失败显式列出，不再静默当空书） */
  missing_books?: string[];
}

export async function addDistillCorpus(
  title: string,
  format: string,
  contentB64: string,
  weight?: number,
): Promise<{ id: string; chars: number }> {
  const params: Record<string, string> = { title, format, content_b64: contentB64 };
  if (weight !== undefined) params.weight = String(weight);
  return await api<{ id: string; chars: number }>("/distill/corpus", params, "POST");
}

export async function listDistillCorpus(): Promise<BookSource[]> {
  return await api<BookSource[]>("/distill/corpus/list");
}

export async function deleteDistillCorpus(id: string): Promise<void> {
  await api("/distill/corpus/delete", { id }, "DELETE");
}

export async function analyzeDistill(weights?: Record<string, number>): Promise<DistillAnalyzeResult> {
  const params: Record<string, string> = {};
  if (weights && Object.keys(weights).length > 0) {
    params.weights = Object.entries(weights).map(([k, v]) => `${k}=${v}`).join(",");
  }
  return await api<DistillAnalyzeResult>("/distill/analyze", params, "POST");
}

export async function getStyleRecipe(): Promise<StyleRecipe> {
  return await api<StyleRecipe>("/distill/recipe");
}

export async function updateStyleRecipe(strength: number): Promise<void> {
  await api("/distill/recipe/update", { strength: String(strength) }, "PUT");
}

export async function deleteStyleRecipe(): Promise<void> {
  await api("/distill/recipe/delete", undefined, "DELETE");
}

// ---- 级联同步（P4） ----

export interface ChangedFact {
  entity: string;
  attribute: string;
  old_value: string;
  new_value: string;
}

export interface AffectedChapter {
  chapter_id: string;
  chapter_no: number;
  title: string;
  matched_entities: string[];
  snippet: string;
}

export interface CascadeAnalyzeResult {
  ok: boolean;
  changed_facts: ChangedFact[];
  affected: AffectedChapter[];
  limit: number;
  note: string;
}

export interface CascadeApplyItem {
  chapter_id: string;
  chapter_no: number;
  title: string;
  rewritten: string;
}

export interface CascadeApplyResult {
  ok: boolean;
  results: CascadeApplyItem[];
  /** P2-1：级联审计日志写失败时的显式警告 */
  log_warning?: string | null;
}

export async function cascadeAnalyze(
  chapterId: string,
  original: string,
  rewritten: string,
): Promise<CascadeAnalyzeResult> {
  const params: Record<string, string> = { chapter_id: chapterId, original, rewritten };
  return await api<CascadeAnalyzeResult>("/writing/cascade/analyze", params, "POST");
}

export async function cascadeApply(
  chapterId: string,
  targetChapterIds: string[],
  changedFacts: ChangedFact[],
): Promise<CascadeApplyResult> {
  const params: Record<string, string> = {
    chapter_id: chapterId,
    target_chapter_ids: targetChapterIds.join(","),
    changed_facts: JSON.stringify(changedFacts),
  };
  return await api<CascadeApplyResult>("/writing/cascade/apply", params, "POST");
}

// ---- 细纲化与批量写作（P5） ----

export interface DetailItem {
  chapter_no: number;
  title: string;
  summary: string;
}

export interface DetailGenerateResult {
  ok: boolean;
  items: DetailItem[];
  count: number;
  model: string;
  note: string;
  calls?: number;
  missing_chapters?: number[];
}

export interface DetailImportResult {
  ok: boolean;
  created: number;
  updated: number;
  note: string;
}

export interface BatchWriteItem {
  chapter_id: string;
  chapter_no: number;
  title: string;
  content: string;
  model: string;
  anti_slop_warnings: string[];
}

export interface BatchWriteResult {
  ok: boolean;
  results: BatchWriteItem[];
  batch_size: number;
  note: string;
}

export async function detailGenerate(arcId?: string): Promise<DetailGenerateResult> {
  const params: Record<string, string> = {};
  if (arcId) params.arc_id = arcId;
  return await api<DetailGenerateResult>("/outline/detail/generate", params, "POST");
}

export async function detailImport(items: DetailItem[]): Promise<DetailImportResult> {
  return await api<DetailImportResult>(
    "/outline/detail/import",
    { detail_json: JSON.stringify(items) },
    "POST",
  );
}

export async function batchWrite(
  chapterIds: string[],
  techniqueIds?: string[],
): Promise<BatchWriteResult> {
  const params: Record<string, string> = { chapter_ids: chapterIds.join(",") };
  if (techniqueIds && techniqueIds.length > 0) params.technique_ids = techniqueIds.join(",");
  return await api<BatchWriteResult>("/writing/batch", params, "POST");
}

// ---- 归档压缩 + 操作日志 / 回滚 / 成本（P6） ----

export interface OperationLogEntry {
  time: string;
  chapter_id: string;
  applied: string[];
  skipped: string[];
  warnings: string[];
  facts?: Array<{ kind: string; name: string; attribute?: string | null; old_value?: string | null; new_value?: string | null }>;
}

export interface OperationLogList {
  total: number;
  returned: number;
  entries: OperationLogEntry[];
}

export interface RollbackResult {
  ok: boolean;
  rolled_back: number;
  remaining: number;
  undone: string[];
  /** P2-1：审计日志截断写失败时的显式警告（提醒勿重复回滚） */
  log_warning?: string | null;
}

export interface CompressResult {
  ok: boolean;
  archived: number;
  keep_recent: number;
  note: string;
}

export interface CostReport {
  operation_count: number;
  fact_extract_count: number;
  cascade_count: number;
  distilled_books: number;
  tier: string;
  agent_bindings: Array<{ role: string; display_name: string; llm_config_id: string | null }>;
  note: string;
}

export async function listOperations(limit?: number): Promise<OperationLogList> {
  const params: Record<string, string> = {};
  if (limit !== undefined) params.limit = String(limit);
  return await api<OperationLogList>("/log/operations", params);
}

export async function rollbackOperations(lastN: number): Promise<RollbackResult> {
  return await api<RollbackResult>("/log/rollback", { last_n: String(lastN) }, "POST");
}

export async function compressArchive(keepRecent: number): Promise<CompressResult> {
  return await api<CompressResult>("/archive/compress", { keep_recent: String(keepRecent) }, "POST");
}

export async function listArchive(): Promise<{ generated_at?: string; keep_recent?: number; volumes: Array<{ chapter_no: number; title: string; summary: string; word_count: number }> }> {
  return await api("/archive/list");
}

export async function getCostReport(): Promise<CostReport> {
  return await api<CostReport>("/log/cost");
}
