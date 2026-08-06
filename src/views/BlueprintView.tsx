import { useState, useEffect } from "react";
import type { CSSProperties, ReactNode } from "react";
import type {
  BlueprintReport,
  BookBlueprint,
  CharacterMatrixEntry,
  Commitment,
  EntityDossier,
  ProjectData,
  ResourceEntry,
  Subplot,
  VolumeBlueprint,
} from "../types";
import { settleBlueprint, settleBlueprintWithModules, checkBlueprint, saveBlueprint, listModules } from "../ipc";
import { importBookForContinuation } from "../ipc";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import {
  BookOpen, AlertTriangle, CheckCircle2, RefreshCw, Loader2,
  Plus, Trash2, Edit3, Check, Save, Star, Layers, Upload,
} from "lucide-react";
import type { StoryModule } from "../types";
import {
  commitmentKindLabel, commitmentStatusLabel, entityTypeLabel,
  foreshadowStatusLabel, functionLabel, resourceStatusLabel, roleLabel,
  subplotStatusLabel, translateIssue, reportSummary, volumeStatusLabel,
} from "./blueprintText";

interface BlueprintViewProps {
  projectData: ProjectData;
  onRefresh?: () => void;
}

// 行内编辑字段定义
interface FieldSpec {
  key: string;
  label: string;
  kind?: "text" | "textarea" | "number" | "boolean" | "select" | "json";
  options?: string[];
  optionLabels?: Record<string, string>;
  placeholder?: string;
}

