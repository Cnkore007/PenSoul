import { useState, useEffect, useCallback, useRef } from "react";
import type { Expert, LlmModel } from "../types";
import { DEFAULT_DISCUSSION_AGENTS } from "../types";
import { saveExperts, loadExperts, listModels, scanExpertsFolder, distillExpert, getExpertsFolder, deleteExpertSkill, getDistillState } from "../ipc";
import { listen } from "@tauri-apps/api/event";
import {
  Bot, Trash2, Edit3,
  Sparkles, FolderOpen, Loader2, CheckCircle2, XCircle,
  Cpu, User, Save,
} from "lucide-react";

interface PhaseEvent { phase: string; status: string; message: string; detail: string; }

export function ExpertLibraryView() {
  const [experts, setExperts] = useState<Expert[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [msg, setMsg] = useState("");
  const [loading, setLoading] = useState(true);

  // 编辑
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editForm, setEditForm] = useState({ name: "", description: "", sourcePersona: "", modelId: "gpt-4o", perspective: "", defaultPrompt: "" });

  // 蒸馏
  const [showDistill, setShowDistill] = useState(false);
  const [distillPersona, setDistillPersona] = useState("");
  const [distillModel, setDistillModel] = useState("gpt-4o");
  const [distillRunning, setDistillRunning] = useState(false);
  const [phases, setPhases] = useState<PhaseEvent[]>([]);
  const unlistenRef = useRef<(() => void) | null>(null);

  // 本地导入
  const [showImport, setShowImport] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [foundSkills, setFoundSkills] = useState<(Expert & { selected: boolean })[]>([]);

  // 重新拉取专家列表（蒸馏在后台完成后刷新用）
  const refreshExperts = useCallback(async () => {
    const raw = await loadExperts().catch(() => []);
    setExperts((raw || []).map((e: any) => ({
      id: e.id, name: e.name, description: e.description,
      sourcePersona: e.source_persona, modelId: e.model_id,
      perspective: e.perspective, defaultPrompt: e.default_prompt,
      createdAt: e.created_at, skillPath: e.skill_path, skillSummary: e.skill_summary,
    })));
  }, []);

  useEffect(() => {
    Promise.all([
      loadExperts().catch(() => []),
      listModels().catch(() => []),
    ]).then(([rawExperts, rawModels]) => {
      setExperts((rawExperts || []).map((e: any) => ({
        id: e.id, name: e.name, description: e.description,
        sourcePersona: e.source_persona, modelId: e.model_id,
        perspective: e.perspective, defaultPrompt: e.default_prompt,
        createdAt: e.created_at, skillPath: e.skill_path, skillSummary: e.skill_summary,
      })));
      setModels(rawModels);
      setLoading(false);
    });
  }, []);

  // 页面切换后重连：若专家蒸馏仍在后台进行，自动打开蒸馏面板并恢复进度
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const st = await getDistillState().catch(() => null);
      if (!st || cancelled || !st.running || st.kind !== "expert") return;
      setDistillRunning(true);
      setShowDistill(true);
      (st.events ?? []).forEach(ev => {
        if (ev.phase === "__distill__") return;
        setPhases(prev => {
          const i = prev.findIndex(p => p.phase === ev.phase);
          if (i >= 0) {
            const u = [...prev];
            u[i] = ev;
            return u;
          }
          return [...prev, ev];
        });
      });
      const unlistenFn = await listen<PhaseEvent>("distill-phase", (evt) => {
        const e = evt.payload;
        if (e.phase === "__distill__") {
          if (e.status === "finished") {
            void refreshExperts();
            setMsg("专家蒸馏已在后台完成，列表已刷新");
          } else {
            setPhases(prev => [...prev, e]);
            setMsg(`专家蒸馏失败：${e.message}`);
          }
          setDistillRunning(false);
          return;
        }
        setPhases(prev => {
          const i = prev.findIndex(p => p.phase === e.phase);
          if (i >= 0) {
            const u = [...prev];
            u[i] = e;
            return u;
          }
          return [...prev, e];
        });
      });
      unlistenRef.current = unlistenFn;
      if (cancelled) { unlistenFn(); return; }
      // 订阅间隙任务可能已结束：复查一次，避免错过终态
      const st2 = await getDistillState().catch(() => null);
      if (cancelled) { unlistenFn(); return; }
      if (st2 && !st2.running) {
        unlistenFn();
        setDistillRunning(false);
        void refreshExperts();
      }
    })();
    return () => { cancelled = true; };
  }, [refreshExperts]);

  useEffect(() => {
    return () => { if (unlistenRef.current) unlistenRef.current(); };
  }, []);

  const persist = useCallback(async (updated: Expert[]) => {
    setExperts(updated);
    const payload = updated.map(e => ({
      id: e.id, name: e.name, description: e.description,
      source_persona: e.sourcePersona, model_id: e.modelId,
      perspective: e.perspective, default_prompt: e.defaultPrompt,
      created_at: e.createdAt, skill_path: e.skillPath || null,
      skill_summary: e.skillSummary || null,
    }));
    try { await saveExperts(payload); } catch (e) { console.error("保存失败:", e); }
  }, []);

  const handleEdit = (e: Expert) => {
    setEditForm({
      name: e.name, description: e.description, sourcePersona: e.sourcePersona,
      modelId: e.modelId, perspective: e.perspective, defaultPrompt: e.defaultPrompt,
    });
    setEditingId(e.id);
  };

  const handleSaveEdit = () => {
    if (!editForm.name.trim()) return;
    persist(experts.map(e => e.id === editingId ? {
      ...e, ...editForm, createdAt: e.createdAt,
    } : e));
    setEditingId(null);
  };

  const handleDelete = async (id: string) => {
    const target = experts.find(e => e.id === id);
    if (target?.skillPath) {
      try { await deleteExpertSkill(target.skillPath); } catch (_) {}
    }
    persist(experts.filter(e => e.id !== id));
  };

  // ── 本地导入：自动扫描 Experts 文件夹 ──
  const openImportPanel = async () => {
    setShowImport(true);
    setScanning(true);
    setFoundSkills([]);
    try {
      const folderPath = await getExpertsFolder();
      const raw = await scanExpertsFolder(folderPath);
      if (raw && raw.length > 0) {
        const existingPaths = new Set(experts.filter(e => e.skillPath).map(e => e.skillPath));
        setFoundSkills(raw.map((e: any) => ({
          id: e.id, name: e.name, description: e.description,
          sourcePersona: e.source_persona, modelId: e.model_id,
          perspective: e.perspective, defaultPrompt: e.default_prompt,
          createdAt: e.created_at, skillPath: e.skill_path, skillSummary: e.skill_summary,
          selected: !e.skill_path || !existingPaths.has(e.skill_path),
        })));
      }
    } catch (e: any) {
      setMsg(`扫描失败: ${e}`);
    }
    setScanning(false);
  };

  const doImportSkills = async () => {
    const selected = foundSkills.filter(s => s.selected);
    if (selected.length === 0) { setMsg("请选择要导入的技能"); setTimeout(() => setMsg(""), 3000); return; }
    const newExperts: Expert[] = selected.map(({ selected: _, ...e }) => e as Expert);
    await persist([...newExperts, ...experts]);
    setMsg(`已导入 ${newExperts.length} 个: ${newExperts.map(e => e.name).join("、")}`);
    setShowImport(false);
    setTimeout(() => setMsg(""), 4000);
  };

  // ── 预置 ──
  const importPresets = () => {
    let count = 0;
    for (const a of DEFAULT_DISCUSSION_AGENTS) {
      if (experts.some(e => e.sourcePersona === a.name)) continue;
      persist([{
        id: `expert-pre-${Date.now()}-${count}`,
        name: a.name, description: `预设评审: ${a.perspective}`,
        sourcePersona: a.name, modelId: a.model,
        perspective: a.perspective, defaultPrompt: a.prompt,
        createdAt: new Date().toISOString().split("T")[0],
      }, ...experts]);
      count++;
    }
    setMsg(`导入 ${count} 个预置评审`);
    setTimeout(() => setMsg(""), 3000);
  };

  // ── 蒸馏 ──
  const startDistill = async () => {
    const persona = distillPersona.trim();
    if (!persona) return;
    setDistillRunning(true);
    setPhases([]);

    const unlistenFn = await listen<PhaseEvent>("distill-phase", (evt) => {
      setPhases(prev => {
        const i = prev.findIndex(p => p.phase === evt.payload.phase);
        if (i >= 0) { const u = [...prev]; u[i] = evt.payload; return u; }
        return [...prev, evt.payload];
      });
    });
    unlistenRef.current = unlistenFn;

    try {
      const result = await distillExpert(persona, distillModel || null);
      const ne: Expert = {
        id: result.id, name: result.name, description: result.description,
        sourcePersona: result.source_persona, modelId: result.model_id,
        perspective: result.perspective, defaultPrompt: result.default_prompt,
        createdAt: result.created_at, skillPath: result.skill_path, skillSummary: result.skill_summary,
      };
      await persist([ne, ...experts]);
      setDistillPersona("");
      setShowDistill(false);
      setMsg(`已创建「${ne.name}」`);
    } catch (e: any) {
      setPhases(prev => [...prev, { phase: "错误", status: "error", message: `蒸馏失败: ${e}`, detail: "" }]);
    } finally {
      setDistillRunning(false);
      if (unlistenRef.current) { unlistenRef.current(); unlistenRef.current = null; }
    }
    setTimeout(() => setMsg(""), 4000);
  };

  if (loading) {
    return (
      <div className="view-container">
        <div className="view-header"><h2>专家库</h2></div>
        <div className="empty-state"><div className="empty-state-text">加载中...</div></div>
      </div>
    );
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>专家库</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
          PenSoul 技能 · 本地导入 · 智能蒸馏
        </span>
      </div>

      {/* 操作栏 */}
      <div style={{ display: "flex", gap: "var(--space-sm)", flexWrap: "wrap", marginBottom: "var(--space-xl)" }}>
        <button className="btn btn-primary" onClick={() => { setShowDistill(true); setPhases([]); setDistillPersona(""); }}>
          <Sparkles size={15} /> 新增专家
        </button>
        <button className="btn btn-secondary" onClick={openImportPanel}>
          <FolderOpen size={15} /> 从本地导入
        </button>
        <button className="btn btn-secondary" onClick={importPresets}>
          <Bot size={15} /> 导入预置评审
        </button>
        <button className="btn btn-accent" onClick={() => { persist(experts); setMsg("已保存"); setTimeout(() => setMsg(""), 2000); }}
          style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}>
          <Save size={13} /> 保存
        </button>
        {msg && (
          <span style={{ fontSize: "var(--text-xs)", padding: "6px 12px", borderRadius: "var(--radius-sm)", background: "var(--color-jade-wash)", color: "var(--color-jade)" }}>{msg}</span>
        )}
      </div>

      {/* 导入面板 */}
      {showImport && (
        <div style={{ background: "var(--color-paper)", border: "1px solid var(--color-accent)", borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)", marginBottom: "var(--space-xl)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)" }}>
            <FolderOpen size={18} style={{ color: "var(--color-accent)" }} />
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
              从本地 Experts 文件夹导入
            </span>
          </div>

          {scanning ? (
            <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "var(--space-md)", color: "var(--color-ink-3)", fontSize: "var(--text-sm)" }}>
              <Loader2 size={16} className="spinning" /> 正在扫描 Experts 文件夹...
            </div>
          ) : foundSkills.length === 0 ? (
            <div style={{ padding: "var(--space-md)", color: "var(--color-ink-3)", fontSize: "var(--text-sm)" }}>
              未在 Experts 文件夹中发现可导入的技能
            </div>
          ) : (
            <>
              <div style={{ marginBottom: "var(--space-sm)", fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
                发现以下技能，勾选需要导入的：
              </div>
              {foundSkills.map((s, i) => (
                <label key={i} style={{
                  display: "flex", alignItems: "center", gap: "var(--space-sm)",
                  padding: "var(--space-sm) var(--space-md)", cursor: "pointer",
                  background: s.selected ? "var(--color-accent-wash)" : "transparent",
                  borderRadius: "var(--radius-sm)", marginBottom: 4,
                }}>
                  <input type="checkbox" checked={s.selected}
                    onChange={() => setFoundSkills(prev => { const u = [...prev]; u[i] = { ...u[i], selected: !u[i].selected }; return u; })} />
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: "var(--text-sm)", fontWeight: 500 }}>{s.name}</div>
                    <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{s.description}</div>
                  </div>
                  {s.skillPath && <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-jade)" }}>已有文件</span>}
                </label>
              ))}
              <div style={{ display: "flex", gap: "var(--space-sm)", marginTop: "var(--space-md)" }}>
                <button className="btn btn-primary" onClick={doImportSkills}
                  disabled={!foundSkills.some(s => s.selected)}>
                  导入选中 ({foundSkills.filter(s => s.selected).length})
                </button>
                <button className="btn btn-secondary" onClick={() => setShowImport(false)}>取消</button>
              </div>
            </>
          )}
        </div>
      )}

      {/* 蒸馏面板 */}
      {showDistill && (
        <div style={{ background: "var(--color-paper)", border: "1px solid var(--color-accent)", borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)", marginBottom: "var(--space-xl)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)" }}>
            <Sparkles size={18} style={{ color: "var(--color-accent)" }} />
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
              PenSoul 技能蒸馏
            </span>
          </div>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginBottom: "var(--space-sm)" }}>
            输入人名或主题，选择模型，AI 将自动分析并生成可复用的技能卡，保存到 Experts 文件夹
          </div>
          <div style={{ display: "flex", gap: "var(--space-sm)", alignItems: "center" }}>
            <input className="pm-input" type="text" style={{ flex: 1 }}
              placeholder="如：鲁迅、村上春树、王小波..."
              value={distillPersona} onChange={e => setDistillPersona(e.target.value)}
              disabled={distillRunning} />
            <select className="pm-input" style={{ width: 200 }}
              value={distillModel} onChange={e => setDistillModel(e.target.value)}
              disabled={distillRunning}>
              {models.length === 0 && <option value="gpt-4o">GPT-4o</option>}
              {models.map(m => (
                <option key={m.model_id} value={m.model_id} disabled={!m.is_available}>
                  {m.display_name} {!m.is_available ? "(未配置)" : ""}
                </option>
              ))}
            </select>
            <button className="btn btn-primary" onClick={startDistill}
              disabled={distillRunning || !distillPersona.trim()}>
              {distillRunning ? <><Loader2 size={14} className="spinning" /> 蒸馏中...</> : "开始蒸馏"}
            </button>
            {!distillRunning && (
              <button className="btn btn-secondary" onClick={() => setShowDistill(false)}>取消</button>
            )}
          </div>

          {phases.length > 0 && (
            <div style={{ marginTop: "var(--space-lg)", display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginBottom: 2 }}>进度：</div>
              {phases.map((p, i) => (
                <div key={i} style={{
                  display: "flex", gap: "var(--space-sm)", alignItems: "flex-start",
                  padding: "var(--space-sm) var(--space-md)",
                  background: p.status === "error" ? "var(--color-error-wash)" : p.status === "done" ? "var(--color-jade-wash)" : "var(--color-subtle-bg)",
                  borderRadius: "var(--radius-sm)",
                  border: `1px solid ${p.status === "error" ? "var(--color-error)" : p.status === "done" ? "var(--color-jade)" : "var(--color-rule-light)"}`,
                }}>
                  <div style={{ flexShrink: 0, marginTop: 2 }}>
                    {p.status === "running" && <Loader2 size={14} className="spinning" style={{ color: "var(--color-accent)" }} />}
                    {p.status === "done" && <CheckCircle2 size={14} style={{ color: "var(--color-jade)" }} />}
                    {p.status === "error" && <XCircle size={14} style={{ color: "var(--color-error)" }} />}
                  </div>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <span style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: p.status === "error" ? "var(--color-error)" : p.status === "done" ? "var(--color-jade)" : "var(--color-ink)" }}>
                        {p.phase}
                      </span>
                      <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                        {p.status === "running" ? "..." : p.status === "done" ? "完成" : "出错"}
                      </span>
                    </div>
                    <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginTop: 2 }}>{p.message}</div>
                    {p.detail && p.status === "done" && (
                      <details style={{ marginTop: 4 }}>
                        <summary style={{ fontSize: "var(--text-2xs)", color: "var(--color-accent)", cursor: "pointer" }}>详情</summary>
                        <pre style={{ fontSize: "var(--text-2xs)", marginTop: 4, padding: "var(--space-sm)", background: "var(--color-subtle-bg)", borderRadius: "var(--radius-xs)", whiteSpace: "pre-wrap", maxHeight: 200, overflow: "auto" }}>{p.detail}</pre>
                      </details>
                    )}
                  </div>
                </div>
              ))}
              {!distillRunning && (
                <div style={{ textAlign: "center", padding: "var(--space-md)", fontSize: "var(--text-sm)", color: "var(--color-jade)" }}>
                  <CheckCircle2 size={16} style={{ verticalAlign: "middle", marginRight: 6 }} />
                  蒸馏完成！技能已保存到 Experts 文件夹
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* 编辑列表 */}
      {experts.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">专</div>
          <div className="empty-state-text">专家库为空</div>
          <div className="empty-state-sub">从本地导入或使用蒸馏在线生成</div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
          {experts.map(ex => editingId === ex.id ? (
            // 行内编辑
            <div key={ex.id} style={{
              background: "var(--color-paper)", border: "1px solid var(--color-accent)",
              borderRadius: "var(--radius-md)", padding: "var(--space-md) var(--space-lg)",
            }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-sm)" }}>
                <div><label className="pm-label">名称</label>
                  <input className="pm-input" value={editForm.name} onChange={e => setEditForm(p => ({ ...p, name: e.target.value }))} /></div>
                <div><label className="pm-label">人物</label>
                  <input className="pm-input" value={editForm.sourcePersona} onChange={e => setEditForm(p => ({ ...p, sourcePersona: e.target.value }))} /></div>
                <div><label className="pm-label">维度</label>
                  <input className="pm-input" value={editForm.perspective} onChange={e => setEditForm(p => ({ ...p, perspective: e.target.value }))} /></div>
                <div><label className="pm-label">模型</label>
                  <select className="pm-input" value={editForm.modelId} onChange={e => setEditForm(p => ({ ...p, modelId: e.target.value }))}>
                    {models.map(m => <option key={m.model_id} value={m.model_id}>{m.display_name}</option>)}
                    {models.length === 0 && <option value="gpt-4o">GPT-4o</option>}
                  </select></div>
              </div>
              <div style={{ marginTop: "var(--space-sm)" }}>
                <label className="pm-label">描述</label>
                <textarea className="pm-textarea" rows={2} value={editForm.description} onChange={e => setEditForm(p => ({ ...p, description: e.target.value }))} />
              </div>
              <div style={{ marginTop: "var(--space-sm)" }}>
                <label className="pm-label">评审提示词</label>
                <textarea className="pm-textarea" rows={2} value={editForm.defaultPrompt} onChange={e => setEditForm(p => ({ ...p, defaultPrompt: e.target.value }))} />
              </div>
              <div style={{ display: "flex", gap: "var(--space-sm)", marginTop: "var(--space-md)" }}>
                <button className="btn btn-primary" onClick={handleSaveEdit} disabled={!editForm.name.trim()}>保存</button>
                <button className="btn btn-secondary" onClick={() => setEditingId(null)}>取消</button>
              </div>
            </div>
          ) : (
            <div key={ex.id} style={{ background: "var(--color-paper)", border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-md)", padding: "var(--space-md) var(--space-lg)" }}>
              <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-md)" }}>
                <div style={{ width: 40, height: 40, borderRadius: "var(--radius-sm)", background: "var(--color-accent-wash)", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                  <Bot size={20} style={{ color: "var(--color-accent)" }} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
                    <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "1px", color: "var(--color-ink)" }}>{ex.name}</span>
                    <span style={{ fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: "var(--radius-xs)", background: "var(--color-jade-wash)", color: "var(--color-jade)" }}>{ex.modelId}</span>
                    {ex.skillPath && <span style={{ fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: "var(--radius-xs)", background: "var(--color-indigo-wash)", color: "var(--color-indigo)" }}>技能</span>}
                  </div>
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 4 }}>{ex.description}</div>
                  <div style={{ display: "flex", gap: "var(--space-md)", marginTop: 6, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                    <span><User size={11} style={{ verticalAlign: "middle" }} /> {ex.sourcePersona}</span>
                    <span><Cpu size={11} style={{ verticalAlign: "middle" }} /> {ex.perspective}</span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
                  <button className="pv-icon-btn" onClick={() => handleEdit(ex)} title="编辑"><Edit3 size={14} /></button>
                  <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDelete(ex.id)} title="删除"><Trash2 size={14} /></button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
