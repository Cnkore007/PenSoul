import { useState, useEffect, useCallback, useMemo } from "react";
import {
  Workflow,
  Plus,
  Trash2,
  Save,
  RotateCcw,
  Pencil,
  Check,
  X,
  Zap,
  AlertCircle,
  Clock,
  ArrowRight,
  BookOpen,
  Sparkles,
  Layers,
  Eraser,
} from "lucide-react";
import type {
  WorkflowTemplate,
  WorkflowStageDef,
  BookPackage,
  BookCardInfo,
  LlmModel,
  StageSkillConfig,
} from "../types";
import {
  listWorkflowTemplates,
  saveWorkflowTemplates,
  resetWorkflowTemplates,
  listBookPackages,
  deleteBookPackage,
  listModels,
  clearAllProjectOverrides,
  getDistillState,
} from "../ipc";
import { EXEC_STAGES, emptyStageConfig } from "../workflow";
import { BookDistillPanel } from "../components/BookDistillPanel";
import { MethodologyDistillPanel } from "../components/MethodologyDistillPanel";
import { confirmDialog } from "../dialogs";

const gateIcons: Record<string, React.ReactNode> = {
  auto: <Zap size={13} />,
  manual: <AlertCircle size={13} />,
  conditional: <Clock size={13} />,
};
const gateLabels: Record<string, string> = {
  auto: "自动",
  manual: "人工",
  conditional: "条件",
};

// 三个可绑定技能卡/模型的执行环节（key 与后端 applicable_stages 一致）
const STAGE_LABELS: Record<string, string> = {
  outline_expand: "细纲展开",
  chapter_writing: "章节写作",
  review: "一致性审查",
};
// 阶段 key 的中文显示名（模板编辑区展示用，key 本身是后端契约不可改）
const STAGE_KEY_LABELS: Record<string, string> = {
  chapter_planning: "章前策划",
  chapter_writing: "章节写作",
  chapter_review: "卖点与质量审查",
  state_injection: "状态回灌",
};
// 每环节可绑的维度（与后端 DIMENSIONS 的适用环节一致）
const STAGE_DIMS: Record<string, { slug: string; label: string }[]> = {
  outline_expand: [
    { slug: "structure", label: "结构卡" },
    { slug: "character", label: "人物卡" },
    { slug: "tension", label: "张力卡" },
    { slug: "genre", label: "类型卡" },
  ],
  chapter_writing: [
    { slug: "style", label: "文风卡" },
    { slug: "character", label: "人物卡" },
    { slug: "tension", label: "张力卡" },
  ],
  review: [{ slug: "style", label: "文风卡" }],
};

type IndexedCard = BookCardInfo & { pkgTitle: string };

// 新模板默认四阶段（章前策划 → 写作 → 审查 → 回灌，与管线编排 key 一致）
function defaultStages(): WorkflowStageDef[] {
  return [
    {
      stage: "chapter_planning",
      display_name: "章前策划",
      prompt_hint: "写前策划：产出本章节拍表（章节目标、开场钩子、3-6 个场景、爽点、断章钩子、伏笔埋收）。",
      gate: "auto",
      on_fail: null,
      max_retries: 2,
      enabled: true,
    },
    {
      stage: "chapter_writing",
      display_name: "章节写作",
      prompt_hint: "严格按节拍表撰写正文：场景之间用细节自然钩连，叙述、对话与动作按叙事需要交织，允许长句铺陈，结尾断章钩子，遵守反 AI 味铁律。",
      gate: "auto",
      on_fail: null,
      max_retries: 2,
      enabled: true,
    },
    {
      stage: "chapter_review",
      display_name: "卖点与质量审查",
      prompt_hint: "按七维加权审查本章（卖点/钩子/情绪/节奏/断章/一致性/文笔），输出分数与问题清单。",
      gate: "conditional",
      on_fail: "chapter_writing",
      max_retries: 2,
      enabled: true,
    },
    {
      stage: "state_injection",
      display_name: "状态回灌",
      prompt_hint: "提炼本章纪要，回灌滚动备忘录。",
      gate: "auto",
      on_fail: null,
      max_retries: 1,
      enabled: true,
    },
  ];
}