// 一稿手动测试页：开书定盘 + 账本浏览 + 行内编辑/增删 + 确定性检查
export function BlueprintView({ projectData, onRefresh }: BlueprintViewProps) {
  const bp = projectData.blueprint;
  const [draft, setDraft] = useState<BookBlueprint>(bp);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<BlueprintReport | null>(null);
  const [message, setMessage] = useState("");
  const [editingKey, setEditingKey] = useState<string | null>(null);
  // 参考模块（开书定盘时勾选注入，灵感库不是正典）
  const [showModulePicker, setShowModulePicker] = useState(false);
  const [modules, setModules] = useState<StoryModule[]>([]);
  const [selectedModuleIds, setSelectedModuleIds] = useState<Set<string>>(new Set());
  // 讨论成果过期检测：蓝图来源指纹 vs 当前讨论成果摘要
  const syn = projectData.sprout?.lastDiscussion?.synthesis;
  const stale = !!(
    bp.settled &&
    bp.source_stamp &&
    syn &&
    calcStamp(syn) !== bp.source_stamp
  );

  // 外部刷新（定盘/保存成功）后同步草稿
  useEffect(() => {
    setDraft(bp);
  }, [bp]);

  async function doSettle(referenceModules: StoryModule[]) {
    setBusy(true);
    setMessage("");
    try {
      const result: BookBlueprint = referenceModules.length > 0
        ? await settleBlueprintWithModules(referenceModules)
        : await settleBlueprint();
      setDraft(result);
      setReport(null);
      setMessage(
        `定盘完成：${result.commitments.length} 条承诺、${result.volumes.length} 个卷蓝图、${result.character_matrix.length} 个人物、${result.foreshadows.length} 条伏笔、${result.subplots.length} 条副线` +
        (referenceModules.length > 0 ? `（参考 ${referenceModules.length} 个模块）` : ""),
      );
      onRefresh?.();
    } catch (e) {
      setMessage(`定盘失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function handleSettle() {
    await doSettle([]);
  }

  async function openModulePicker() {
    setShowModulePicker(true);
    if (modules.length === 0) {
      setModules(await listModules().catch(() => []));
    }
  }

  async function handleSettleWithModules() {
    const picked = modules.filter(m => selectedModuleIds.has(m.module_id));
    await doSettle(picked);
  }

  // 导入已有正文 → 反推蓝图 → 继续扩写（半成品续写）
  async function handleImportContinuation() {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "小说正文", extensions: ["txt", "md", "markdown", "epub", "pdf"] }],
    });
    if (typeof selected !== "string" || !selected) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await importBookForContinuation(selected);
      setDraft(result);
      setReport(null);
      setMessage(`正文已导入并反推蓝图：${result.volumes.length} 个卷、${result.character_matrix.length} 个人物、${result.foreshadows.length} 条伏笔。可在下方修正后继续扩写。`);
      onRefresh?.();
    } catch (e) {
      setMessage(`导入失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  async function handleCheck() {
    setBusy(true);
    try {
      const r = await checkBlueprint();
      setReport(r);
    } catch (e) {
      setMessage(`检查失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  // 保存用户编辑后的蓝图（草稿 → 后端落盘）
  async function handleSave() {
    setBusy(true);
    setMessage("");
    try {
      await saveBlueprint(draft);
      setEditingKey(null);
      setMessage("蓝图修改已保存");
      onRefresh?.();
    } catch (e) {
      setMessage(`保存失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  // 通用：更新草稿里某个账本数组
  function patchLedger<K extends keyof BookBlueprint>(
    ledger: K,
    updater: (items: BookBlueprint[K]) => BookBlueprint[K],
  ) {
    setDraft(prev => ({ ...prev, [ledger]: updater(prev[ledger]) }));
  }

  function patchItem(
    ledger: keyof BookBlueprint,
    idKey: string,
    id: string,
    patch: Record<string, unknown>,
  ) {
    patchLedger(ledger as "commitments", items =>
      (items as any[]).map(it => (String(it[idKey]) === id ? { ...it, ...patch } : it)) as any);
  }

  function removeItem(ledger: keyof BookBlueprint, idKey: string, id: string) {
    patchLedger(ledger as "commitments", items =>
      (items as any[]).filter(it => String(it[idKey]) !== id) as any);
  }

  function addItem(ledger: keyof BookBlueprint, empty: unknown) {
    patchLedger(ledger as "commitments", items => [...(items as any[]), empty] as any);
  }

  // 给某卷添加一个空白节奏点（打开编辑表单让用户填写）
  function addVolumeBeat(volumeNo: number) {
    setDraft(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_no === volumeNo ? {
        ...v,
        beats: [
          ...(v.beats ?? []),
          { beat_id: `bt-${Date.now()}`, beat_type: "buildup", chapter: 0, note: "", links: [] },
        ],
      } : v),
    }));
  }

  function startEdit(ledger: string, id: string) {
    setEditingKey(`${ledger}:${id}`);
  }

  function closeEdit() {
    setEditingKey(null);
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>开书定盘</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px" }}>
          讨论收敛后，把成果熔炼为全书蓝图：承诺、分卷、人物、伏笔、副线、资源与实体档案 · 账本可直接修改、增删并保存
        </span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="btn btn-secondary" onClick={handleImportContinuation} disabled={busy}
            title="导入已写好的小说正文（txt/md/epub/pdf），自动反推蓝图后继续扩写">
            {busy ? <Loader2 size={15} className="spin" /> : <Upload size={15} />} 导入正文续写
          </button>
          <button className="btn btn-secondary" onClick={openModulePicker} disabled={busy || !bp.settled}
            title="勾选蒸馏书库中沉淀的钩子/开场/爽点等模块，重新定盘时注入蓝图转换">
            <Layers size={15} /> 参考模块{selectedModuleIds.size > 0 ? `（${selectedModuleIds.size}）` : ""}
          </button>
          <button className="btn btn-secondary" onClick={handleCheck} disabled={busy || !bp.settled}>
            {busy && !report ? <Loader2 size={15} className="spin" /> : <RefreshCw size={15} />} 运行检查
          </button>
          <button className="btn btn-secondary" onClick={handleSettle} disabled={busy}>
            {busy && !report ? <Loader2 size={15} className="spin" /> : <BookOpen size={15} />} 重新定盘
          </button>
          <button className="btn btn-primary" onClick={handleSave} disabled={busy || !bp.settled}>
            <Save size={15} /> 保存修改
          </button>
        </div>
      </div>

      {/* ── 参考模块选择面板（灵感库） ── */}
      {showModulePicker && (
        <div className="card" style={{ padding: "12px 16px", marginBottom: 16 }}>
          <div className="card-header" style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
            <Layers size={14} style={{ verticalAlign: -2, marginRight: 6 }} />
            参考模块（勾选 1-5 个，重新定盘时注入蓝图转换；模块只是灵感，禁止照搬案例）
          </div>
          {modules.length === 0 ? (
            <div style={{ fontSize: 12, color: "var(--color-ink-3)" }}>
              模块库为空——先在「工作流 → 写作技能库 → 模块库」蒸馏一本书，模块会自动从结构/张力卡投影出来。
            </div>
          ) : (
            <>
              <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 280, overflow: "auto" }}>
                {modules.map(m => (
                  <label key={m.module_id} style={{
                    display: "flex", alignItems: "flex-start", gap: 8, cursor: "pointer",
                    padding: "6px 8px", borderRadius: "var(--radius-sm)",
                    background: selectedModuleIds.has(m.module_id) ? "var(--color-accent-wash)" : "var(--color-bg)",
                  }}>
                    <input
                      type="checkbox"
                      checked={selectedModuleIds.has(m.module_id)}
                      style={{ marginTop: 2 }}
                      onChange={e => {
                        setSelectedModuleIds(prev => {
                          const next = new Set(prev);
                          if (e.target.checked) {
                            if (next.size >= 5) return prev;
                            next.add(m.module_id);
                          } else {
                            next.delete(m.module_id);
                          }
                          return next;
                        });
                      }}
                    />
                    <div style={{ fontSize: 12, lineHeight: 1.6 }}>
                      <span style={{ fontWeight: 600 }}>{m.name}</span>
                      <span style={{ color: "var(--color-ink-3)", marginLeft: 6 }}>《{m.source_book}》· {m.module_type}</span>
                      <div style={{ color: "var(--color-ink-2)" }}>{m.example}</div>
                    </div>
                  </label>
                ))}
              </div>
              <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                <button className="btn btn-primary" style={{ padding: "4px 14px", fontSize: 12 }}
                  onClick={handleSettleWithModules} disabled={busy || selectedModuleIds.size === 0}>
                  <Star size={13} /> 用所选模块定盘
                </button>
                <button className="btn btn-secondary" style={{ padding: "4px 10px", fontSize: 12 }}
                  onClick={() => setShowModulePicker(false)}>
                  收起
                </button>
              </div>
            </>
          )}
        </div>
      )}

      {message && (
        <div className="card" style={{ padding: "10px 14px", marginBottom: 16, fontSize: "var(--text-xs)" }}>
          {message}
        </div>
      )}
      {stale && (
        <div className="card" style={{ padding: 12, marginBottom: 16, borderColor: "var(--color-warning)", color: "#b45309" }}>
          讨论成果已更新，当前蓝图来自旧讨论（{bp.settled_at}）。请点击「重新定盘」刷新蓝图。
        </div>
      )}

      {!bp.settled ? (
        <div className="card" style={{ padding: 24, textAlign: "center" }}>
          <p>尚未定盘。请先在「灵魂萌芽」完成一次多 Agent 讨论与成果提炼，再回来点击「开书定盘」。</p>
        </div>
      ) : (
        <>
          <Overview bp={draft} />
          <CheckReport report={report} bp={draft} />
          <Commitments
            items={draft.commitments}
            editingKey={editingKey}
            onEdit={id => startEdit("commitments", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("commitments", "commitment_id", id, patch)}
            onDelete={id => { removeItem("commitments", "commitment_id", id); setEditingKey(null); }}
            onAdd={() => {
              const empty: Commitment = {
                commitment_id: `cmt-${Date.now()}`,
                statement: "", kind: "rule", priority: 2, scope: "book",
                resolution_chapter: null, ongoing: true, status: "active", sources: [],
              };
              addItem("commitments", empty);
              startEdit("commitments", empty.commitment_id);
            }}
          />
          <Volumes
            items={draft.volumes}
            editingKey={editingKey}
            onEdit={id => startEdit("volumes", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("volumes", "volume_no", id, patch)}
            onDelete={id => { removeItem("volumes", "volume_no", id); setEditingKey(null); }}
            onAddBeat={addVolumeBeat}
            onAdd={() => {
              const empty: VolumeBlueprint = {
                volume_no: draft.volumes.reduce((m, v) => Math.max(m, v.volume_no), 0) + 1,
                title: "新卷", one_line: "", function: "setup", reader_promise: "",
                chapter_start: 0, chapter_end: 0, central_conflict: "",
                climax_scene: "", climax_chapter: null, volume_hook: "", pacing: "",
                arcs_pushed: [], subplots_started: [], subplots_resolved: [],
                foreshadows_planted: [], foreshadows_paid_off: [], status: "planned",
              };
              addItem("volumes", empty);
              startEdit("volumes", String(draft.volumes.reduce((m, v) => Math.max(m, v.volume_no), 0) + 1));
            }}
          />
          <Characters
            items={draft.character_matrix}
            editingKey={editingKey}
            onEdit={id => startEdit("characters", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("character_matrix", "character_name", id, patch)}
            onDelete={id => { removeItem("character_matrix", "character_name", id); setEditingKey(null); }}
            onAdd={() => {
              const empty: CharacterMatrixEntry = {
                character_name: "新人物", role: "ally", core_values: [], taboo: [],
                speech_style: "", wants: "", fears: "", secret: "", arc: [],
                knows: [], does_not_know: [], max_absent_chapters: 0,
                last_appeared: 0, sources: [],
              };
              addItem("character_matrix", empty);
              startEdit("characters", empty.character_name);
            }}
          />
          <Foreshadows
            items={draft.foreshadows}
            editingKey={editingKey}
            onEdit={id => startEdit("foreshadows", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("foreshadows", "foreshadow_id", id, patch)}
            onDelete={id => { removeItem("foreshadows", "foreshadow_id", id); setEditingKey(null); }}
            onAdd={() => {
              const empty = {
                foreshadow_id: `fs-${Date.now()}`,
                name: "新伏笔", description: "", kind: "line",
                planted_chapter: 0, expected_payoff_chapter: 0, actual_payoff_chapter: 0,
                status: "planned", related_characters: [], related_items: [], sources: [],
              };
              addItem("foreshadows", empty);
              startEdit("foreshadows", empty.foreshadow_id);
            }}
          />
          <Subplots
            items={draft.subplots}
            editingKey={editingKey}
            onEdit={id => startEdit("subplots", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("subplots", "subplot_id", id, patch)}
            onDelete={id => { removeItem("subplots", "subplot_id", id); setEditingKey(null); }}
            onAdd={() => {
              const empty: Subplot = {
                subplot_id: `sp-${Date.now()}`,
                name: "新副线", line_tags: [], mainline_relation: "", status: "active",
                start_chapter: 0, end_chapter: null, characters: [],
                last_touched_chapter: 0, touch_interval_limit: 3,
                open_threads: [], sources: [],
              };
              addItem("subplots", empty);
              startEdit("subplots", empty.subplot_id);
            }}
          />
          <Resources
            items={draft.resources}
            editingKey={editingKey}
            onEdit={id => startEdit("resources", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("resources", "resource_id", id, patch)}
            onDelete={id => { removeItem("resources", "resource_id", id); setEditingKey(null); }}
            onAdd={() => {
              const empty: ResourceEntry = {
                resource_id: `res-${Date.now()}`,
                name: "新资源", rtype: "item", owner: "", status: "available",
                acquired_chapter: 0, consumed_chapter: 0, constraints: [], note: "", sources: [],
              };
              addItem("resources", empty);
              startEdit("resources", empty.resource_id);
            }}
          />
          <Dossiers
            items={draft.dossiers}
            editingKey={editingKey}
            onEdit={id => startEdit("dossiers", id)}
            onClose={closeEdit}
            onPatch={(id, patch) => patchItem("dossiers", "entity_id", id, patch)}
            onDelete={id => { removeItem("dossiers", "entity_id", id); setEditingKey(null); }}
            onAdd={() => {
              const empty: EntityDossier = {
                entity_type: "character", entity_id: `dossier-${Date.now()}`,
                name: "新档案", static_ref: "", current: {},
                change_log: [], appearances: [], pending: [], conflicts: [], sources: [],
              };
              addItem("dossiers", empty);
              startEdit("dossiers", empty.entity_id);
            }}
          />
        </>
      )}
    </div>
  );
}

function Overview({ bp }: { bp: BookBlueprint }) {
  return (
    <div className="card" style={{ padding: 16, marginBottom: 16 }}>
      <h3 style={{ marginTop: 0 }}>蓝图概览</h3>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 8 }}>
        <Stat label="承诺" value={bp.commitments.length} />
        <Stat label="卷蓝图" value={bp.volumes.length} />
        <Stat label="人物矩阵" value={bp.character_matrix.length} />
        <Stat label="伏笔" value={bp.foreshadows.length} />
        <Stat label="副线" value={bp.subplots.length} />
        <Stat label="资源" value={bp.resources.length} />
        <Stat label="实体档案" value={bp.dossiers.length} />
      </div>
      <p style={{ marginBottom: 0, color: "var(--color-muted)", fontSize: "var(--text-xs)" }}>
        定盘来源：{bp.settled_from || "（空）"} · 定盘时间：{bp.settled_at || "—"}
      </p>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div style={{ background: "var(--color-bg)", borderRadius: 8, padding: "10px 12px" }}>
      <div style={{ fontSize: 22, fontWeight: 700 }}>{value}</div>
      <div style={{ color: "var(--color-muted)", fontSize: 13 }}>{label}</div>
    </div>
  );
}

function CheckReport({ report, bp }: { report: BlueprintReport | null; bp: BookBlueprint }) {
  if (!report) return null;
  const hard = report.issues.filter(i => i.severity === "H");
  const soft = report.issues.filter(i => i.severity === "S");
  return (
    <div className="card" style={{ padding: 16, marginBottom: 16, borderColor: hard.length ? "var(--color-error)" : undefined }}>
      <h3 style={{ marginTop: 0 }}>全书体检</h3>
      <div style={{ display: "flex", gap: 16, marginBottom: 10 }}>
        <span style={{ color: hard.length ? "var(--color-error)" : "var(--color-success)" }}>
          <AlertTriangle size={14} style={{ verticalAlign: -2 }} /> 硬性 {hard.length}
        </span>
        <span style={{ color: soft.length ? "#b45309" : "var(--color-muted)" }}>软性 {soft.length}</span>
        <span style={{ color: "var(--color-muted)" }}>已写章节 {report.written_chapters}</span>
        <span style={{ color: "var(--color-ink-3)" }}>· {reportSummary(report)}</span>
      </div>
      {report.issues.length === 0 && (
        <p style={{ margin: 0 }}><CheckCircle2 size={14} style={{ verticalAlign: -2 }} /> 全部通过</p>
      )}
      {report.issues.map((it, i) => {
        const t = translateIssue(it, bp);
        return (
          <div key={i} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 6, background: it.severity === "H" ? "rgba(220,38,38,0.08)" : "rgba(180,83,9,0.08)" }}>
            <div style={{ fontWeight: 600, fontSize: 13 }}>{t.title}</div>
            <div style={{ fontSize: 12, color: "var(--color-ink-2)", marginTop: 2 }}>{t.fix}</div>
            <details style={{ marginTop: 4, fontSize: 11, color: "var(--color-ink-3)" }}>
              <summary style={{ cursor: "pointer" }}>技术细节</summary>
              <code style={{ whiteSpace: "pre-wrap" }}>{it.rule_id} · {it.ledger} · {it.target_id}：{it.message}</code>
            </details>
          </div>
        );
      })}
    </div>
  );
}

// ── 通用行内编辑 ──

function FieldInput({ spec, value, onChange }: { spec: FieldSpec; value: unknown; onChange: (v: any) => void }) {
  const inputStyle: CSSProperties = {
    padding: "5px 8px", borderRadius: "var(--radius-sm)",
    border: "1px solid var(--color-rule)", background: "var(--color-paper)",
    color: "var(--color-ink)", fontSize: 12, width: "100%", boxSizing: "border-box",
  };
  if (spec.kind === "boolean") {
    return (
      <input type="checkbox" checked={!!value} onChange={e => onChange(e.target.checked)} style={{ width: "auto" }} />
    );
  }
  if (spec.kind === "select" && spec.options) {
    return (
      <select style={inputStyle} value={(value as string) ?? ""} onChange={e => onChange(e.target.value)}>
        {spec.options.map(o => <option key={o} value={o}>{spec.optionLabels?.[o] ?? o}</option>)}
      </select>
    );
  }
  if (spec.kind === "number") {
    const v = value == null ? "" : String(value);
    return (
      <input
        type="number" style={inputStyle} value={v}
        placeholder={spec.placeholder}
        onChange={e => {
          const t = e.target.value;
          onChange(t === "" ? null : Number(t));
        }}
      />
    );
  }
  if (spec.kind === "json") {
    const text = value === undefined ? "" : typeof value === "string" ? value : JSON.stringify(value ?? {}, null, 2);
    return (
      <textarea
        rows={4} style={{ ...inputStyle, fontFamily: "var(--font-mono)", lineHeight: 1.5 }}
        value={text}
        placeholder={spec.placeholder}
        onChange={e => {
          const t = e.target.value;
          try { onChange(JSON.parse(t)); } catch { /* 非法 JSON 不更新 */ }
        }}
      />
    );
  }
  if (spec.kind === "textarea") {
    return (
      <textarea rows={2} style={inputStyle} value={(value as string) ?? ""} placeholder={spec.placeholder} onChange={e => onChange(e.target.value)} />
    );
  }
  return (
    <input style={inputStyle} value={(value as string) ?? ""} placeholder={spec.placeholder} onChange={e => onChange(e.target.value)} />
  );
}

function RowEditForm({
  fields, item, onPatch, onClose, onDelete,
}: {
  fields: FieldSpec[];
  item: any;
  onPatch: (patch: Record<string, unknown>) => void;
  onClose: () => void;
  onDelete: () => void;
}) {
  return (
    <div style={{ width: "100%", marginTop: 6 }}>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 8 }}>
        {fields.map(f => (
          <div key={f.key}>
            <div style={{ fontSize: 10, color: "var(--color-ink-3)", marginBottom: 2 }}>{f.label}</div>
            <FieldInput spec={f} value={item?.[f.key]} onChange={v => onPatch({ [f.key]: v })} />
          </div>
        ))}
      </div>
      <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
        <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: 11 }} onClick={onClose}>
          <Check size={12} /> 完成
        </button>
        <button className="btn btn-secondary" style={{ padding: "4px 8px", fontSize: 11 }} onClick={onDelete}>
          <Trash2 size={12} /> 删除此条
        </button>
      </div>
    </div>
  );
}

