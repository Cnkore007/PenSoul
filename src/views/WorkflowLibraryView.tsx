import { useState, useEffect, useCallback } from "react";
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
} from "lucide-react";
import type { WorkflowTemplate, WorkflowStageDef } from "../types";
import {
  listWorkflowTemplates,
  saveWorkflowTemplates,
  resetWorkflowTemplates,
} from "../ipc";

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

// 新模板默认三阶段（与管线固定三阶段 key 一致）
function defaultStages(): WorkflowStageDef[] {
  return [
    {
      stage: "chapter_writing",
      display_name: "章节写作",
      prompt_hint: "根据本章梗概与前文承接撰写正文。",
      gate: "auto",
      on_fail: null,
      max_retries: 2,
      enabled: true,
    },
    {
      stage: "chapter_review",
      display_name: "一致性审查",
      prompt_hint: "用不同模型审查本章一致性，输出 score 与 issues。",
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

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setTemplates(await listWorkflowTemplates());
    } catch (e: any) {
      setError("加载工作流模板失败: " + (e?.message ?? e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

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
      flash("模板库已保存");
    } catch (e: any) {
      setError("保存失败: " + (e?.message ?? e));
    }
  }

  async function handleResetBuiltins() {
    if (!window.confirm("恢复内置模板到出厂状态？自定义模板会保留，内置模板上手动改过的内容将丢失。")) return;
    try {
      setTemplates(await resetWorkflowTemplates());
      setEditingId(null);
      setDraft(null);
      flash("内置模板已恢复");
    } catch (e: any) {
      setError("恢复失败: " + (e?.message ?? e));
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
    if (!window.confirm(`删除模板「${t.name}」？项目若引用了它将无法解析，请先在其他项目里换模板。`)) return;
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

  if (loading) {
    return <div className="view-container"><div className="empty-state" style={{ padding: 40 }}>加载模板库…</div></div>;
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>工作流模板库</h2>
        <p style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", margin: 0 }}>
          作品库层面的全局工作流：定义「网文 / 传统 / 科幻 / 通用」等模板，项目里只引用模板并做局部覆盖
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
      </div>

      {msg && <div style={{ marginBottom: 12, padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "var(--color-jade-wash)", color: "var(--color-jade)", fontSize: "var(--text-xs)" }}>{msg}</div>}
      {error && <div style={{ marginBottom: 12, padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "#fef2f2", color: "#991b1b", fontSize: "var(--text-xs)" }}>{error}</div>}

      {templates.length === 0 && (
        <div className="empty-state" style={{ padding: 40 }}>
          <div className="empty-state-text">还没有工作流模板</div>
          <div className="empty-state-sub">点击「新建模板」创建第一个工作流，或在作品库页面打开已有模板</div>
        </div>
      )}

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
        {templates.map(t => {
          const isEditing = editingId === t.template_id;
          const stageCount = t.stages?.filter(s => s.enabled).length ?? 0;
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
                      {!t.builtin && (
                        <button className="pv-icon-btn pv-icon-btn-danger" title="删除模板" onClick={() => deleteTemplate(t)}>
                          <Trash2 size={14} />
                        </button>
                      )}
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
                </>
              )}
            </div>
          );
        })}
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
        <span style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink-2)" }}>执行阶段（管线固定三阶段，可调整手册/门控/重试）</span>
        {draft.stages.map(s => (
          <div key={s.stage} style={{ padding: "var(--space-sm) var(--space-md)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", flexWrap: "wrap" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--text-xs)", width: 110 }}>{s.stage}</span>
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
                  <option key={x.stage} value={x.stage}>回退到 {x.stage}</option>
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
      </div>
    </div>
  );
}