function newCustomTemplate(seq: number): WorkflowTemplate {
  return {
    template_id: `custom-${Date.now()}-${seq}`,
    name: "自定义工作流",
    version: "1.0",
    genre: "通用",
    description: "自定义创作流，可自行调整阶段手册、门控与重试策略。",
    builtin: false,
    enabled: true,
    review_pass_score: 80,
    stages: defaultStages(),
    bindings: {},
  };
}

export function WorkflowLibraryView() {
  const [templates, setTemplates] = useState<WorkflowTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");
  // 行内编辑：一次只编辑一个模板，草稿独立保存，点「保存模板」才落盘
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<WorkflowTemplate | null>(null);
  // 展开查看的阶段手册
  const [expandedStage, setExpandedStage] = useState<string | null>(null);
  // 展开编辑「环节技能绑定」的模板
  const [expandedBindings, setExpandedBindings] = useState<string | null>(null);
  // 写作技能库
  const [packages, setPackages] = useState<BookPackage[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [showDistill, setShowDistill] = useState(false);
  const [showMethodologyDistill, setShowMethodologyDistill] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [ts, pkgs, ms] = await Promise.all([
        listWorkflowTemplates(),
        listBookPackages().catch(() => []),
        listModels().catch(() => []),
      ]);
      setTemplates(ts);
      setPackages(pkgs);
      setModels((ms || []).filter((m: any) => m.is_available !== false));
    } catch (e: any) {
      setError("加载工作流数据失败: " + (e?.message ?? e));
    } finally {
      setLoading(false);
    }
  }, []);

  // 页面切换后重连：若蒸馏仍在后台进行，自动打开对应面板（面板内部负责恢复进度）
  useEffect(() => {
    getDistillState().then(st => {
      if (!st.running) return;
      if (st.kind === "book") setShowDistill(true);
      else if (st.kind === "methodology") setShowMethodologyDistill(true);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // skill_path → 卡信息索引（含所属包名）
  const cardIndex = useMemo(() => {
    const map = new Map<string, IndexedCard>();
    for (const p of packages) for (const c of p.cards) map.set(c.skill_path, { ...c, pkgTitle: p.title });
    return map;
  }, [packages]);

  // 某环节某维度的候选卡（按卡自身声明的适用环节过滤）
  function candidates(stage: string, dim: string): IndexedCard[] {
    const out: IndexedCard[] = [];
    for (const p of packages)
      for (const c of p.cards)
        if (c.dimension === dim && c.applicable_stages.includes(stage)) out.push({ ...c, pkgTitle: p.title });
    return out;
  }

  function flash(text: string) {
    setMsg(text);
    setError("");
    setTimeout(() => setMsg(""), 3000);
  }

  // ── 保存 ──

  async function handleSaveAll() {
    try {
      await saveWorkflowTemplates(templates);
      setEditingId(null);
      setDraft(null);
      await load();
      flash("模板库已保存（含环节绑定）");
    } catch (e: any) {
      setError("保存失败: " + (e?.message ?? e));
    }
  }

  async function handleResetBuiltins() {
    if (!(await confirmDialog("恢复内置模板到出厂状态？自定义模板会保留，内置模板上手动改过的内容将丢失。"))) return;
    try {
      setTemplates(await resetWorkflowTemplates());
      setEditingId(null);
      setDraft(null);
      flash("内置模板已恢复");
    } catch (e: any) {
      setError("恢复失败: " + (e?.message ?? e));
    }
  }

  // 一键清空所有项目的项目级覆盖（覆盖层退役，统一由模板绑定接管）
  async function handleClearOverrides() {
    if (!(await confirmDialog("一键清空所有项目的项目级覆盖？项目将只保留模板引用，各环节绑定统一由全局模板接管。该操作不可恢复。"))) return;
    try {
      const n = await clearAllProjectOverrides();
      flash(`已清空 ${n} 个项目的项目级覆盖`);
    } catch (e: any) {
      setError("清空失败: " + (e?.message ?? e));
    }
  }

  // ── 模板级操作 ──

  function toggleEnabled(t: WorkflowTemplate) {
    setTemplates(prev => prev.map(x => x.template_id === t.template_id ? { ...x, enabled: !x.enabled } : x));
  }

  function addTemplate() {
    const t = newCustomTemplate(templates.length + 1);
    setTemplates(prev => [...prev, t]);
    setEditingId(t.template_id);
    setDraft(JSON.parse(JSON.stringify(t)));
  }

  function startEdit(t: WorkflowTemplate) {
    setEditingId(t.template_id);
    setDraft(JSON.parse(JSON.stringify(t)));
  }

  function cancelEdit() {
    // 未保存的新模板直接移除
    if (draft && !templates.some(t => t.template_id === draft.template_id)) {
      setTemplates(prev => prev.filter(t => t.template_id !== draft!.template_id));
    }
    setEditingId(null);
    setDraft(null);
  }

  async function deleteTemplate(t: WorkflowTemplate) {
    if (!(await confirmDialog(`删除模板「${t.name}」？项目若引用了它将无法解析，请先在其他项目里换模板。`))) return;
    const next = templates.filter(x => x.template_id !== t.template_id);
    setTemplates(next);
    if (editingId === t.template_id) {
      setEditingId(null);
      setDraft(null);
    }
    try {
      await saveWorkflowTemplates(next);
      flash(`已删除「${t.name}」`);
    } catch (e: any) {
      setError("删除失败: " + (e?.message ?? e));
      await load();
    }
  }

  // 删除按钮统一入口：核心内置「网文创作流」不可删除（后端自动补回），改为停用；
  // 其余内置模板与自定义模板直接删除。
  async function deleteOrDisableTemplate(t: WorkflowTemplate) {
    if (t.builtin && t.template_id === "webnovel") {
      if (!(await confirmDialog(`核心内置模板「${t.name}」不可删除（系统会自动补回）。改为停用，让它不再出现在项目选择列表？`))) return;
      toggleEnabled(t);
      flash(`已停用核心内置模板「${t.name}」，可在「恢复内置模板」时重新启用`);
      return;
    }
    await deleteTemplate(t);
  }

  // ── 草稿编辑 ──

  function patchDraft(patch: Partial<WorkflowTemplate>) {
    setDraft(prev => (prev ? { ...prev, ...patch } : prev));
  }

  function patchStage(stage: string, patch: Partial<WorkflowStageDef>) {
    setDraft(prev => prev ? {
      ...prev,
      stages: prev.stages.map(s => s.stage === stage ? { ...s, ...patch } : s),
    } : prev);
  }

  function saveDraft() {
    if (!draft || !draft.name.trim()) {
      setError("模板名称不能为空");
      return;
    }
    if (!draft.template_id.trim()) {
      setError("模板 ID 不能为空");
      return;
    }
    const id = draft.template_id;
    const exists = templates.some(t => t.template_id === id && t.template_id !== editingId);
    if (exists) {
      setError(`模板 ID「${id}」已存在`);
      return;
    }
    setTemplates(prev => {
      const found = prev.some(t => t.template_id === id);
      return found
        ? prev.map(t => (t.template_id === id ? draft : t))
        : [...prev, draft];
    });
    setEditingId(null);
    setDraft(null);
    flash("已保存到草稿列表，点击「保存模板库」落盘");
  }

  // ── 模板级环节绑定（写入 template.bindings，项目自动继承） ──

  function patchBinding(t: WorkflowTemplate, stage: string, patch: Partial<StageSkillConfig>) {
    const bindings = (t.bindings ?? {}) as Record<string, StageSkillConfig>;
    const cur = bindings[stage] ?? emptyStageConfig();
    setTemplates(prev => prev.map(x => x.template_id === t.template_id
      ? { ...x, bindings: { ...bindings, [stage]: { ...cur, ...patch } } }
      : x));
  }

  // 某环节某维度槽当前绑定的卡（cards 中属于该维度的那张）
  function slotValue(t: WorkflowTemplate, stage: string, dim: string): string {
    const cfg = (t.bindings?.[stage] as StageSkillConfig | undefined);
    return (cfg?.cards ?? []).find(p => cardIndex.get(p)?.dimension === dim) ?? "";
  }

  function setSlot(t: WorkflowTemplate, stage: string, dim: string, path: string) {
    const cfg = (t.bindings?.[stage] as StageSkillConfig | undefined) ?? emptyStageConfig();
    const kept = (cfg.cards ?? []).filter(p => cardIndex.get(p)?.dimension !== dim);
    patchBinding(t, stage, { cards: path ? [...kept, path] : kept });
  }

  // ── 技能包管理 ──

  function handleDistilled(pkg: BookPackage | null) {
    if (pkg) {
      setPackages(prev => [pkg, ...prev]);
      flash(`《${pkg.title}》蒸馏完成：${pkg.cards.length} 张技能卡，可绑定到模板环节`);
    } else {
      // 后台完成（页面切换后重连场景）：重新拉取技能包列表
      listBookPackages().then(pkgs => setPackages(pkgs)).catch(() => {});
      flash("蒸馏已在后台完成，技能卡已刷新");
    }
    setShowDistill(false);
    setShowMethodologyDistill(false);
  }

  async function handleDeletePackage(pkg: BookPackage) {
    if (!(await confirmDialog(`删除技能包《${pkg.title}》？其中的 ${pkg.cards.length} 张技能卡将一并删除，不可恢复。`))) return;
    try {
      await deleteBookPackage(pkg.package);
    } catch (e: any) {
      setError(`删除失败: ${typeof e === "string" ? e : e?.message || String(e)}`);
      setTimeout(() => setError(""), 4000);
      return;
    }
    // 清理所有模板绑定中对已删卡的引用，并立即落盘
    const dead = new Set(pkg.cards.map(c => c.skill_path));
    const next = templates.map(t => {
      const bindings = (t.bindings ?? {}) as Record<string, StageSkillConfig>;
      let changed = false;
      const nb: Record<string, StageSkillConfig> = {};
      for (const [stage, cfg] of Object.entries(bindings)) {
        if (Array.isArray(cfg.cards)) {
          const cards = cfg.cards.filter(p => !dead.has(p));
          if (cards.length !== cfg.cards.length) changed = true;
          nb[stage] = { ...cfg, cards };
        } else {
          nb[stage] = cfg;
        }
      }
      return changed ? { ...t, bindings: nb } : t;
    });
    setTemplates(next);
    setPackages(prev => prev.filter(p => p.package !== pkg.package));
    try {
      await saveWorkflowTemplates(next);
    } catch (e: any) {
      setError("清理模板绑定失败: " + (e?.message ?? e));
    }
    flash(`已删除《${pkg.title}》技能包`);
  }

  const boundCount = templates.reduce(
    (n, t) => n + EXEC_STAGES.reduce((m, s) => {
      const cfg = (t.bindings?.[s] as StageSkillConfig | undefined);
      return m + (cfg?.cards?.length ?? 0);
    }, 0),
    0,
  );

  if (loading) {
    return <div className="view-container"><div className="empty-state" style={{ padding: 40 }}>加载工作流…</div></div>;
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>工作流</h2>
        <p style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", margin: 0 }}>
          模板库 + 环节技能绑定统一在主页面维护（存 data/workflows/templates.json）；项目内只选模板，
          各环节绑定（模型 + 技法卡）对所有项目生效
        </p>
      </div>

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: "var(--space-md)" }}>
        <button className="btn btn-primary" onClick={addTemplate}>
          <Plus size={14} /> 新建模板
        </button>
        <button className="btn btn-secondary" onClick={handleSaveAll}>
          <Save size={14} /> 保存模板库
        </button>
        <button className="btn btn-secondary" onClick={handleResetBuiltins}>
          <RotateCcw size={14} /> 恢复内置模板
        </button>
        <button className="btn btn-secondary" onClick={handleClearOverrides}>
          <Eraser size={14} /> 清空项目覆盖
        </button>
      </div>

      {msg && <div style={{ marginBottom: 12, padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "var(--color-jade-wash)", color: "var(--color-jade)", fontSize: "var(--text-xs)" }}>{msg}</div>}
      {error && <div style={{ marginBottom: 12, padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "#fef2f2", color: "#991b1b", fontSize: "var(--text-xs)" }}>{error}</div>}

      {templates.length === 0 && (
        <div className="empty-state" style={{ padding: 40 }}>
          <div className="empty-state-text">还没有工作流模板</div>
          <div className="empty-state-sub">点击「新建模板」创建第一个工作流，项目在造化工坊页选择模板后自动按此执行</div>
        </div>
      )}

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
        {templates.map(t => {
          const isEditing = editingId === t.template_id;
          const stageCount = t.stages?.filter(s => s.enabled).length ?? 0;
          const bindingsOpen = expandedBindings === t.template_id;
          return (
            <div key={t.template_id} className="card" style={{ border: isEditing ? "1px solid var(--color-jade)" : undefined }}>
              {isEditing && draft ? (
                <TemplateEditor
                  draft={draft}
                  onPatch={patchDraft}
                  onPatchStage={patchStage}
                  onSave={saveDraft}
                  onCancel={cancelEdit}
                />
              ) : (
                <>
                  <div className="card-header" style={{ justifyContent: "space-between" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", flexWrap: "wrap" }}>
                      <Workflow size={15} color="var(--color-ochre)" />
                      <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>{t.name}</span>
                      <span className="tag" style={{ fontSize: "var(--text-2xs)" }}>{t.genre}</span>
                      <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>v{t.version}</span>
                      {t.builtin && <span className="tag tag-success" style={{ fontSize: "var(--text-2xs)" }}>内置</span>}
                      <span className="tag" style={{ fontSize: "var(--text-2xs)" }}>{stageCount} 阶段 · 审查 {t.review_pass_score} 分放行</span>
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                        <input type="checkbox" checked={t.enabled} onChange={() => toggleEnabled(t)} />
                        启用
                      </label>
                      <button
                        className="pv-icon-btn pv-icon-btn-danger"
                        title={t.builtin && t.template_id === "webnovel" ? "核心内置模板不可删除，点击停用" : "删除模板"}
                        onClick={() => deleteOrDisableTemplate(t)}
                      >
                        <Trash2 size={14} />
                      </button>
                      <button className="pv-icon-btn" title="编辑模板" onClick={() => startEdit(t)}>
                        <Pencil size={14} />
                      </button>
                    </div>
                  </div>
                  <p className="pv-plugin-desc">{t.description}</p>

                  {/* 阶段流程可视化 */}
                  <div className="pv-pipeline">
                    <div className="pv-flow">
                      <div className="pv-node pv-node-start"><div className="pv-node-dot pv-dot-green" /><span>开始</span></div>
                      {t.stages.filter(s => s.enabled).map((s, idx) => (
                        <div key={s.stage} style={{ display: "flex", alignItems: "center" }}>
                          <div className="pv-connector"><div className="pv-connector-line" /><ArrowRight size={12} className="pv-connector-arrow" /></div>
                          <div
                            className={`pv-node pv-node-stage ${expandedStage === s.stage ? "pv-node-editing" : ""}`}
                            onClick={() => setExpandedStage(expandedStage === s.stage ? null : s.stage)}
                          >
                            <div className="pv-stage-header">
                              <span className="pv-stage-number">{idx + 1}</span>
                              <span className="pv-stage-name">{s.display_name}</span>
                            </div>
                            <div className="pv-stage-tags">
                              <span className={`pv-tag pv-tag-gate pv-gate-${s.gate}`}>{gateIcons[s.gate]} {gateLabels[s.gate]}</span>
                            </div>
                          </div>
                        </div>
                      ))}
                      <div className="pv-connector"><div className="pv-connector-line" /><ArrowRight size={12} className="pv-connector-arrow" /></div>
                      <div className="pv-node pv-node-end"><Check size={14} /><span>完成</span></div>
                    </div>

                    {expandedStage && t.stages.find(s => s.stage === expandedStage) && (
                      <div className="pv-stage-detail">
                        <div className="pv-detail-row">
                          <span className="pv-detail-label">阶段</span>
                          <span className="pv-detail-value">{t.stages.find(s => s.stage === expandedStage)!.display_name}</span>
                        </div>
                        <div className="pv-detail-row">
                          <span className="pv-detail-label">门控</span>
                          <span className="pv-detail-value">{gateLabels[t.stages.find(s => s.stage === expandedStage)!.gate]}放行</span>
                        </div>
                        <div className="pv-detail-row">
                          <span className="pv-detail-label">重试</span>
                          <span className="pv-detail-value">{t.stages.find(s => s.stage === expandedStage)!.max_retries} 次</span>
                        </div>
                        {t.stages.find(s => s.stage === expandedStage)!.on_fail && (
                          <div className="pv-detail-row">
                            <span className="pv-detail-label">失败回退</span>
                            <span className="pv-detail-value">{t.stages.find(s => s.stage === expandedStage)!.on_fail}</span>
                          </div>
                        )}
                        <div className="pv-detail-row pv-detail-row-full">
                          <span className="pv-detail-label">工作手册</span>
                          <p className="pv-detail-prompt">{t.stages.find(s => s.stage === expandedStage)!.prompt_hint}</p>
                        </div>
                      </div>
                    )}
                  </div>

                  {/* 模板级环节技能绑定 */}
                  <div style={{ marginTop: "var(--space-md)", borderTop: "1px solid var(--color-rule-light)", paddingTop: "var(--space-sm)" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", flexWrap: "wrap" }}>
                      <button
                        className="btn btn-secondary"
                        style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}
                        onClick={() => setExpandedBindings(bindingsOpen ? null : t.template_id)}
                      >
                        <Layers size={13} /> {bindingsOpen ? "收起环节绑定" : "环节技能绑定"}
                      </button>
                      <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                        模板级默认绑定：各项目选用此模板时自动生效（{EXEC_STAGES.reduce((n, s) => {
                          const cfg = (t.bindings?.[s] as StageSkillConfig | undefined);
                          return n + (cfg?.cards?.length ?? 0);
                        }, 0)} 张卡）
                      </span>
                    </div>

                    {bindingsOpen && (
                      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)", marginTop: "var(--space-sm)" }}>
                        {EXEC_STAGES.map(stage => {
                          const cfg = (t.bindings?.[stage] as StageSkillConfig | undefined) ?? emptyStageConfig();
                          return (
                            <div key={stage} style={{ display: "flex", alignItems: "center", gap: "var(--space-md)", flexWrap: "wrap", padding: "var(--space-sm) var(--space-md)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                              <span style={{ width: 76, fontWeight: 600, fontSize: "var(--text-sm)", flexShrink: 0 }}>{STAGE_LABELS[stage]}</span>
                              <select
                                className="pm-input"
                                style={{ marginBottom: 0, width: 170, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                                value={cfg.model ?? ""}
                                onChange={e => { patchBinding(t, stage, { model: e.target.value || null }); }}
                                title="本环节使用的 LLM 模型（留空=自动）"
                              >
                                <option value="">自动模型</option>
                                {models.map(m => (
                                  <option key={m.model_id} value={m.model_id}>{m.display_name || m.model_id}</option>
                                ))}
                              </select>
                              {STAGE_DIMS[stage].map(d => (
                                <label key={d.slug} style={{ display: "flex", alignItems: "center", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                                  {d.label}
                                  <select
                                    className="pm-input"
                                    style={{ marginBottom: 0, width: 190, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                                    value={slotValue(t, stage, d.slug)}
                                    onChange={e => { setSlot(t, stage, d.slug, e.target.value); }}
                                  >
                                    <option value="">不绑卡</option>
                                    {candidates(stage, d.slug).map(c => (
                                      <option key={c.skill_path} value={c.skill_path}>《{c.pkgTitle}》</option>
                                    ))}
                                  </select>
                                </label>
                              ))}
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          );
        })}
      </div>

      {/* ── 写作技能库 ── */}
      <div className="card" style={{ marginTop: "var(--space-lg)" }}>
        <div className="card-header" style={{ justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
            <BookOpen size={15} color="var(--color-ink-3)" />
            <h3>写作技能库（{packages.length} 个技能包 · 已绑定 {boundCount} 张卡）</h3>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }} onClick={() => setShowMethodologyDistill(s => !s)}>
              <Sparkles size={13} /> 蒸馏方法论
            </button>
            <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }} onClick={() => setShowDistill(s => !s)}>
              <Sparkles size={13} /> 蒸馏一本书
            </button>
          </div>
        </div>

        {showDistill && (
          <BookDistillPanel models={models} onDistilled={handleDistilled} onClose={() => setShowDistill(false)} />
        )}
        {showMethodologyDistill && (
          <MethodologyDistillPanel models={models} onDistilled={handleDistilled} onClose={() => setShowMethodologyDistill(false)} />
        )}

        {packages.length === 0 && !showDistill && !showMethodologyDistill ? (
          <div className="empty-state" style={{ padding: "24px" }}>
            <div className="empty-state-text">技能库为空</div>
            <div className="empty-state-sub">
              蒸馏一本书或一段方法论，提炼为写作技能卡（保存到 WritingCard 文件夹），再绑定到上方模板环节
            </div>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
            {packages.map(pkg => (
              <div key={pkg.package} style={{ display: "flex", alignItems: "center", gap: "var(--space-md)", padding: "var(--space-sm) var(--space-md)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                <BookOpen size={16} style={{ color: "var(--color-accent)", flexShrink: 0 }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                    <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>《{pkg.title}》</span>
                    {pkg.author && <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{pkg.author}</span>}
                    {pkg.created_at && <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{pkg.created_at}</span>}
                  </div>
                  <div style={{ display: "flex", gap: 6, marginTop: 4, flexWrap: "wrap" }}>
                    {pkg.cards.map(c => (
                      <span key={c.skill_path} title={c.description} style={{ fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: 10, background: "var(--color-indigo-wash)", color: "var(--color-indigo)" }}>
                        {c.dimension_label}
                      </span>
                    ))}
                  </div>
                </div>
                <button className="pv-icon-btn pv-icon-btn-danger" title="删除技能包" onClick={() => handleDeletePackage(pkg)}>
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── 模板编辑表单 ──

function TemplateEditor({
  draft,
  onPatch,
  onPatchStage,
  onSave,
  onCancel,
}: {
  draft: WorkflowTemplate;
  onPatch: (patch: Partial<WorkflowTemplate>) => void;
  onPatchStage: (stage: string, patch: Partial<WorkflowStageDef>) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  return (
    <div style={{ padding: "4px 0" }}>
      <div className="card-header" style={{ justifyContent: "space-between" }}>
        <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>编辑模板</span>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }} onClick={onSave}>
            <Check size={13} /> 保存到草稿
          </button>
          <button className="btn btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }} onClick={onCancel}>
            <X size={13} /> 取消
          </button>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-sm)", marginTop: 8 }}>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          模板名称
          <input className="pm-input" value={draft.name} onChange={e => onPatch({ name: e.target.value })} />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          模板 ID（不可变，英文/数字/连字符）
          <input className="pm-input" value={draft.template_id} disabled={draft.builtin} onChange={e => onPatch({ template_id: e.target.value })} />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          体裁标签
          <input className="pm-input" value={draft.genre} onChange={e => onPatch({ genre: e.target.value })} placeholder="网文 / 传统 / 科幻 / 通用…" />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          版本号
          <input className="pm-input" value={draft.version} onChange={e => onPatch({ version: e.target.value })} />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          审查放行阈值（0-100）
          <input
            className="pm-input"
            type="number" min={0} max={100}
            value={draft.review_pass_score}
            onChange={e => onPatch({ review_pass_score: Number(e.target.value) || 0 })}
          />
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", paddingTop: 16 }}>
          <input type="checkbox" checked={draft.enabled} onChange={e => onPatch({ enabled: e.target.checked })} />
          启用（停用后不进入项目选择列表）
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", gridColumn: "1 / -1" }}>
          说明
          <textarea className="pm-input" rows={2} value={draft.description} onChange={e => onPatch({ description: e.target.value })} />
        </label>
      </div>

      <div style={{ marginTop: "var(--space-md)", display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
        <span style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink-2)" }}>执行阶段（按模板编排，可调整手册/门控/重试）</span>
        {draft.stages.map(s => (
          <div key={s.stage} style={{ padding: "var(--space-sm) var(--space-md)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", flexWrap: "wrap" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--text-xs)", width: 110 }}>{STAGE_KEY_LABELS[s.stage] ?? s.stage}</span>
              <input
                className="pm-input"
                style={{ width: 160, marginBottom: 0, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                value={s.display_name}
                onChange={e => onPatchStage(s.stage, { display_name: e.target.value })}
                placeholder="阶段显示名"
              />
              <select
                className="pm-input"
                style={{ width: 110, marginBottom: 0, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                value={s.gate}
                onChange={e => onPatchStage(s.stage, { gate: e.target.value as WorkflowStageDef["gate"] })}
              >
                <option value="auto">自动</option>
                <option value="manual">人工</option>
                <option value="conditional">条件</option>
              </select>
              <select
                className="pm-input"
                style={{ width: 130, marginBottom: 0, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                value={s.on_fail ?? ""}
                onChange={e => onPatchStage(s.stage, { on_fail: e.target.value || null })}
                title="门控失败时的回退阶段"
              >
                <option value="">不回退</option>
                {draft.stages.filter(x => x.stage !== s.stage).map(x => (
                  <option key={x.stage} value={x.stage}>回退到 {STAGE_KEY_LABELS[x.stage] ?? x.stage}</option>
                ))}
              </select>
              <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                重试
                <input
                  className="pm-input"
                  type="number" min={0} max={10}
                  style={{ width: 52, marginBottom: 0, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                  value={s.max_retries}
                  onChange={e => onPatchStage(s.stage, { max_retries: Math.max(0, Number(e.target.value) || 0) })}
                />
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                <input type="checkbox" checked={s.enabled} onChange={e => onPatchStage(s.stage, { enabled: e.target.checked })} />
                启用
              </label>
              {s.stage === "chapter_review" && (
                <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}
                  title="前 3 章强制要求开场钩子与爽点维度 ≥ 8 分，否则拦截重写">
                  <input
                    type="checkbox"
                    checked={s.golden_gate ?? false}
                    onChange={e => onPatchStage(s.stage, { golden_gate: e.target.checked })}
                  />
                  黄金三章门控（前 3 章）
                </label>
              )}
              {s.stage === "chapter_planning" && (
                <button
                  className="pv-icon-btn pv-icon-btn-danger"
                  title="删除章前策划（模板退回默认三阶段）"
                  onClick={() => onPatch({ stages: draft.stages.filter(x => x.stage !== "chapter_planning") })}
                >
                  <Trash2 size={13} />
                </button>
              )}
            </div>
            <textarea
              className="pm-input"
              rows={2}
              style={{ marginTop: 6 }}
              value={s.prompt_hint}
              onChange={e => onPatchStage(s.stage, { prompt_hint: e.target.value })}
              placeholder="阶段工作手册（注入引擎指导怎么写/怎么判）"
            />
          </div>
        ))}
        {!draft.stages.some(s => s.stage === "chapter_planning") && (
          <button
            className="btn btn-secondary"
            style={{ padding: "4px 10px", fontSize: "var(--text-xs)", alignSelf: "flex-start" }}
            onClick={() => onPatch({
              stages: [
                ...draft.stages,
                {
                  stage: "chapter_planning",
                  display_name: "章前策划",
                  prompt_hint: "写前策划：产出本章节拍表（章节目标、开场钩子、3-6 个场景、爽点、断章钩子、伏笔埋收）。",
                  gate: "auto",
                  on_fail: null,
                  max_retries: 2,
                  enabled: true,
                },
              ],
            })}
          >
            <Plus size={12} /> 添加章前策划
          </button>
        )}
      </div>
    </div>
  );
}