function RowActions({ editing, onEdit, onDelete }: { editing: boolean; onEdit: () => void; onDelete: () => void }) {
  if (editing) return null;
  return (
    <span style={{ display: "inline-flex", gap: 4, marginLeft: "auto" }}>
      <button className="btn btn-secondary" style={{ padding: "2px 7px", fontSize: 10 }} onClick={onEdit} title="修改">
        <Edit3 size={11} />
      </button>
      <button className="btn btn-secondary" style={{ padding: "2px 7px", fontSize: 10, color: "var(--color-error)" }} onClick={onDelete} title="删除">
        <Trash2 size={11} />
      </button>
    </span>
  );
}

function Section({
  title, onAdd, children, tech,
}: {
  title: string;
  onAdd?: () => void;
  children: ReactNode;
  tech?: unknown;
}) {
  return (
    <div className="card" style={{ padding: 16, marginBottom: 16 }}>
      <div className="card-header" style={{ fontSize: 14, fontWeight: 600, display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
        {title}
        {onAdd && (
          <button className="btn btn-secondary" style={{ marginLeft: "auto", padding: "3px 10px", fontSize: 11 }} onClick={onAdd}>
            <Plus size={12} /> 新增
          </button>
        )}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>{children}</div>
      {tech !== undefined && (
        <details style={{ marginTop: 10, fontSize: 11, color: "var(--color-ink-3)" }}>
          <summary style={{ cursor: "pointer" }}>技术细节（原始数据）</summary>
          <pre style={{ margin: "6px 0 0", whiteSpace: "pre-wrap", maxHeight: 240, overflow: "auto", fontSize: 11 }}>
            {JSON.stringify(tech, null, 2)}
          </pre>
        </details>
      )}
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <div style={{ color: "var(--color-muted)", fontSize: 13 }}>{text}</div>;
}

// ── 各账本 ──

function Commitments({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: Commitment[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "statement", label: "承诺内容", kind: "textarea" },
    {
      key: "kind", label: "类型", kind: "select",
      options: ["theme", "promise", "tone", "rule", "no_go"],
      optionLabels: { theme: "主题", promise: "读者承诺", tone: "基调", rule: "铁律", no_go: "禁区" },
    },
    {
      key: "status", label: "状态", kind: "select",
      options: ["active", "fulfilled", "waived", "broken"],
      optionLabels: { active: "在守", fulfilled: "已兑现", waived: "已豁免", broken: "破了" },
    },
    { key: "priority", label: "优先级", kind: "number" },
    { key: "scope", label: "作用范围", kind: "text" },
    { key: "resolution_chapter", label: "兑现章（留空=未定）", kind: "number" },
    { key: "ongoing", label: "持续型承诺", kind: "boolean" },
  ];
  return (
    <Section title={`这本书答应读者的承诺（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂无承诺条目（将从设定规则的铁律/禁区类内容提取）" />}
      {items.map(c => {
        const editing = editingKey === `commitments:${c.commitment_id}`;
        return (
          <div key={c.commitment_id} className="bp-row">
            <b>{c.statement}</b>
            <span className="bp-tag">{commitmentKindLabel(c.kind)}</span>
            <span className="bp-tag">{commitmentStatusLabel(c.status)}</span>
            <span className="bp-tag">{c.ongoing ? "持续型" : "兑现型"}</span>
            {c.resolution_chapter && <span className="bp-tag">兑现于第{c.resolution_chapter}章</span>}
            <RowActions editing={editing} onEdit={() => onEdit(c.commitment_id)} onDelete={() => onDelete(c.commitment_id)} />
            {editing && (
              <RowEditForm
                fields={fields}
                item={c}
                onPatch={p => onPatch(c.commitment_id, p)}
                onClose={onClose}
                onDelete={() => onDelete(c.commitment_id)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function Volumes({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd, onAddBeat,
}: {
  items: VolumeBlueprint[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
  onAddBeat: (volumeNo: number) => void;
}) {
  const fields: FieldSpec[] = [
    { key: "title", label: "卷名" },
    { key: "one_line", label: "一句话梗概", kind: "textarea" },
    {
      key: "function", label: "本卷功能", kind: "select",
      options: ["setup", "escalation", "climax", "resolution"],
      optionLabels: { setup: "开局", escalation: "升级", climax: "高潮", resolution: "收束" },
    },
    {
      key: "status", label: "状态", kind: "select",
      options: ["planned", "outlined", "drafting", "closed"],
      optionLabels: { planned: "待规划", outlined: "已定纲", drafting: "写作中", closed: "已完成" },
    },
    { key: "chapter_start", label: "起始章", kind: "number" },
    { key: "chapter_end", label: "结束章", kind: "number" },
    { key: "climax_scene", label: "高潮场景" },
    { key: "climax_chapter", label: "高潮章（留空=未定）", kind: "number" },
    { key: "volume_hook", label: "卷末钩子" },
    { key: "beats", label: "节奏点（高级：JSON 数组）", kind: "json" },
  ];
  const BEAT_LABELS: Record<string, string> = {
    hook: "钩子", buildup: "蓄力", payoff: "爽点", fall: "回落", climax: "高潮", hook_end: "卷末钩子",
  };
  return (
    <Section title={`全书结构一览（${items.length} 卷）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂未分卷（将按讨论成果的卷标记自动生成）" />}
      {items.map(v => {
        const editing = editingKey === `volumes:${v.volume_no}`;
        return (
          <div key={v.volume_no} className="bp-row" style={{ flexDirection: "column", alignItems: "flex-start" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, width: "100%" }}>
              <b>第{v.volume_no}卷 · {v.title}</b>
              <span className="bp-tag">{functionLabel(v.function)}</span>
              <span className="bp-tag">{volumeStatusLabel(v.status)}</span>
              <span className="bp-tag">{v.chapter_start}-{v.chapter_end}章</span>
              <RowActions editing={editing} onEdit={() => onEdit(String(v.volume_no))} onDelete={() => onDelete(String(v.volume_no))} />
            </div>
            {!editing && (
              <>
                <div style={{ color: "var(--color-muted)", fontSize: 13 }}>{v.one_line}</div>
                <div style={{ fontSize: 13 }}>
                  高潮：{v.climax_scene || "未定"}
                  {v.climax_chapter ? `（第${v.climax_chapter}章）` : ""}
                  {v.volume_hook ? ` · 卷末钩子：${v.volume_hook}` : ""}
                </div>
                {(v.beats ?? []).length > 0 && (
                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 2 }}>
                    {(v.beats ?? [])
                      .slice()
                      .sort((a, b) => a.chapter - b.chapter)
                      .map(b => (
                        <span key={b.beat_id} style={{
                          fontSize: 11, padding: "1px 8px", borderRadius: 10,
                          background: "var(--color-paper)", border: "1px solid var(--color-rule)",
                          color: "var(--color-ink-2)",
                        }}>
                          {BEAT_LABELS[b.beat_type] ?? b.beat_type} · 第{b.chapter || "?"}章
                          {b.note ? ` · ${b.note}` : ""}
                        </span>
                      ))}
                  </div>
                )}
                <button className="btn btn-secondary" style={{ padding: "2px 10px", fontSize: 10 }}
                  onClick={() => { onAddBeat(v.volume_no); onEdit(String(v.volume_no)); }}>
                  <Plus size={11} /> 添加节奏点
                </button>
              </>
            )}
            {editing && (
              <RowEditForm
                fields={fields}
                item={v}
                onPatch={p => onPatch(String(v.volume_no), p)}
                onClose={onClose}
                onDelete={() => onDelete(String(v.volume_no))}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function Characters({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: CharacterMatrixEntry[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "character_name", label: "姓名" },
    {
      key: "role", label: "定位", kind: "select",
      options: ["protagonist", "ally", "mentor", "antagonist", "love_interest", "supporting", "rival"],
      optionLabels: {
        protagonist: "主角", ally: "盟友", mentor: "导师", antagonist: "对手",
        love_interest: "恋人", supporting: "配角", rival: "劲敌",
      },
    },
    { key: "speech_style", label: "说话风格" },
    { key: "wants", label: "核心欲望" },
    { key: "fears", label: "恐惧" },
    { key: "secret", label: "秘密" },
  ];
  return (
    <Section title={`人物档案卡（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂无人物" />}
      {items.map(c => {
        const editing = editingKey === `characters:${c.character_name}`;
        return (
          <div key={c.character_name} className="bp-row" style={{ flexDirection: "column", alignItems: "flex-start" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, width: "100%" }}>
              <b>{c.character_name}</b>
              <span className="bp-tag">{roleLabel(c.role)}</span>
              {c.core_values.map(v => <span key={v} className="bp-tag">核心：{v}</span>)}
              <RowActions editing={editing} onEdit={() => onEdit(c.character_name)} onDelete={() => onDelete(c.character_name)} />
            </div>
            {!editing && (
              <>
                <div style={{ fontSize: 13 }}>欲望：{c.wants || "未定"} · 恐惧：{c.fears || "未定"}</div>
                {c.secret && <div style={{ fontSize: 13, color: "#9d174d" }}>秘密：{c.secret}</div>}
                {c.arc.length > 0 && (
                  <div style={{ fontSize: 13 }}>
                    弧光：{c.arc.map(a => `${a.name}（${a.chapter_range}）`).join(" → ")}
                  </div>
                )}
              </>
            )}
            {editing && (
              <RowEditForm
                fields={fields}
                item={c}
                onPatch={p => onPatch(c.character_name, p)}
                onClose={onClose}
                onDelete={() => onDelete(c.character_name)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function Foreshadows({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: BookBlueprint["foreshadows"];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "name", label: "名称" },
    { key: "description", label: "描述", kind: "textarea" },
    { key: "kind", label: "类型" },
    {
      key: "status", label: "状态", kind: "select",
      options: ["planned", "planted", "progressing", "paid_off", "waived", "overdue"],
      optionLabels: {
        planned: "待埋设", planted: "已埋设", progressing: "推进中",
        paid_off: "已回收", waived: "已放弃", overdue: "已逾期",
      },
    },
    { key: "planted_chapter", label: "埋设章", kind: "number" },
    { key: "expected_payoff_chapter", label: "预期回收章", kind: "number" },
  ];
  return (
    <Section title={`悬念清单（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂未登记伏笔（将从情节节点的伏笔计划展开）" />}
      {items.map(f => {
        const editing = editingKey === `foreshadows:${f.foreshadow_id}`;
        return (
          <div key={f.foreshadow_id} className="bp-row">
            <b>{f.name}</b>
            <span className="bp-tag">{foreshadowStatusLabel(f.status)}</span>
            <span className="bp-tag">埋于第{f.planted_chapter || "?"}章{f.expected_payoff_chapter ? ` → 回收于第${f.expected_payoff_chapter}章` : " → 回收未定"}</span>
            <span style={{ color: "var(--color-muted)", fontSize: 13 }}>{f.description}</span>
            <RowActions editing={editing} onEdit={() => onEdit(f.foreshadow_id)} onDelete={() => onDelete(f.foreshadow_id)} />
            {editing && (
              <RowEditForm
                fields={fields}
                item={f}
                onPatch={p => onPatch(f.foreshadow_id, p)}
                onClose={onClose}
                onDelete={() => onDelete(f.foreshadow_id)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function Subplots({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: Subplot[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "name", label: "副线名" },
    { key: "mainline_relation", label: "与主线关系" },
    {
      key: "status", label: "状态", kind: "select",
      options: ["planned", "active", "paused", "dormant", "resolved", "abandoned"],
      optionLabels: {
        planned: "待启动", active: "进行中", paused: "暂停",
        dormant: "休眠", resolved: "已解决", abandoned: "已放弃",
      },
    },
    { key: "start_chapter", label: "起始章", kind: "number" },
    { key: "end_chapter", label: "结束章（留空=进行中）", kind: "number" },
    { key: "touch_interval_limit", label: "闲置上限（章）", kind: "number" },
  ];
  return (
    <Section title={`支线清单（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂未发现副线（将从情节线的多线标签聚合）" />}
      {items.map(s => {
        const editing = editingKey === `subplots:${s.subplot_id}`;
        return (
          <div key={s.subplot_id} className="bp-row">
            <b>{s.name}</b>
            <span className="bp-tag">{subplotStatusLabel(s.status)}</span>
            <span className="bp-tag">{s.start_chapter}-{s.end_chapter ?? "?"}章</span>
            <span className="bp-tag">闲置上限 {s.touch_interval_limit} 章</span>
            <span style={{ color: "var(--color-muted)", fontSize: 13 }}>{s.mainline_relation}</span>
            <RowActions editing={editing} onEdit={() => onEdit(s.subplot_id)} onDelete={() => onDelete(s.subplot_id)} />
            {editing && (
              <RowEditForm
                fields={fields}
                item={s}
                onPatch={p => onPatch(s.subplot_id, p)}
                onClose={onClose}
                onDelete={() => onDelete(s.subplot_id)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function Resources({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: ResourceEntry[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "name", label: "名称" },
    { key: "rtype", label: "类型" },
    {
      key: "status", label: "状态", kind: "select",
      options: ["available", "used", "consumed", "lost", "destroyed", "transferred", "revealed"],
      optionLabels: {
        available: "可用", used: "已使用", consumed: "已消耗", lost: "已丢失",
        destroyed: "已毁", transferred: "已转手", revealed: "已公开",
      },
    },
    { key: "owner", label: "持有者" },
    { key: "note", label: "备注", kind: "textarea" },
  ];
  return (
    <Section title={`道具与金手指（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂未登记资源（将从世界观物品图带入）" />}
      {items.map(r => {
        const editing = editingKey === `resources:${r.resource_id}`;
        return (
          <div key={r.resource_id} className="bp-row">
            <b>{r.name}</b>
            <span className="bp-tag">{r.rtype}</span>
            <span className="bp-tag">{resourceStatusLabel(r.status)}</span>
            <span className="bp-tag">持有者：{r.owner || "无"}</span>
            <RowActions editing={editing} onEdit={() => onEdit(r.resource_id)} onDelete={() => onDelete(r.resource_id)} />
            {editing && (
              <RowEditForm
                fields={fields}
                item={r}
                onPatch={p => onPatch(r.resource_id, p)}
                onClose={onClose}
                onDelete={() => onDelete(r.resource_id)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

// 档案 current 状态的中文标签渲染：只展示已知字段，未知字段折叠
const DOSSIER_LABELS: Record<string, string> = {
  outfit: "着装", accessories: "配饰", physical: "身体特征", weapons: "武器",
  mood: "情绪", goal: "当前目标", location: "位置", alive: "状态", health: "健康",
  knowledge: "知道的事", relations: "人际关系", techniques: "功法/技能",
  description: "描述", level: "层级", region: "区域", faction: "所属势力",
  unlocked_chapter: "解锁章",
};

function DossierCurrentView({ current }: { current: unknown }) {
  if (!current || typeof current !== "object") {
    return <div style={{ fontSize: 12, color: "var(--color-ink-3)" }}>（暂无状态数据）</div>;
  }
  const obj = current as Record<string, any>;
  const rows: Array<{ label: string; value: string }> = [];
  for (const section of ["appearance", "state", "abilities"]) {
    const sec = obj[section];
    if (!sec || typeof sec !== "object") continue;
    for (const [k, v] of Object.entries(sec)) {
      if (v === undefined || v === null || v === "") continue;
      let text: string;
      if (Array.isArray(v)) {
        text = v.map((x: any) => typeof x === "string" ? x : (x.name ?? JSON.stringify(x))).join("、");
      } else if (typeof v === "object") {
        text = JSON.stringify(v);
      } else {
        text = String(v);
      }
      if (k === "alive") text = v ? "活着" : "已故";
      rows.push({ label: DOSSIER_LABELS[k] ?? k, value: text });
    }
  }
  if (rows.length === 0) {
    return <div style={{ fontSize: 12, color: "var(--color-ink-3)" }}>（暂无状态数据）</div>;
  }
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: "4px 16px", fontSize: 12, width: "100%" }}>
      {rows.map(r => (
        <div key={r.label} style={{ display: "flex", gap: 6 }}>
          <span style={{ color: "var(--color-ink-3)", flexShrink: 0 }}>{r.label}：</span>
          <span style={{ color: "var(--color-ink-2)" }}>{r.value}</span>
        </div>
      ))}
    </div>
  );
}

function Dossiers({
  items, editingKey, onEdit, onClose, onPatch, onDelete, onAdd,
}: {
  items: EntityDossier[];
  editingKey: string | null;
  onEdit: (id: string) => void;
  onClose: () => void;
  onPatch: (id: string, patch: Record<string, unknown>) => void;
  onDelete: (id: string) => void;
  onAdd: () => void;
}) {
  const fields: FieldSpec[] = [
    { key: "name", label: "名称" },
    {
      key: "entity_type", label: "类型", kind: "select",
      options: ["character", "location", "faction"],
      optionLabels: { character: "人物", location: "地点", faction: "势力" },
    },
    { key: "static_ref", label: "静态引用" },
    { key: "current", label: "当前状态（高级：JSON）", kind: "json" },
  ];
  return (
    <Section title={`角色状态卡（${items.length}）`} onAdd={onAdd} tech={items}>
      {items.length === 0 && <Empty text="暂未生成档案骨架（将随人物/地点自动建立）" />}
      {items.map(d => {
        const editing = editingKey === `dossiers:${d.entity_id}`;
        return (
          <div key={d.entity_id} className="bp-row" style={{ flexDirection: "column", alignItems: "flex-start" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, width: "100%" }}>
              <b>{d.name}</b>
              <span className="bp-tag">{entityTypeLabel(d.entity_type)}</span>
              {d.change_log.length > 0 && <span className="bp-tag">变更 {d.change_log.length} 条</span>}
              {d.pending.length > 0 && <span className="bp-tag">悬置 {d.pending.length} 条</span>}
              {d.conflicts.length > 0 && <span className="bp-tag" style={{ color: "var(--color-error)" }}>冲突 {d.conflicts.length}</span>}
              <RowActions editing={editing} onEdit={() => onEdit(d.entity_id)} onDelete={() => onDelete(d.entity_id)} />
            </div>
            {!editing && (
              <>
                <DossierCurrentView current={d.current} />
                <details style={{ fontSize: 11, color: "var(--color-ink-3)", marginTop: 4 }}>
                  <summary style={{ cursor: "pointer" }}>原始数据（JSON）</summary>
                  <pre style={{ margin: "6px 0 0", whiteSpace: "pre-wrap", maxHeight: 180, overflow: "auto" }}>
                    {JSON.stringify(d.current ?? {}, null, 2)}
                  </pre>
                </details>
              </>
            )}
            {editing && (
              <RowEditForm
                fields={fields}
                item={d}
                onPatch={p => onPatch(d.entity_id, p)}
                onClose={onClose}
                onDelete={() => onDelete(d.entity_id)}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

// 与后端 synthesis_stamp 一致：角色数|情节数|规则数|地点数|总结字数
function calcStamp(syn: any): string {
  if (!syn) return "";
  return `${syn.characters?.length ?? 0}|${syn.outline_beats?.length ?? 0}|${syn.setting_rules?.length ?? 0}|${syn.locations?.length ?? 0}|${(syn.summary ?? "").length}`;
}
