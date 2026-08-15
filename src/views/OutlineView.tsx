// OutlineView — 大纲（脉络与章节）

import { useState, useEffect, useCallback } from "react";
import {
  listOutlineArcs,
  listChapters,
  createOutlineArc,
  createChapter,
  updateOutlineArc,
  updateChapter,
  deleteOutlineArc,
  deleteChapter,
  detailGenerate,
  detailImport,
} from "../ipc";
import type { DetailItem } from "../ipc";
import type { OutlineArc, Chapter } from "../types";
import { chapterStatusLabels, label } from "../labels";

export default function OutlineView() {
  const [arcs, setArcs] = useState<OutlineArc[]>([]);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [newTitle, setNewTitle] = useState("");
  const [startCh, setStartCh] = useState("1");
  const [endCh, setEndCh] = useState("50");
  const [newChTitle, setNewChTitle] = useState("");
  const [msg, setMsg] = useState("");

  const [editArcId, setEditArcId] = useState<string | null>(null);
  const [editArcData, setEditArcData] = useState<any>({});
  const [editChId, setEditChId] = useState<string | null>(null);
  const [editChData, setEditChData] = useState<any>({});

  // P5 细纲化
  const [detailItems, setDetailItems] = useState<DetailItem[] | null>(null);
  const [detailBusy, setDetailBusy] = useState(false);
  const [detailModel, setDetailModel] = useState("");
  const [detailArcId, setDetailArcId] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [a, c] = await Promise.all([listOutlineArcs(), listChapters()]);
      setArcs(a); setChapters(c);
    } catch { setMsg("加载失败"); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleAddArc = async () => {
    if (!newTitle.trim()) return;
    try {
      await createOutlineArc(newTitle.trim(), parseInt(startCh) || 1, parseInt(endCh) || 50);
      setNewTitle(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleAddChapter = async () => {
    if (!newChTitle.trim()) return;
    try {
      await createChapter(newChTitle.trim());
      setNewChTitle(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteArc = async (arcId: string) => {
    if (!confirm("确定删除此脉络？")) return;
    try { await deleteOutlineArc(arcId); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteChapter = async (chapterId: string) => {
    if (!confirm("确定删除此章节？")) return;
    try { await deleteChapter(chapterId); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveArc = async () => {
    if (!editArcId) return;
    try {
      await updateOutlineArc(editArcId, editArcData);
      setEditArcId(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveChapter = async () => {
    if (!editChId) return;
    try {
      await updateChapter(editChId, editChData);
      setEditChId(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDetailGenerate = async () => {
    if (arcs.length === 0) {
      setMsg("请先创建大纲脉络，再一键细纲化。");
      return;
    }
    setDetailBusy(true);
    setMsg("");
    setDetailItems(null);
    try {
      const result = await detailGenerate(detailArcId || undefined);
      setDetailItems(result.items);
      setDetailModel(result.model);
      setMsg(result.note);
    } catch (e: any) {
      setMsg(`细纲化失败: ${e}`);
    } finally {
      setDetailBusy(false);
    }
  };

  const handleDetailUpdate = (index: number, field: keyof DetailItem, value: string) => {
    setDetailItems((prev) =>
      prev
        ? prev.map((item, i) =>
            i === index ? { ...item, [field]: field === "chapter_no" ? parseInt(value) || 0 : value } : item,
          )
        : prev,
    );
  };

  const handleDetailImport = async () => {
    if (!detailItems || detailItems.length === 0) return;
    setDetailBusy(true);
    try {
      const result = await detailImport(detailItems);
      setMsg(result.note);
      setDetailItems(null);
      const [a, c] = await Promise.all([listOutlineArcs(), listChapters()]);
      setArcs(a);
      setChapters(c);
    } catch (e: any) {
      setMsg(`细纲导入失败: ${e}`);
    } finally {
      setDetailBusy(false);
    }
  };

  return (
    <div className="view-card">
      <h2>大纲</h2>
      {msg && <p className="msg">{msg}</p>}

      <div className="section">
        <h3>情节脉络</h3>
        <div className="form-row">
          <input className="ps-input" placeholder="脉络标题" value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)} />
          <input className="ps-input ps-input-sm" placeholder="起始章" value={startCh}
            onChange={(e) => setStartCh(e.target.value)} />
          <input className="ps-input ps-input-sm" placeholder="结束章" value={endCh}
            onChange={(e) => setEndCh(e.target.value)} />
          <button className="btn-primary" onClick={handleAddArc}>添加</button>
        </div>

        {arcs.length > 0 ? (
          <div className="arc-list">
            {arcs.map((a) => (
              <div key={a.arc_id} className={`arc-item ${editArcId === a.arc_id ? "editing" : ""}`}>
                {editArcId === a.arc_id ? (
                  <div className="edit-form-inline">
                    <input className="ps-input" value={editArcData.title || ""} onChange={e => setEditArcData({...editArcData, title: e.target.value})} placeholder="标题" />
                    <input className="ps-input" value={editArcData.description || ""} onChange={e => setEditArcData({...editArcData, description: e.target.value})} placeholder="描述" />
                    <input className="ps-input ps-input-sm" value={editArcData.chapter_start || ""} onChange={e => setEditArcData({...editArcData, chapter_start: parseInt(e.target.value) || 0})} placeholder="起始" />
                    <input className="ps-input ps-input-sm" value={editArcData.chapter_end || ""} onChange={e => setEditArcData({...editArcData, chapter_end: parseInt(e.target.value) || 0})} placeholder="结束" />
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleSaveArc}>保存</button>
                      <button className="btn-sm" onClick={() => setEditArcId(null)}>取消</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="arc-header">
                      <span className="arc-title">{a.title}</span>
                      <span className="arc-range">第{a.chapter_start}-{a.chapter_end}章 ({a.chapter_count}章) · 已细纲至第{a.expanded_until || 0}章</span>
                    </div>
                    {a.description && <p className="arc-desc">{a.description}</p>}
                    <div className="btn-group">
                      <button className="btn-sm" onClick={() => { setEditArcId(a.arc_id); setEditArcData({title: a.title, description: a.description, chapter_start: a.chapter_start, chapter_end: a.chapter_end}); }}>编辑</button>
                      <button className="btn-sm btn-danger" onClick={() => handleDeleteArc(a.arc_id)}>删除</button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p className="empty">暂无大纲脉络。</p>
        )}
      </div>

      {/* P5 一键细纲化 */}
      <div className="section">
        <h3>细纲化（大纲 → 带标题的细纲 → 导入笔耕）</h3>
        <div className="form-row">
          <select
            className="ps-input"
            value={detailArcId}
            onChange={(e) => setDetailArcId(e.target.value)}
          >
            <option value="">全部脉络（分块生成）</option>
            {arcs.map((a) => (
              <option key={a.arc_id} value={a.arc_id}>
                {a.title}（第{a.chapter_start}-{a.chapter_end}章）
              </option>
            ))}
          </select>
          <button className="btn-sm" onClick={handleDetailGenerate} disabled={detailBusy || arcs.length === 0}>
            {detailBusy ? "生成中…" : "生成细纲"}
          </button>
          {detailItems && detailItems.length > 0 && (
            <button className="btn-primary" onClick={handleDetailImport} disabled={detailBusy}>
              导入笔耕
            </button>
          )}
        </div>
        <p className="llm-hint">
          建议每次选择一个脉络生成，便于逐段验收。系统会按每 12 章一块调用细纲 Agent，
          并注入核心概念、世界观、人物档案与风格笔记；生成后请先编辑确认，再「导入笔耕」。
        </p>
        {detailItems && detailItems.length > 0 && (
          <div className="detail-list">
            {detailItems.map((item, i) => (
              <div key={i} className="detail-item">
                <div className="form-row">
                  <input
                    className="ps-input ps-input-sm"
                    value={item.chapter_no}
                    onChange={(e) => handleDetailUpdate(i, "chapter_no", e.target.value)}
                    style={{ width: 70 }}
                  />
                  <input
                    className="ps-input"
                    value={item.title}
                    onChange={(e) => handleDetailUpdate(i, "title", e.target.value)}
                    placeholder="章节标题"
                  />
                </div>
                <textarea
                  className="ps-input ps-textarea detail-summary"
                  value={item.summary}
                  onChange={(e) => handleDetailUpdate(i, "summary", e.target.value)}
                  placeholder="细纲全文"
                />
              </div>
            ))}
            <p className="llm-hint">共 {detailItems.length} 章 · 模型 {detailModel || "—"}</p>
          </div>
        )}
      </div>

      <div className="section">
        <h3>章节列表 ({chapters.length})</h3>
        <div className="form-row">
          <input className="ps-input" placeholder="新章节标题" value={newChTitle}
            onChange={(e) => setNewChTitle(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAddChapter()} />
          <button className="btn-primary" onClick={handleAddChapter}>添加章节</button>
        </div>
        {chapters.length > 0 ? (
          <div className="chapter-list">
            {chapters.map((c) => (
              <div key={c.chapter_id} className={`chapter-item ${editChId === c.chapter_id ? "editing" : ""}`}>
                {editChId === c.chapter_id ? (
                  <div className="edit-form-inline">
                    <input className="ps-input" value={editChData.title || ""} onChange={e => setEditChData({...editChData, title: e.target.value})} placeholder="标题" />
                    <input className="ps-input" value={editChData.summary || ""} onChange={e => setEditChData({...editChData, summary: e.target.value})} placeholder="摘要" />
                    <select className="ps-input" value={editChData.status || ""} onChange={e => setEditChData({...editChData, status: e.target.value})}>
                      <option value="Draft">草稿</option>
                      <option value="Reviewing">审阅中</option>
                      <option value="Reviewed">已审阅</option>
                      <option value="Polished">已润色</option>
                      <option value="Published">已发布</option>
                    </select>
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleSaveChapter}>保存</button>
                      <button className="btn-sm" onClick={() => setEditChId(null)}>取消</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <span className="chapter-no">第{c.chapter_no}章</span>
                    <span className="chapter-title">{c.title || "未命名"}</span>
                    <span className="chapter-status">{label(chapterStatusLabels, c.status)}</span>
                    <span className="chapter-words">{c.word_count}字</span>
                    <div className="btn-group">
                      <button className="btn-sm" onClick={() => { setEditChId(c.chapter_id); setEditChData({title: c.title, summary: c.summary, status: c.status}); }}>编辑</button>
                      <button className="btn-sm btn-danger" onClick={() => handleDeleteChapter(c.chapter_id)}>删除</button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p className="empty">暂无章节。</p>
        )}
      </div>
    </div>
  );
}
