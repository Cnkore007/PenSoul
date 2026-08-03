import { useState, useEffect } from "react";
import { ChevronRight, ChevronDown, FileText, Plus, Edit3, Trash2, Check, X, GitBranch, Wand2, Layers, Eraser } from "lucide-react";
import type { ProjectData, VolumeWithChapters, Chapter, OutlineArc, LlmModel } from "../types";
import { deleteChapter, deleteVolume, expandOutlineArcAll, getOutlineExpandState, cancelOutlineExpand, saveOutlineArcs, listModels, smartVolumeSplit, clearOutline } from "../ipc";
import { listen } from "@tauri-apps/api/event";
import { EntityAnnotations } from "../components/EntityAnnotations";
import { confirmDialog, messageDialog } from "../dialogs";

interface OutlineViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  // 细纲展开后从后端重载章节列表（新增章节在后端落库）
  onRefresh?: () => Promise<void> | void;
}

// 隐式默认卷 ID：未显式建卷时章节挂在这里，页面平铺展示，不显示卷头
const DEFAULT_VOL_ID = "_default";
export function OutlineView({ projectData, persistProjectData, onRefresh }: OutlineViewProps) {
  // 新建卷 / 章节
  const [newVolumeTitle, setNewVolumeTitle] = useState("");
  const [showNewVolume, setShowNewVolume] = useState(false);
  const [newChapterTitle, setNewChapterTitle] = useState("");
  const [showNewChapterFor, setShowNewChapterFor] = useState<string | null>(null);
  // 行内编辑：卷重命名
  const [editingVolumeId, setEditingVolumeId] = useState<string | null>(null);
  const [editVolumeTitle, setEditVolumeTitle] = useState("");
  // 行内编辑：章节标题 + 梗概
  const [editingChapterId, setEditingChapterId] = useState<string | null>(null);
  const [editChapterTitle, setEditChapterTitle] = useState("");
  const [editChapterSummary, setEditChapterSummary] = useState("");
  // 情节脉络：展开中的节点 / 错误 / 行内编辑
  const [expandingArcId, setExpandingArcId] = useState<string | null>(null);
  // 展开进度（自动续展时显示 已展开/总数）
  const [expandProgress, setExpandProgress] = useState<{ expanded: number; total: number } | null>(null);
  const [arcError, setArcError] = useState<string | null>(null);
  const [editingArcId, setEditingArcId] = useState<string | null>(null);
  const [editArcTitle, setEditArcTitle] = useState("");
  const [editArcDesc, setEditArcDesc] = useState("");
  const [editArcStart, setEditArcStart] = useState(0);
  const [editArcEnd, setEditArcEnd] = useState(0);
  // 细纲展开模型：默认跟随工作流页「环节技能绑定 → 细纲展开」，可在本页现场覆盖
  const [models, setModels] = useState<LlmModel[]>([]);
  const [expandModel, setExpandModel] = useState("");

  useEffect(() => {
    listModels()
      .then((ms) => setModels(ms as LlmModel[]))
      .catch(() => {});
  }, []);

  // 页面切换后重连：若细纲展开后台任务仍在进行，恢复运行状态与进度
  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    (async () => {
      const st = await getOutlineExpandState().catch(() => null);
      if (cancelled || !st) return;
      if (st.running && st.arc_id) {
        setExpandingArcId(st.arc_id);
        setExpandProgress(st.progress ?? null);
        setArcError(null);
      } else if (st.events?.some((e: any) => e.phase === "error")) {
        const lastErr = [...(st.events ?? [])].reverse().find((e: any) => e.phase === "error");
        setArcError(lastErr?.message ?? "细纲展开失败");
      }
    })();
    // 订阅实时进度（后台任务逐批展开时更新）
    const onEvent = (evt: any) => {
      const e = evt?.payload;
      if (!e) return;
      if (e.arc_id) setExpandingArcId(e.arc_id);
      setExpandProgress({ expanded: e.expanded ?? 0, total: e.total ?? 0 });
      if (e.phase === "progress") {
        // 每批落库后刷新章节列表，让新细纲渐进可见
        onRefresh?.();
      } else if (e.phase === "done") {
        setExpandingArcId(null);
        setExpandProgress(null);
        onRefresh?.();
      } else if (e.phase === "cancelled") {
        setExpandingArcId(null);
        setExpandProgress(null);
        onRefresh?.();
      } else if (e.phase === "error") {
        setExpandingArcId(null);
        setExpandProgress(null);
        setArcError(e.message || "细纲展开失败");
        onRefresh?.();
      }
    };
    listen<any>("outline-expand-phase", onEvent).then(un => unlisteners.push(un)).catch(() => {});
    return () => {
      cancelled = true;
      unlisteners.forEach(un => un());
    };
  }, [onRefresh]);

  const arcs = projectData.outlineArcs ?? [];

  // ── 情节脉络：展开细纲 / 编辑 / 删除 ──

  function updateArcsLocal(next: OutlineArc[]) {
    persistProjectData(prev => ({ ...prev, outlineArcs: next }));
  }

  // 展开细纲：一键后台展开该节点全部剩余章节（后端按批 20 章自动续展，
  // 批间携带已展开章节作为上下文衔接）；进度经事件推送，切换页面后自动重连恢复。
  // 「取消」后保留已展开部分，可再次点击继续。
  async function handleExpandArc(arc: OutlineArc) {
    setExpandingArcId(arc.arc_id);
    setArcError(null);
    const total = Math.max(0, arc.chapter_end - arc.chapter_start + 1);
    const already = arc.expanded_until > 0
      ? Math.max(0, Math.min(arc.expanded_until, arc.chapter_end) - arc.chapter_start + 1)
      : 0;
    setExpandProgress({ expanded: already, total });
    try {
      const stageCfg = projectData.workflowSkills?.outline_expand;
      // 现场选择的模型优先，其次工作流配置，最后由后端自动选第一个可用模型
      const model = expandModel || stageCfg?.model || null;
      const cards = stageCfg?.cards ?? null;
      await expandOutlineArcAll(arc.arc_id, model, cards);
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.message || String(e);
      if (msg.includes("已全部展开")) {
        // 并发/重入下已展开完，视为完成
        setExpandingArcId(null);
        setExpandProgress(null);
      } else if (msg.includes("已有细纲展开任务正在进行")) {
        // 已有后台任务在跑：重连恢复即可，不重复报错
      } else {
        setArcError(msg);
        setExpandingArcId(null);
        setExpandProgress(null);
      }
      await onRefresh?.();
    }
  }

  // 取消当前节点的后台展开（已展开部分保留）
  function handleCancelExpand() {
    cancelOutlineExpand().catch(() => {});
  }

  function startEditArc(arc: OutlineArc) {
    setEditingArcId(arc.arc_id);
    setEditArcTitle(arc.title);
    setEditArcDesc(arc.description);
    setEditArcStart(arc.chapter_start);
    setEditArcEnd(arc.chapter_end);
  }

  async function handleSaveArcEdit() {
    if (!editingArcId || !editArcTitle.trim()) return;
    const start = Math.max(1, editArcStart || 1);
    const end = Math.max(start, editArcEnd || start);
    const next = arcs.map(a => a.arc_id === editingArcId
      ? { ...a, title: editArcTitle.trim(), description: editArcDesc.trim(), chapter_start: start, chapter_end: end }
      : a);
    updateArcsLocal(next);
    setEditingArcId(null);
    await saveOutlineArcs(next).catch(e => setArcError("脉络保存失败: " + (e?.message ?? e)));
  }

  async function handleDeleteArc(arc: OutlineArc) {
    const expandedCount = Math.max(0, Math.min(arc.expanded_until, arc.chapter_end) - arc.chapter_start + (arc.expanded_until > 0 ? 1 : 0));
    const hint = expandedCount > 0
      ? `删除脉络节点「${arc.title}」？已展开的 ${expandedCount} 章细纲与正文不受影响。`
      : `删除脉络节点「${arc.title}」？`;
    if (!(await confirmDialog(hint))) return;
    const next = arcs.filter(a => a.arc_id !== arc.arc_id);
    updateArcsLocal(next);
    await saveOutlineArcs(next).catch(e => setArcError("脉络删除失败: " + (e?.message ?? e)));
  }

  const volumes = projectData.volumes;
  const realVolumes = volumes.filter(v => v.volume_id !== DEFAULT_VOL_ID);
  const defaultVol = volumes.find(v => v.volume_id === DEFAULT_VOL_ID);
  // 未显式建卷 → 平铺模式：章节直接列出，没有卷概念
  const flat = realVolumes.length === 0;

  // 细纲不计入章节数与字数：章节数只统计已开始细写（有正文）的章节
  const allChapters = volumes.flatMap(v => v.chapters);
  const writtenChapters = allChapters.filter(c => (c.word_count ?? 0) > 0);
  const totalChapters = writtenChapters.length;
  const totalWords = writtenChapters.reduce((s, c) => s + (c.word_count ?? 0), 0);
  const outlineCount = allChapters.length - writtenChapters.length;

  function toggleVolume(volId: string) {
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId ? { ...v, expanded: !v.expanded } : v),
    }));
  }

  // ── 卷：新建 / 重命名 / 删除 ──

  function addVolume() {
    if (!newVolumeTitle.trim()) return;
    const vol: VolumeWithChapters = {
      volume_id: `vol-${Date.now()}`,
      title: newVolumeTitle.trim(),
      chapter_count: 0,
      expanded: true,
      chapters: [],
    };
    persistProjectData(prev => ({ ...prev, volumes: [...prev.volumes, vol] }));
    setNewVolumeTitle("");
    setShowNewVolume(false);
  }

  function startRenameVolume(vol: VolumeWithChapters) {
    setEditingVolumeId(vol.volume_id);
    setEditVolumeTitle(vol.title);
  }

  function handleRenameVolume(volId: string) {
    const title = editVolumeTitle.trim();
    if (!title) return;
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId ? { ...v, title } : v),
    }));
    setEditingVolumeId(null);
  }

  async function handleDeleteVolume(vol: VolumeWithChapters) {
    const hint = vol.chapters.length > 0
      ? `删除卷「${vol.title}」？其中的 ${vol.chapters.length} 个章节及正文将一并删除，不可恢复。`
      : `删除卷「${vol.title}」？`;
    if (!(await confirmDialog(hint))) return;
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.filter(v => v.volume_id !== vol.volume_id),
    }));
    deleteVolume(vol.volume_id).catch(err => console.error("删除卷失败:", err));
  }

  // 智能分卷：按基础设定卷数均分全部章节到各卷
  async function handleSmartVolumeSplit() {
    const target = projectData.settings.targetVolumes;
    if (target <= 0) {
      setArcError("智能分卷需要先在基础设定中填写「预计卷数」");
      return;
    }
    const ok = await confirmDialog(`按基础设定卷数（${target} 卷）智能分卷？\n全部章节将按顺序均分到「第一卷/第二卷…」，原有卷结构会被替换。`);
    if (!ok) return;
    try {
      const res = await smartVolumeSplit();
      await onRefresh?.();
      await messageDialog(res.message);
    } catch (e: any) {
      setArcError(typeof e === "string" ? e : e?.message || String(e));
    }
  }

  // 一键清空大纲：卷/章节/脉络节点全部删除（前后端同步）
  async function handleClearOutline() {
    if (allChapters.length === 0 && arcs.length === 0) return;
    const hint = `一键删除全部大纲内容？\n${allChapters.length} 个章节（含 ${outlineCount} 个细纲、${totalChapters} 个已写正文）、${realVolumes.length} 个卷、${arcs.length} 段情节脉络将全部删除，不可恢复。`;
    if (!(await confirmDialog(hint))) return;
    try {
      await clearOutline();
      await onRefresh?.();
      await messageDialog("大纲已全部清空");
    } catch (e: any) {
      setArcError(typeof e === "string" ? e : e?.message || String(e));
    }
  }

  // ── 章节：新建 / 编辑（标题 + 梗概）/ 删除 ──

  function addChapter(volId: string) {
    if (!newChapterTitle.trim()) return;
    const ch: Chapter = {
      chapter_id: `ch-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      volume_id: volId,
      title: newChapterTitle.trim(),
      summary: "",
      content: "",
      word_count: 0,
      version: 1,
      status: "Draft",
    };
    persistProjectData(prev => {
      if (!prev.volumes.some(v => v.volume_id === volId)) {
        return { ...prev, volumes: [...prev.volumes, { volume_id: volId, title: "", chapter_count: 1, expanded: true, chapters: [ch] }] };
      }
      return {
        ...prev,
        volumes: prev.volumes.map(v => v.volume_id === volId
          ? { ...v, chapters: [...v.chapters, ch], chapter_count: v.chapters.length + 1, expanded: true }
          : v),
      };
    });
    setNewChapterTitle("");
    setShowNewChapterFor(null);
  }

  function startEditChapter(ch: Chapter) {
    setEditingChapterId(ch.chapter_id);
    setEditChapterTitle(ch.title);
    setEditChapterSummary(ch.summary ?? "");
  }

  function handleSaveChapterEdit(volId: string) {
    if (!editingChapterId || !editChapterTitle.trim()) return;
    const title = editChapterTitle.trim();
    const summary = editChapterSummary.trim();
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId
        ? { ...v, chapters: v.chapters.map(c => c.chapter_id === editingChapterId ? { ...c, title, summary } : c) }
        : v),
    }));
    setEditingChapterId(null);
  }

  async function handleDeleteChapter(volId: string, ch: Chapter) {
    const hint = ch.word_count > 0
      ? `删除章节「${ch.title}」？已写入的 ${ch.word_count} 字正文将一并删除，不可恢复。`
      : `删除章节「${ch.title}」？`;
    if (!(await confirmDialog(hint))) return;
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId
        ? { ...v, chapters: v.chapters.filter(c => c.chapter_id !== ch.chapter_id), chapter_count: v.chapters.length - 1 }
        : v),
    }));
    deleteChapter(ch.chapter_id).catch(err => console.error("删除章节失败:", err));
  }

  // ── 渲染片段 ──

  const renderChapterRow = (volId: string, ch: Chapter) => (
    editingChapterId === ch.chapter_id ? (
      <div key={ch.chapter_id} style={{ display: "flex", flexDirection: "column", gap: 8, padding: "8px 32px" }}>
        <input className="pm-input" style={{ marginBottom: 0 }} placeholder="章节标题" value={editChapterTitle} onChange={e => setEditChapterTitle(e.target.value)} autoFocus />
        <textarea
          className="pm-textarea"
          style={{ marginBottom: 0 }}
          rows={3}
          placeholder="章节梗概（大纲层信息：本章发生什么、冲突是什么，非正文）"
          value={editChapterSummary}
          onChange={e => setEditChapterSummary(e.target.value)}
        />
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-primary" style={{ padding: "4px 10px" }} onClick={() => handleSaveChapterEdit(volId)} disabled={!editChapterTitle.trim()}>保存</button>
          <EntityAnnotations target={`chapter:${ch.chapter_id}:summary`} title="细纲批注" />
          <button className="btn btn-secondary" style={{ padding: "4px 10px" }} onClick={() => setEditingChapterId(null)}>取消</button>
        </div>
      </div>
    ) : (
      <div key={ch.chapter_id} className="chapter-item">
        <FileText size={13} className="chapter-icon" />
        <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
          <span className="chapter-title">{(ch.chapter_no ?? 0) > 0 ? `第${ch.chapter_no}章 · ` : ""}{ch.title}</span>
          {ch.summary && (
            <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {ch.summary}
            </span>
          )}
        </div>
        <span className="chapter-actions">
          <EntityAnnotations target={`chapter:${ch.chapter_id}:summary`} title="细纲批注" />
          <button className="pv-icon-btn" title="编辑章节" onClick={() => startEditChapter(ch)}><Edit3 size={13} /></button>
          <button className="pv-icon-btn pv-icon-btn-danger" title="删除章节" onClick={() => handleDeleteChapter(volId, ch)}><Trash2 size={13} /></button>
        </span>
        <span className="chapter-words">{ch.word_count.toLocaleString()} 字</span>
        <span className={"chapter-status-badge badge-" + ch.status.toLowerCase()}>
          {{ Draft: "草稿", Reviewing: "评审中", Reviewed: "已评审", Polished: "已精修", Published: "已发布" }[ch.status] ?? ch.status}
        </span>
      </div>
    )
  );

  const renderAddChapter = (volId: string) => (
    showNewChapterFor === volId ? (
      <div style={{ display: "flex", gap: 8, padding: "8px 32px" }}>
        <input className="pm-input" style={{ marginBottom: 0, flex: 1 }} placeholder="章节标题" value={newChapterTitle} onChange={e => setNewChapterTitle(e.target.value)} autoFocus onKeyDown={e => e.key === "Enter" && addChapter(volId)} />
        <button className="btn btn-primary" style={{ padding: "4px 10px" }} onClick={() => addChapter(volId)}>添加</button>
        <button className="btn btn-secondary" style={{ padding: "4px 10px" }} onClick={() => { setShowNewChapterFor(null); setNewChapterTitle(""); }}>取消</button>
      </div>
    ) : (
      <div style={{ padding: "4px 32px" }}>
        <button className="btn btn-secondary" style={{ fontSize: "var(--text-xs)", padding: "2px 8px" }} onClick={() => setShowNewChapterFor(volId)}>
          <Plus size={12} /> 添加章节
        </button>
      </div>
    )
  );

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>大纲</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px" }}>
          结构与梗概在此维护 · 正文由工作流在笔耕细写
        </span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="btn btn-secondary" onClick={handleClearOutline}
            title="一键删除全部卷、章节（含细纲与正文）与情节脉络（不可恢复）"
            disabled={allChapters.length === 0 && arcs.length === 0}>
            <Eraser size={15} /> 全部删除
          </button>
          <button className="btn btn-secondary" onClick={handleSmartVolumeSplit}
            title={projectData.settings.targetVolumes > 0
              ? `按基础设定卷数（${projectData.settings.targetVolumes} 卷）智能分卷：把全部章节按顺序均分到各卷`
              : "智能分卷需要先在基础设定中填写「预计卷数」；未填写时保留导入的卷名"}
            disabled={projectData.settings.targetVolumes <= 0}>
            <Layers size={15} /> 智能分卷
          </button>
          <button className="btn btn-primary" onClick={() => setShowNewVolume(true)}>
            <Plus size={15} /> 新建卷
          </button>
        </div>
      </div>

      {showNewVolume && (
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <input className="pm-input" style={{ marginBottom: 0, flex: 1 }} placeholder="卷名" value={newVolumeTitle} onChange={e => setNewVolumeTitle(e.target.value)} autoFocus onKeyDown={e => e.key === "Enter" && addVolume()} />
          <button className="btn btn-primary" onClick={addVolume}>确定</button>
          <button className="btn btn-secondary" onClick={() => { setShowNewVolume(false); setNewVolumeTitle(""); }}>取消</button>
        </div>
      )}

      {/* ── 情节脉络（大纲规划层）：讨论产出的故事段规划，展开细纲后才生成可写章节 ── */}
      {arcs.length > 0 && (
        <div className="card" style={{ marginBottom: 16 }}>
          <div className="card-header" style={{ fontSize: 14, fontWeight: 600 }}>
            <GitBranch size={15} style={{ verticalAlign: -2, marginRight: 6 }} />
            情节脉络（{arcs.length} 段）
            <span style={{ marginLeft: "auto", display: "inline-flex", alignItems: "center", gap: 8, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", fontWeight: 400 }}>
              每段覆盖一个章节范围，点击「全部展开」一次生成全部逐章细纲（内部按每批 20 章自动续展，可在展开中取消）
              <select
                className="pm-input"
                style={{ marginBottom: 0, width: 180, padding: "2px 6px", fontSize: "var(--text-2xs)" }}
                value={expandModel}
                onChange={(e) => setExpandModel(e.target.value)}
                disabled={expandingArcId !== null}
                title="细纲展开使用的模型：默认跟随工作流页「环节技能绑定 → 细纲展开」配置"
              >
                <option value="">展开模型：跟随工作流配置</option>
                {models
                  .filter((m) => m.is_available !== false)
                  .map((m) => (
                    <option key={m.model_id} value={m.model_id}>
                      {m.display_name || m.model_id}
                    </option>
                  ))}
              </select>
            </span>
          </div>
          {arcError && (
            <div style={{ margin: "8px 12px 0", padding: "6px 10px", background: "var(--color-error-wash)", border: "1px solid var(--color-error)", borderRadius: "var(--radius-sm)", fontSize: "var(--text-xs)", color: "var(--color-error)" }}>
              {arcError}
            </div>
          )}
          <div style={{ display: "flex", flexDirection: "column", gap: 4, padding: "8px 4px" }}>
            {arcs.map(arc => {
              const total = Math.max(0, arc.chapter_end - arc.chapter_start + 1);
              const done = arc.expanded_until >= arc.chapter_end && arc.chapter_end > 0;
              const expandedCount = arc.expanded_until > 0
                ? Math.max(0, Math.min(arc.expanded_until, arc.chapter_end) - arc.chapter_start + 1)
                : 0;
              const expanding = expandingArcId === arc.arc_id;
              return editingArcId === arc.arc_id ? (
                <div key={arc.arc_id} style={{ display: "flex", flexDirection: "column", gap: 8, padding: "10px 12px", borderRadius: 8, background: "var(--color-paper-warm)" }}>
                  <input className="pm-input" style={{ marginBottom: 0 }} placeholder="节点标题" value={editArcTitle} onChange={e => setEditArcTitle(e.target.value)} autoFocus />
                  <textarea className="pm-textarea" style={{ marginBottom: 0 }} rows={3} placeholder="剧情规划" value={editArcDesc} onChange={e => setEditArcDesc(e.target.value)} />
                  <div style={{ display: "flex", gap: 8, alignItems: "center", fontSize: "var(--text-xs)" }}>
                    <span style={{ opacity: 0.7 }}>章节范围</span>
                    <input className="pm-input" type="number" min="1" style={{ marginBottom: 0, width: 80, padding: "4px 8px" }} value={editArcStart || ""} onChange={e => setEditArcStart(parseInt(e.target.value) || 0)} />
                    <span>—</span>
                    <input className="pm-input" type="number" min="1" style={{ marginBottom: 0, width: 80, padding: "4px 8px" }} value={editArcEnd || ""} onChange={e => setEditArcEnd(parseInt(e.target.value) || 0)} />
                    <span style={{ opacity: 0.5 }}>章</span>
                    <span style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
                      <button className="btn btn-primary" style={{ padding: "4px 10px" }} onClick={handleSaveArcEdit} disabled={!editArcTitle.trim()}>保存</button>
                      <button className="btn btn-secondary" style={{ padding: "4px 10px" }} onClick={() => setEditingArcId(null)}>取消</button>
                    </span>
                  </div>
                </div>
              ) : (
                <div key={arc.arc_id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "10px 12px", borderRadius: 8, background: "var(--color-paper-warm)" }}>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>{arc.title}</span>
                      <span style={{ fontSize: "var(--text-2xs)", padding: "1px 8px", borderRadius: 10, background: "var(--color-indigo-wash)", color: "var(--color-indigo)", whiteSpace: "nowrap" }}>
                        第{arc.chapter_start}–{arc.chapter_end}章
                      </span>
                      <span style={{ fontSize: "var(--text-2xs)", color: done ? "var(--color-jade)" : "var(--color-ink-3)", whiteSpace: "nowrap" }}>
                        {done ? "细纲已齐" : expandedCount > 0 ? `细纲 ${expandedCount}/${total} 章` : `共 ${total} 章 · 未展开`}
                      </span>
                    </div>
                    {arc.description && (
                      <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginTop: 3, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {arc.description}
                      </div>
                    )}
                  </div>
                  {!done && (expanding ? (
                    <button className="btn btn-secondary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)", whiteSpace: "nowrap" }}
                      onClick={handleCancelExpand}
                      title="停止自动续展，已展开的细纲保留，可再次点击继续">
                      <X size={13} /> 取消（{expandProgress ? `${expandProgress.expanded}/${expandProgress.total}` : "展开中"}）
                    </button>
                  ) : (
                    <button className="btn btn-accent" style={{ padding: "4px 12px", fontSize: "var(--text-xs)", whiteSpace: "nowrap" }}
                      disabled={expandingArcId !== null}
                      title="调用 LLM 按本段规划生成全部逐章梗概（每批 20 章自动续展，可在展开中取消）"
                      onClick={() => handleExpandArc(arc)}>
                      <Wand2 size={13} /> 全部展开（共 {total} 章）
                    </button>
                  ))}
                  <button className="pv-icon-btn" title="编辑节点" onClick={() => startEditArc(arc)}><Edit3 size={13} /></button>
                  <EntityAnnotations target={`outline_arc:${arc.arc_id}`} />
                  <button className="pv-icon-btn pv-icon-btn-danger" title="删除节点" onClick={() => handleDeleteArc(arc)}><Trash2 size={13} /></button>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* ── 章节细纲（可写层） ── */}
      <div className="card outline-card">
        {allChapters.length === 0 && realVolumes.length === 0 && (
          <div className="empty-state" style={{ padding: "40px 20px" }}>
            <div className="empty-state-icon">纲</div>
            <div className="empty-state-text">{arcs.length > 0 ? "还没有章节细纲" : "尚无大纲"}</div>
            <div className="empty-state-sub">
              {arcs.length > 0
                ? "点击上方脉络节点的「展开细纲」，按剧情规划生成逐章梗概"
                : "点击「新建卷」分卷规划，或直接添加章节，也可以从灵魂萌芽的讨论成果导入"}
            </div>
          </div>
        )}

        {/* 平铺模式：未显式建卷，章节直接列出 */}
        {flat && defaultVol && defaultVol.chapters.length > 0 && (
          <div className="chapter-list">
            {defaultVol.chapters.map(ch => renderChapterRow(DEFAULT_VOL_ID, ch))}
          </div>
        )}
        {flat && (totalChapters > 0 || defaultVol) && renderAddChapter(DEFAULT_VOL_ID)}

        {/* 分组模式：显式建卷后，未归卷章节归入「未分卷」 */}
        {!flat && defaultVol && defaultVol.chapters.length > 0 && (
          <div className="outline-volume-group">
            <div className="volume-header" style={{ cursor: "default" }}>
              <span className="volume-expand-icon" />
              <span className="volume-title">未分卷</span>
              <span className="volume-count">{defaultVol.chapters.length} 章</span>
            </div>
            <div className="chapter-list">
              {defaultVol.chapters.map(ch => renderChapterRow(DEFAULT_VOL_ID, ch))}
            </div>
          </div>
        )}
        {!flat && realVolumes.map(volume => (
          <div key={volume.volume_id} className="outline-volume-group">
            <div onClick={() => toggleVolume(volume.volume_id)} className="volume-header">
              <span className="volume-expand-icon">
                {volume.expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
              </span>
              {editingVolumeId === volume.volume_id ? (
                <span style={{ display: "flex", gap: 4, alignItems: "center", flex: 1 }} onClick={e => e.stopPropagation()}>
                  <input
                    className="pm-input"
                    style={{ marginBottom: 0, flex: 1, padding: "2px 8px", fontSize: "var(--text-xs)" }}
                    value={editVolumeTitle}
                    onChange={e => setEditVolumeTitle(e.target.value)}
                    autoFocus
                    onKeyDown={e => e.key === "Enter" && handleRenameVolume(volume.volume_id)}
                  />
                  <button className="pv-icon-btn" title="保存" onClick={() => handleRenameVolume(volume.volume_id)}><Check size={13} /></button>
                  <button className="pv-icon-btn" title="取消" onClick={() => setEditingVolumeId(null)}><X size={13} /></button>
                </span>
              ) : (
                <>
                  <span className="volume-title">{volume.title}</span>
                  <span className="volume-actions" onClick={e => e.stopPropagation()}>
                    <button className="pv-icon-btn" title="重命名卷" onClick={() => startRenameVolume(volume)}><Edit3 size={13} /></button>
                    <button className="pv-icon-btn pv-icon-btn-danger" title="删除卷" onClick={() => handleDeleteVolume(volume)}><Trash2 size={13} /></button>
                  </span>
                </>
              )}
              <span className="volume-count">{volume.chapters.length} 章</span>
            </div>
            {volume.expanded && (
              <div className="chapter-list">
                {volume.chapters.map(ch => renderChapterRow(volume.volume_id, ch))}
                {renderAddChapter(volume.volume_id)}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="outline-stats">
        {arcs.length > 0 && (
          <>
            <span>脉络 {arcs.length} 段</span>
            <span className="outline-stat-sep">·</span>
          </>
        )}
        <span>已写 {totalChapters} 章</span>
        {outlineCount > 0 && (
          <>
            <span className="outline-stat-sep">·</span>
            <span>细纲 {outlineCount} 章</span>
          </>
        )}
        <span className="outline-stat-sep">·</span>
        <span>{totalWords.toLocaleString()} 字</span>
      </div>
    </div>
  );
}
