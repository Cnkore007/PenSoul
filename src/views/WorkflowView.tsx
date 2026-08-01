import { useState, useEffect, useMemo } from "react";
import { Play, ArrowRight, CheckCircle, Zap, AlertCircle, Clock, Save, BookOpen, Trash2, Sparkles, RotateCcw, Layers } from "lucide-react";
import type { ProjectData, ViewType, BookPackage, BookCardInfo, LlmModel, StageSkillConfig, WorkflowRef, WorkflowTemplate } from "../types";
import { listBookPackages, deleteBookPackage, listModels, listWorkflowTemplates } from "../ipc";
import { EXEC_STAGES, computeEffectiveSkills, effectiveStageConfig } from "../workflow";
import { BookDistillPanel } from "../components/BookDistillPanel";

interface WorkflowViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  onNavigate?: (view: ViewType) => void;
}

// 三个可绑定技能卡/模型的执行环节（key 与后端 applicable_stages 一致）
const STAGE_LABELS: Record<string, string> = {
  outline_expand: "细纲展开",
  chapter_writing: "章节写作",
  review: "一致性审查",
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

const gateIcons: Record<string, React.ReactNode> = { auto: <Zap size={13} />, manual: <AlertCircle size={13} />, conditional: <Clock size={13} /> };
const gateLabels: Record<string, string> = { auto: "自动", manual: "人工", conditional: "条件" };

type IndexedCard = BookCardInfo & { pkgTitle: string };

function emptyRef(): WorkflowRef {
  return { template_id: null, template_version: null, overrides: {} };
}

export function WorkflowView({ projectData, persistProjectData, onNavigate }: WorkflowViewProps) {
  const [templates, setTemplates] = useState<WorkflowTemplate[]>([]);
  const [expandedStage, setExpandedStage] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  // 写作技能库
  const [packages, setPackages] = useState<BookPackage[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [showDistill, setShowDistill] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    listBookPackages().then(setPackages).catch(() => {});
    listModels()
      .then((ms) => setModels((ms || []).filter((m: any) => m.is_available !== false)))
      .catch(() => {});
    listWorkflowTemplates()
      .then(setTemplates)
      .catch(() => {});
  }, []);

  // 当前项目引用（未配置时用空引用）
  const ref: WorkflowRef = projectData.workflowRef ?? emptyRef();
  const selectedTemplate = templates.find(t => t.template_id === ref.template_id);

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

  function flashSaved() {
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }

  // ── 项目引用写入：同时重算派生有效配置（大纲展开/造化工坊即时消费） ──
  function persistRef(next: WorkflowRef) {
    persistProjectData(prev => ({
      ...prev,
      workflowRef: next,
      workflowSkills: computeEffectiveSkills(templates, next),
    }));
    flashSaved();
  }

  // 选择/取消模板（保留已有覆盖，换模板后覆盖仍生效）
  function selectTemplate(templateId: string) {
    const t = templates.find(x => x.template_id === templateId);
    persistRef({
      template_id: t ? t.template_id : null,
      template_version: t ? t.version : null,
      overrides: ref.overrides ?? {},
    });
  }

  // 写某环节的项目覆盖
  function updateOverride(stage: string, patch: Partial<StageSkillConfig>) {
    const cur = effectiveStageConfig(templates, ref, stage);
    persistRef({
      ...ref,
      overrides: {
        ...(ref.overrides ?? {}),
        [stage]: { ...cur, ...patch },
      },
    });
  }

  // 清除某环节的项目覆盖（恢复为模板绑定/自动）
  function clearOverride(stage: string) {
    const overrides = { ...(ref.overrides ?? {}) };
    delete overrides[stage];
    persistRef({ ...ref, overrides });
  }

  // 某环节某维度槽当前绑定的卡（cards 中属于该维度的那张）
  function slotValue(stage: string, dim: string): string {
    const cfg = effectiveStageConfig(templates, ref, stage);
    return (cfg.cards as string[]).find(p => cardIndex.get(p)?.dimension === dim) ?? "";
  }
  function setSlot(stage: string, dim: string, path: string) {
    const cfg = effectiveStageConfig(templates, ref, stage);
    const kept = cfg.cards.filter(p => cardIndex.get(p)?.dimension !== dim);
    updateOverride(stage, { cards: path ? [...kept, path] : kept });
  }

  function goToHarness() {
    onNavigate?.("harness");
  }

  // ── 技能包管理 ──
  function handleDistilled(pkg: BookPackage) {
    setPackages(prev => [pkg, ...prev]);
    setShowDistill(false);
    setMsg(`《${pkg.title}》蒸馏完成：${pkg.cards.length} 张技能卡，可在上方绑定到环节`);
    setTimeout(() => setMsg(""), 5000);
  }

  async function handleDeletePackage(pkg: BookPackage) {
    if (!window.confirm(`删除技能包《${pkg.title}》？其中的 ${pkg.cards.length} 张技能卡将一并删除，不可恢复。`)) return;
    try {
      await deleteBookPackage(pkg.package);
    } catch (e: any) {
      setMsg(`删除失败: ${typeof e === "string" ? e : e?.message || String(e)}`);
      setTimeout(() => setMsg(""), 4000);
      return;
    }
    // 清理项目覆盖中对已删卡的引用
    const dead = new Set(pkg.cards.map(c => c.skill_path));
    const overrides = { ...(ref.overrides ?? {}) };
    for (const s of EXEC_STAGES) {
      const oc = overrides[s];
      if (oc && Array.isArray(oc.cards)) {
        overrides[s] = { ...oc, cards: oc.cards.filter(p => !dead.has(p)) };
      }
    }
    persistRef({ ...ref, overrides });
    setPackages(prev => prev.filter(p => p.package !== pkg.package));
    setMsg(`已删除《${pkg.title}》技能包`);
    setTimeout(() => setMsg(""), 3000);
  }

  const boundCount = EXEC_STAGES.reduce((n, s) => n + effectiveStageConfig(templates, ref, s).cards.length, 0);
  const selectableTemplates = templates.filter(t => t.enabled);

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>项目工作流</h2>
        {saved && <span className="tag tag-success"><Save size={12} /> 已保存</span>}
      </div>

      <div className="empty-state" style={{ padding: "16px 0 24px", textAlign: "left" }}>
        <div className="empty-state-sub" style={{ fontSize: "var(--text-sm)" }}>
          模板定义在作品库「工作流模板库」；本项目只选择要用的模板，并在下方按环节覆盖模型与技法卡。
          大纲展开与造化工坊会按「项目覆盖 → 模板绑定」自动解析。
        </div>
      </div>

      {/* ── 模板选择 ── */}
      <div className="card" style={{ marginBottom: "var(--space-lg)" }}>
        <div className="card-header" style={{ justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
            <Layers size={15} color="var(--color-ink-3)" />
            <h3>工作流模板</h3>
          </div>
          <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
            模板本体在作品库统一维护，项目只存引用
          </span>
        </div>

        {selectableTemplates.length === 0 ? (
          <div className="empty-state" style={{ padding: 24 }}>
            <div className="empty-state-text">作品库还没有可用的工作流模板</div>
            <div className="empty-state-sub">请先到作品库「工作流模板库」新建或启用模板</div>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
            {selectableTemplates.map(t => {
              const isActive = ref.template_id === t.template_id;
              return (
                <button
                  key={t.template_id}
                  onClick={() => selectTemplate(isActive ? "" : t.template_id)}
                  style={{
                    display: "flex", alignItems: "center", gap: "var(--space-md)",
                    padding: "var(--space-sm) var(--space-md)",
                    border: `1px solid ${isActive ? "var(--color-jade)" : "var(--color-rule)"}`,
                    borderRadius: "var(--radius-sm)",
                    background: isActive ? "var(--color-jade-wash)" : "var(--color-paper-warm)",
                    textAlign: "left", cursor: "pointer",
                  }}
                >
                  <span className={`pv-node-dot ${isActive ? "pv-dot-green" : ""}`} style={{ opacity: isActive ? 1 : 0.35 }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                      <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>{t.name}</span>
                      <span className="tag" style={{ fontSize: "var(--text-2xs)" }}>{t.genre}</span>
                      {isActive && <span className="tag tag-success" style={{ fontSize: "var(--text-2xs)" }}>本项目使用</span>}
                    </div>
                    <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 2 }}>
                      {t.description}
                    </div>
                  </div>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", textAlign: "right" }}>
                    v{t.version}<br />{t.stages.filter(s => s.enabled).length} 阶段 · 审查 {t.review_pass_score} 分
                  </div>
                </button>
              );
            })}
          </div>
        )}

        {/* 模板流程可视化 */}
        {selectedTemplate && (
          <div className="pv-pipeline" style={{ marginTop: "var(--space-md)" }}>
            <div className="pv-flow">
              <div className="pv-node pv-node-start"><div className="pv-node-dot pv-dot-green" /><span>开始</span></div>
              {selectedTemplate.stages.filter(s => s.enabled).map((s, idx) => (
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
              <div className="pv-node pv-node-end"><CheckCircle size={14} /><span>完成</span></div>
            </div>

            {expandedStage && selectedTemplate.stages.find(s => s.stage === expandedStage) && (
              <div className="pv-stage-detail">
                <div className="pv-detail-row">
                  <span className="pv-detail-label">阶段</span>
                  <span className="pv-detail-value">{selectedTemplate.stages.find(s => s.stage === expandedStage)!.display_name}</span>
                </div>
                <div className="pv-detail-row">
                  <span className="pv-detail-label">门控</span>
                  <span className="pv-detail-value">{gateLabels[selectedTemplate.stages.find(s => s.stage === expandedStage)!.gate]}放行</span>
                </div>
                <div className="pv-detail-row">
                  <span className="pv-detail-label">重试</span>
                  <span className="pv-detail-value">{selectedTemplate.stages.find(s => s.stage === expandedStage)!.max_retries} 次</span>
                </div>
                {selectedTemplate.stages.find(s => s.stage === expandedStage)!.on_fail && (
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">失败回退</span>
                    <span className="pv-detail-value">{selectedTemplate.stages.find(s => s.stage === expandedStage)!.on_fail}</span>
                  </div>
                )}
                <div className="pv-detail-row pv-detail-row-full">
                  <span className="pv-detail-label">工作手册</span>
                  <p className="pv-detail-prompt">{selectedTemplate.stages.find(s => s.stage === expandedStage)!.prompt_hint}</p>
                </div>
              </div>
            )}
          </div>
        )}

        {selectedTemplate && (
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-md)", marginTop: "var(--space-md)" }}>
            <button className="btn btn-primary" onClick={goToHarness}>
              <Play size={15} /> 前往造化工坊
            </button>
            <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
              已绑定 {boundCount} 张技法卡
            </span>
          </div>
        )}
      </div>

      {/* ── 环节覆盖（模型 + 技法卡） ── */}
      <div className="card" style={{ marginBottom: "var(--space-lg)" }}>
        <div className="card-header" style={{ justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
            <Zap size={15} color="var(--color-ink-3)" />
            <h3>项目环节覆盖</h3>
          </div>
          <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
            每个环节 = 模型 + 技法卡（每维度最多一张）；不覆盖时跟随模板绑定
          </span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)", padding: "4px 0" }}>
          {EXEC_STAGES.map(stage => {
            const cfg = effectiveStageConfig(templates, ref, stage);
            const bound = selectedTemplate?.bindings?.[stage] as StageSkillConfig | undefined;
            const hasBinding = !!(bound && (bound.model || (bound.cards?.length ?? 0) > 0));
            const hasOverride = !!ref.overrides?.[stage];
            return (
              <div key={stage} style={{ display: "flex", alignItems: "center", gap: "var(--space-md)", flexWrap: "wrap", padding: "var(--space-sm) var(--space-md)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                <span style={{ width: 76, fontWeight: 600, fontSize: "var(--text-sm)", flexShrink: 0 }}>{STAGE_LABELS[stage]}</span>
                <select
                  className="pm-input"
                  style={{ marginBottom: 0, width: 170, padding: "4px 8px", fontSize: "var(--text-xs)" }}
                  value={cfg.model ?? ""}
                  onChange={e => { updateOverride(stage, { model: e.target.value || null }); }}
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
                      value={slotValue(stage, d.slug)}
                      onChange={e => { setSlot(stage, d.slug, e.target.value); }}
                    >
                      <option value="">不绑卡</option>
                      {candidates(stage, d.slug).map(c => (
                        <option key={c.skill_path} value={c.skill_path}>《{c.pkgTitle}》</option>
                      ))}
                    </select>
                  </label>
                ))}
                {hasOverride && (
                  <button className="pv-icon-btn" title="恢复模板默认（清除本项目覆盖）" onClick={() => clearOverride(stage)}>
                    <RotateCcw size={13} />
                  </button>
                )}
                {hasBinding && !hasOverride && (
                  <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ochre)" }}>
                    跟随模板默认{bound?.model ? ` · ${bound.model}` : ""}
                  </span>
                )}
              </div>
            );
          })}
        </div>
        {packages.length === 0 && (
          <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ochre)", padding: "var(--space-sm) var(--space-md)" }}>
            还没有可绑定的技能卡——先在下方「写作技能库」蒸馏一本书
          </div>
        )}
      </div>

      {/* ── 写作技能库 ── */}
      <div className="card" style={{ marginBottom: "var(--space-lg)" }}>
        <div className="card-header" style={{ justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
            <BookOpen size={15} color="var(--color-ink-3)" />
            <h3>写作技能库（{packages.length} 个技能包）</h3>
          </div>
          <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }} onClick={() => setShowDistill(s => !s)}>
            <Sparkles size={13} /> 蒸馏一本书
          </button>
        </div>

        {showDistill && (
          <BookDistillPanel models={models} onDistilled={handleDistilled} onClose={() => setShowDistill(false)} />
        )}

        {msg && (
          <div style={{ margin: "0 0 var(--space-sm)", padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "var(--color-jade-wash)", color: "var(--color-jade)", fontSize: "var(--text-xs)" }}>
            {msg}
          </div>
        )}

        {packages.length === 0 && !showDistill ? (
          <div className="empty-state" style={{ padding: "24px" }}>
            <div className="empty-state-text">技能库为空</div>
            <div className="empty-state-sub">
              蒸馏一本书，提炼它的文风 / 结构 / 人物 / 张力 / 类型技法为技能卡（保存到 WritingCard 文件夹），再绑定到上方环节
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

      {!selectedTemplate && (
        <div style={{ marginTop: 24, padding: 16, background: "var(--color-ochre-wash)", borderRadius: "var(--radius-sm)", color: "var(--color-ochre)", fontSize: "var(--text-sm)" }}>
          <AlertCircle size={14} style={{ verticalAlign: "middle", marginRight: 6 }} />
          尚未选择工作流模板。请在上方选择一个模板，Agent 将按模板阶段自动推进创作（也可不选模板直接绑定环节）。
        </div>
      )}
    </div>
  );
}
