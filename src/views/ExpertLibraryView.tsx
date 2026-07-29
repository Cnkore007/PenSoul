import { useState, useEffect, useCallback } from "react";
import type { Expert, LlmModel } from "../types";
import { saveExperts, loadExperts, listModels, scanNuwaSkills } from "../ipc";
import { DEFAULT_DISCUSSION_AGENTS } from "../types";
import {
  Bot, Plus, Trash2, Edit3,
  Sparkles, Lightbulb, Cpu, User,
  Save, Upload,
} from "lucide-react";

export function ExpertLibraryView() {
  const [experts, setExperts] = useState<Expert[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState({
    name: "",
    description: "",
    sourcePersona: "",
    modelId: "gpt-4o",
    perspective: "",
    defaultPrompt: "",
  });
  const [importMsg, setImportMsg] = useState("");
  const [loading, setLoading] = useState(true);
  const [backendSynced, setBackendSynced] = useState<boolean | null>(null);

  // 从后端加载专家和模型数据
  useEffect(() => {
    Promise.all([
      loadExperts().catch(() => []),
      listModels().catch(() => []),
    ]).then(([rawExperts, rawModels]) => {
      const mapped: Expert[] = rawExperts.map((e: any) => ({
        id: e.id,
        name: e.name,
        description: e.description,
        sourcePersona: e.source_persona,
        modelId: e.model_id,
        perspective: e.perspective,
        defaultPrompt: e.default_prompt,
        createdAt: e.created_at,
        skillPath: e.skill_path,
        skillSummary: e.skill_summary,
      }));
      setExperts(mapped);
      setModels(rawModels);
      setLoading(false);
    });
  }, []);

  const persist = useCallback(async (updated: Expert[]) => {
    setExperts(updated);
    const payload = updated.map(e => ({
      id: e.id,
      name: e.name,
      description: e.description,
      source_persona: e.sourcePersona,
      model_id: e.modelId,
      perspective: e.perspective,
      default_prompt: e.defaultPrompt,
      created_at: e.createdAt,
      skill_path: e.skillPath || null,
      skill_summary: e.skillSummary || null,
    }));
    try {
      await saveExperts(payload);
    } catch (e) {
      console.error("保存专家失败:", e);
    }
  }, []);

  // 同步到后端
  const syncToEngine = useCallback(async () => {
    const payload = experts.map(e => ({
      id: e.id,
      name: e.name,
      description: e.description,
      source_persona: e.sourcePersona,
      model_id: e.modelId,
      perspective: e.perspective,
      default_prompt: e.defaultPrompt,
      created_at: e.createdAt,
      skill_path: e.skillPath || null,
      skill_summary: e.skillSummary || null,
    }));
    try {
      await saveExperts(payload);
      setBackendSynced(true);
    } catch {
      setBackendSynced(false);
    }
    setTimeout(() => setBackendSynced(null), 3000);
  }, [experts]);

  // 从后端加载
  const loadFromEngine = useCallback(async () => {
    try {
      const loaded = await loadExperts();
      if (loaded) {
        const mapped: Expert[] = loaded.map((e: any) => ({
          id: e.id,
          name: e.name,
          description: e.description,
          sourcePersona: e.source_persona,
          modelId: e.model_id,
          perspective: e.perspective,
          defaultPrompt: e.default_prompt,
          createdAt: e.created_at,
          skillPath: e.skill_path,
          skillSummary: e.skill_summary,
        }));
        setExperts(mapped);
        setBackendSynced(true);
      }
    } catch {
      setBackendSynced(false);
    }
    setTimeout(() => setBackendSynced(null), 3000);
  }, []);

  const resetForm = () => {
    setForm({ name: "", description: "", sourcePersona: "", modelId: "gpt-4o", perspective: "", defaultPrompt: "" });
    setEditingId(null);
    setShowForm(false);
  };

  const handleSave = () => {
    if (!form.name.trim()) return;
    const now = new Date().toISOString().split("T")[0];
    if (editingId) {
      persist(experts.map(e => e.id === editingId ? { ...e, ...form, createdAt: e.createdAt } : e));
    } else {
      const newExpert: Expert = {
        id: `expert-${Date.now()}`,
        ...form,
        createdAt: now,
      };
      persist([newExpert, ...experts]);
    }
    resetForm();
  };

  const handleEdit = (e: Expert) => {
    setForm({
      name: e.name,
      description: e.description,
      sourcePersona: e.sourcePersona,
      modelId: e.modelId,
      perspective: e.perspective,
      defaultPrompt: e.defaultPrompt,
    });
    setEditingId(e.id);
    setShowForm(true);
  };

  const handleDelete = (id: string) => {
    persist(experts.filter(e => e.id !== id));
  };

  // 从女娲技能导入：扫描本地 skill 目录，导入 perspective 专家
  const importFromNuwa = async () => {
    try {
      setImportMsg("正在扫描女娲技能...");
      const rawExperts = await scanNuwaSkills();
      if (!rawExperts || rawExperts.length === 0) {
        setImportMsg("未发现可导入的女娲 perspective 技能");
        setTimeout(() => setImportMsg(""), 3000);
        return;
      }
      // 转换为前端格式
      const imported: Expert[] = rawExperts.map((e: any) => ({
        id: e.id,
        name: e.name,
        description: e.description,
        sourcePersona: e.source_persona,
        modelId: e.model_id,
        perspective: e.perspective,
        defaultPrompt: e.default_prompt,
        createdAt: e.created_at,
        skillPath: e.skill_path,
        skillSummary: e.skill_summary,
      }));
      // 去重：已通过 skill_path 导入过的不重复添加
      const existingPaths = new Set(experts.filter(e => e.skillPath).map(e => e.skillPath));
      const newOnes = imported.filter(e => !e.skillPath || !existingPaths.has(e.skillPath));
      if (newOnes.length === 0) {
        setImportMsg("所有女娲技能已导入过");
        setTimeout(() => setImportMsg(""), 3000);
        return;
      }
      const updated = [...newOnes, ...experts];
      await persist(updated);
      setImportMsg(`已导入 ${newOnes.length} 个女娲蒸馏专家`);
      setTimeout(() => setImportMsg(""), 3000);
    } catch (e) {
      console.error("女娲技能导入失败:", e);
      setImportMsg(`导入失败: ${e}`);
      setTimeout(() => setImportMsg(""), 3000);
    }
  };

  // 预置灵感 — 从 DEFAULT_DISCUSSION_AGENTS 创建专家
  const importFromPresets = () => {
    let imported = 0;
    for (const agent of DEFAULT_DISCUSSION_AGENTS) {
      if (experts.some(e => e.sourcePersona === agent.name)) continue;
      const newExpert: Expert = {
        id: `expert-${Date.now()}-pre-${imported}`,
        name: agent.name,
        description: `预设讨论 Agent：${agent.perspective}维度的评审专家`,
        sourcePersona: agent.name,
        modelId: agent.model,
        perspective: agent.perspective,
        defaultPrompt: agent.prompt,
        createdAt: new Date().toISOString().split("T")[0],
      };
      persist([newExpert, ...experts]);
      imported++;
    }
    setImportMsg(`已导入 ${imported} 个预置评审专家`);
    setTimeout(() => setImportMsg(""), 3000);
  };

  if (loading) {
    return (
      <div className="view-container">
        <div className="view-header"><h2>专家库</h2></div>
        <div className="empty-state">
          <div className="empty-state-text">加载中...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>专家库</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px" }}>
          女娲蒸馏 · Agent 配置 · 多维度评审
        </span>
      </div>

      {/* 操作栏 */}
      <div style={{
        display: "flex", gap: "var(--space-sm)", flexWrap: "wrap",
        marginBottom: "var(--space-xl)",
      }}>
        <button className="btn btn-primary" onClick={() => { resetForm(); setShowForm(true); }}>
          <Plus size={15} /> 新建专家
        </button>
        <button className="btn btn-accent" onClick={importFromNuwa}>
          <Sparkles size={15} /> 从女娲导入
        </button>
        <button className="btn btn-secondary" onClick={importFromPresets}>
          <Lightbulb size={15} /> 导入预置评审
        </button>
        <button className="btn btn-accent" onClick={syncToEngine} style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}>
          <Save size={13} /> 同步到引擎
        </button>
        <button className="btn btn-secondary" onClick={loadFromEngine} style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}>
          <Upload size={13} /> 从引擎加载
        </button>
        {backendSynced !== null && (
          <span style={{ fontSize: "var(--text-xs)", padding: "2px 8px", borderRadius: "var(--radius-sm)", background: backendSynced ? "var(--color-jade-wash)" : "var(--color-error-wash)", color: backendSynced ? "var(--color-jade)" : "var(--color-error)" }}>
            {backendSynced ? "已同步" : "失败"}
          </span>
        )}
        {importMsg && (
          <span style={{
            fontSize: "var(--text-xs)", padding: "6px 12px", borderRadius: "var(--radius-sm)",
            background: "var(--color-jade-wash)", color: "var(--color-jade)",
            display: "flex", alignItems: "center",
          }}>
            {importMsg}
          </span>
        )}
      </div>

      {/* 新建/编辑表单 */}
      {showForm && (
        <div style={{
          background: "var(--color-paper)", border: "1px solid var(--color-accent)",
          borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)",
          marginBottom: "var(--space-xl)", boxShadow: "var(--shadow-sm)",
        }}>
          <div style={{
            display: "flex", alignItems: "center", gap: 8,
            marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)",
            borderBottom: "1px solid var(--color-rule-light)",
          }}>
            <Bot size={18} style={{ color: "var(--color-accent)" }} />
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
              {editingId ? "编辑专家" : "新建专家"}
            </span>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-md)" }}>
            <div>
              <label className="pm-label">专家名称</label>
              <input className="pm-input" type="text" placeholder="例：鲁迅文学评论家"
                value={form.name} onChange={e => setForm(p => ({ ...p, name: e.target.value }))} />
            </div>
            <div>
              <label className="pm-label">来源人物</label>
              <input className="pm-input" type="text" placeholder="例：鲁迅"
                value={form.sourcePersona} onChange={e => setForm(p => ({ ...p, sourcePersona: e.target.value }))} />
            </div>
            <div>
              <label className="pm-label">评审维度</label>
              <input className="pm-input" type="text" placeholder="例：文学批评、社会洞察"
                value={form.perspective} onChange={e => setForm(p => ({ ...p, perspective: e.target.value }))} />
            </div>
            <div>
              <label className="pm-label">LLM 模型</label>
              <select className="pm-input" value={form.modelId}
                onChange={e => setForm(p => ({ ...p, modelId: e.target.value }))}>
                {models.map(m => (
                  <option key={m.model_id} value={m.model_id}>
                    {m.display_name} ({m.model_id})
                  </option>
                ))}
                {models.length === 0 && (
                  <>
                    <option value="gpt-4o">GPT-4o</option>
                    <option value="gpt-4o-mini">GPT-4o Mini</option>
                    <option value="claude-sonnet-4-20250514">Claude Sonnet 4</option>
                    <option value="deepseek-chat">DeepSeek V3</option>
                    <option value="qwen-2.5-72b">Qwen 2.5 72B</option>
                  </>
                )}
              </select>
            </div>
          </div>
          <div style={{ marginTop: "var(--space-md)" }}>
            <label className="pm-label">专家描述</label>
            <textarea className="pm-textarea" rows={2} placeholder="简短描述这位专家的专长和特点"
              value={form.description} onChange={e => setForm(p => ({ ...p, description: e.target.value }))} />
          </div>
          <div style={{ marginTop: "var(--space-md)" }}>
            <label className="pm-label">默认评审提示词</label>
            <textarea className="pm-textarea" rows={3}
              placeholder="这位专家在评审构思时使用的提示词模板..."
              value={form.defaultPrompt} onChange={e => setForm(p => ({ ...p, defaultPrompt: e.target.value }))} />
          </div>
          <div style={{ display: "flex", gap: "var(--space-sm)", marginTop: "var(--space-md)" }}>
            <button className="btn btn-primary" onClick={handleSave} disabled={!form.name.trim()}>
              {editingId ? "保存修改" : "创建专家"}
            </button>
            <button className="btn btn-secondary" onClick={resetForm}>取消</button>
          </div>
        </div>
      )}

      {/* 专家列表 */}
      {experts.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">专</div>
          <div className="empty-state-text">专家库为空</div>
          <div className="empty-state-sub">
            点击「从女娲导入」读取蒸馏技能，或手动创建专家
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
          {experts.map(ex => (
            <div key={ex.id} style={{
              background: "var(--color-paper)", border: "1px solid var(--color-rule-light)",
              borderRadius: "var(--radius-md)", padding: "var(--space-md) var(--space-lg)",
              boxShadow: "var(--shadow-subtle)",
            }}>
              <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-md)" }}>
                <div style={{
                  width: 40, height: 40, borderRadius: "var(--radius-sm)",
                  background: "var(--color-accent-wash)", display: "flex",
                  alignItems: "center", justifyContent: "center", flexShrink: 0,
                }}>
                  <Bot size={20} style={{ color: "var(--color-accent)" }} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
                    <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "1px", color: "var(--color-ink)" }}>
                      {ex.name}
                    </span>
                    <span style={{
                      fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: "var(--radius-xs)",
                      background: "var(--color-jade-wash)", color: "var(--color-jade)",
                    }}>
                      {ex.modelId}
                    </span>
                    {ex.skillPath && (
                      <span style={{
                        fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: "var(--radius-xs)",
                        background: "var(--color-indigo-wash)", color: "var(--color-indigo)",
                      }}>
                        女娲
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 4, letterSpacing: "0.3px" }}>
                    {ex.description}
                  </div>
                  <div style={{ display: "flex", gap: "var(--space-md)", marginTop: 6, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                    <span><User size={11} style={{ verticalAlign: "middle" }} /> {ex.sourcePersona}</span>
                    <span><Bot size={11} style={{ verticalAlign: "middle" }} /> {ex.perspective}</span>
                    <span><Cpu size={11} style={{ verticalAlign: "middle" }} /> {ex.modelId}</span>
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
