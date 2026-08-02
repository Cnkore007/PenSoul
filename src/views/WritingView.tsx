import { useState, useEffect, useMemo, useRef } from "react";
import { Save, BookOpen, ChevronRight, ChevronDown, FileText, Wand2, RotateCcw, Loader2, Trash2 } from "lucide-react";
import { TipTapEditor } from "../components/TipTapEditor";
import { AnnotationPanel } from "../components/AnnotationPanel";
import {
  saveChapter,
  reviewChapterChanges,
  applyChapterReview,
  analyzeChapterImpact,
  rewriteChapterWithAnnotations,
  listChapterRevisions,
  rollbackChapter,
  deleteChapter,
  getWritingLessons,
  saveWritingLessons,
  rewriteChapterDeai,
} from "../ipc";
import type { PageReview } from "../ipc";
import { messageDialog, confirmDialog } from "../dialogs";
import { ReviewConfirmModal } from "../components/ReviewConfirmModal";
import type {
  ChapterImpact,
  ProjectData,
  Chapter,
  ChapterAnnotation,
  AnnotationAnchor,
  ChapterRevision,
  WritingLesson,
  DeaiRewriteResult,
} from "../types";

interface WritingViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  chapterId: string | null;
  onWordCountChange: (count: number) => void;
}

export function WritingView({ projectData, persistProjectData, chapterId, onWordCountChange }: WritingViewProps) {
  const [chapter, setChapter] = useState<Chapter | null>(null);
  const [content, setContent] = useState("");
  const [annotations, setAnnotations] = useState<ChapterAnnotation[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  // 受控保存：判定面板状态
  const [review, setReview] = useState<PageReview | null>(null);
  const [verdicts, setVerdicts] = useState<Record<string, string>>({});
  const [selectedId, setSelectedId] = useState<string | null>(chapterId);
  const [expandedVolumes, setExpandedVolumes] = useState<Record<string, boolean>>({});
  const [showNav, setShowNav] = useState(true);
  const [impact, setImpact] = useState<ChapterImpact | null>(null);
  // 按批注重写：进度 / 结果 / 版本历史
  const [rewriting, setRewriting] = useState(false);
  const [rewriteMsg, setRewriteMsg] = useState<string | null>(null);
  const [rewriteResult, setRewriteResult] = useState<{
    new_version: number;
    accepted: string[];
    rejected: string[];
    untouched: string[];
    plan_summary: string;
    lessons: WritingLesson[];
  } | null>(null);
  // 去 AI 味重写：进度 / 结果（建议删除清单、保真问题、残留问题）
  const [deaiRewriting, setDeaiRewriting] = useState(false);
  const [deaiResult, setDeaiResult] = useState<DeaiRewriteResult | null>(null);
  const [revisions, setRevisions] = useState<ChapterRevision[]>([]);
  const [showRevisions, setShowRevisions] = useState(true);
  // 项目写作经验库（批注重写沉淀，注入审查）
  const [lessons, setLessons] = useState<WritingLesson[]>([]);
  const [showLessons, setShowLessons] = useState(false);
  const editorWrapRef = useRef<HTMLDivElement | null>(null);

  // 纯文本 → HTML 段落（管线写入的章节是纯文本，TipTap 需要段落标签才分得清段落）
  function toHtml(text: string): string {
    if (text.includes("<p") || text.includes("<h")) return text;
    return text
      .split(/\n{2,}/)
      .map(p => `<p>${p.replace(/\n/g, "<br>")}</p>`)
      .join("");
  }

  // HTML → 纯文本（统一落库格式，与后端一致性/记忆处理一致）
  function toPlain(html: string): string {
    return html
      .replace(/<br\s*\/?>/gi, "\n")
      .replace(/&nbsp;/gi, " ")
      .replace(/<[^>]*>/g, "")
      .trim();
  }

  // 在 HTML 中按批注锚点（段落索引 + 原文片段）包裹高亮标记
  function applyAnnotations(html: string, annos: ChapterAnnotation[]): string {
    const inline = annos.filter(a => a.anchor);
    if (inline.length === 0) return html;
    // 按顶层段落切分（保留标签），段落索引与编辑器顶层节点顺序一致
    const parts = html.split(/(<p[^>]*>.*?<\/p>|<h[12][^>]*>.*?<\/h[12]>)/gs);
    let paraIdx = -1;
    return parts
      .map(seg => {
        const m = seg.match(/^<(p|h[12])([^>]*)>/);
        if (!m) return seg;
        paraIdx += 1;
        const tag = m[1];
        const inner = seg.slice(m[0].length, seg.length - `</${tag}>`.length);
        const plain = inner
          .replace(/<br\s*\/?>/gi, "\n")
          .replace(/&nbsp;/gi, " ")
          .replace(/<[^>]*>/g, "")
          .trim();
        const segAnnos = inline
          .filter(a => a.anchor!.paragraph_index === paraIdx)
          .sort((a, b) => b.anchor!.offset - a.anchor!.offset);
        if (segAnnos.length === 0) return seg;
        let result = plain;
        for (const a of segAnnos) {
          const text = a.anchor!.text.trim();
          if (!text) continue;
          // 优先按锚点偏移匹配，失败则全文查找（正文被改过的容错）
          let start = result.indexOf(text, Math.min(a.anchor!.offset, result.length));
          if (start === -1) start = result.indexOf(text);
          if (start === -1) continue;
          const end = start + text.length;
          result =
            result.slice(0, start) +
            `<span class="anno-marker" data-anno-id="${a.annotation_id}">` +
            result.slice(start, end) +
            "</span>" +
            result.slice(end);
        }
        return `<${tag}${m[2]}>${result}</${tag}>`;
      })
      .join("");
  }

  // 笔耕只显示已开始细写（有正文）的章节
  const writingVolumes = useMemo(
    () => projectData.volumes
      .map(v => ({ ...v, chapters: v.chapters.filter(c => c.word_count > 0) }))
      .filter(v => v.chapters.length > 0),
    [projectData.volumes]
  );

  useEffect(() => {
    const expanded: Record<string, boolean> = {};
    writingVolumes.forEach(v => { expanded[v.volume_id] = true; });
    setExpandedVolumes(expanded);
  }, [writingVolumes]);

  useEffect(() => {
    if (chapterId) setSelectedId(chapterId);
  }, [chapterId]);

  // 加载项目写作经验库
  useEffect(() => {
    getWritingLessons().then(setLessons).catch(() => {});
  }, []);

  // 删除一条经验并落库
  async function handleRemoveLesson(lessonId: string) {
    const next = lessons.filter(l => l.lesson_id !== lessonId);
    setLessons(next);
    await saveWritingLessons(next).catch(() => {});
  }

  // 选中的章节变化时加载内容
  useEffect(() => {
    if (!selectedId) { setChapter(null); setContent(""); setAnnotations([]); return; }
    for (const vol of projectData.volumes) {
      const ch = vol.chapters.find(c => c.chapter_id === selectedId);
      if (ch) {
        setChapter(ch);
        setAnnotations(ch.annotations ?? []);
        setContent(applyAnnotations(toHtml(ch.content), ch.annotations ?? []));
        setImpact(null);
        setRewriteResult(null);
        setRevisions(ch.revisions ?? []);
        onWordCountChange(ch.word_count);
        return;
      }
    }
    setChapter(null); setContent("");
  }, [selectedId, projectData, onWordCountChange]);

  // 字数统计
  useEffect(() => {
    const plainText = content.replace(/<[^>]*>/g, "");
    onWordCountChange(plainText.length);
  }, [content, onWordCountChange]);

  async function handleSave() {
    if (!chapter) return;
    setSaving(true);
    const plainText = toPlain(content);
    try {
      // 受控保存：先让 LLM 判定本章批注/修改的有效性与影响，二次确认后再落库
      const r = await reviewChapterChanges(chapter.chapter_id, plainText);
      setReview(r);
      const v: Record<string, string> = {};
      for (const item of r.items) v[item.id] = item.verdict || "uncertain";
      setVerdicts(v);
    } catch (e: any) {
      await messageDialog("审核分析失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    } finally {
      setSaving(false);
    }
  }

  // 用户二次确认后真正落库
  async function handleApplyReview() {
    if (!chapter || !review) return;
    setSaving(true);
    const plainText = toPlain(content);
    try {
      const confirmations = review.items.map(item => ({ id: item.id, verdict: verdicts[item.id] ?? "uncertain" }));
      const result = await applyChapterReview(chapter.chapter_id, plainText, confirmations);
      const newVersion = result.new_version;
      const updated: Chapter = {
        ...chapter,
        content: plainText,
        word_count: plainText.length,
        version: newVersion,
        annotations,
        revisions,
      };
      persistProjectData(prev => ({
        ...prev,
        volumes: prev.volumes.map(v => ({
          ...v,
          chapters: v.chapters.map(c => c.chapter_id === chapter.chapter_id ? updated : c),
        })),
      }));
      setChapter(updated);
      setReview(null);
      setContent(applyAnnotations(toHtml(plainText), annotations));
      // 刷新版本历史（受控保存已把旧版入快照，前端本地列表需同步）
      try {
        const revs = await listChapterRevisions(chapter.chapter_id);
        setRevisions(revs ?? []);
      } catch {
        // 版本列表刷新失败不阻塞保存流程
      }
      const report = await analyzeChapterImpact(chapter.chapter_id);
      setImpact(report);
      const nAffected = Array.isArray(report.affected) ? report.affected.length : 0;
      const nIssues = Array.isArray(report.consistency) ? report.consistency.length : 0;
      setSaveMsg(`已保存 · 沉淀 ${result.lessons.length} 条经验 · 影响 ${nAffected} 处 · 一致性提示 ${nIssues} 项`);
    } catch (e: any) {
      await messageDialog("保存失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    } finally {
      setSaving(false);
      setTimeout(() => setSaveMsg(null), 4000);
    }
  }

  // ── 批注操作 ──
  function handleAddAnnotation(anchor: AnnotationAnchor, kind: ChapterAnnotation["kind"], text: string): string | null {
    if (!chapter) return null;
    const id = `anno-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const anno: ChapterAnnotation = {
      annotation_id: id,
      kind,
      anchor,
      content: text,
      status: "open",
      created_at: new Date().toISOString(),
    };
    setAnnotations(prev => [...prev, anno]);
    return id;
  }

  function handleAddChapterAnnotation(kind: ChapterAnnotation["kind"], text: string) {
    if (!chapter) return;
    const id = `anno-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const anno: ChapterAnnotation = {
      annotation_id: id,
      kind,
      anchor: null,
      content: text,
      status: "open",
      created_at: new Date().toISOString(),
    };
    setAnnotations(prev => [...prev, anno]);
  }

  function handleUpdateAnnotation(id: string, patch: Partial<ChapterAnnotation>) {
    setAnnotations(prev => prev.map(a => (a.annotation_id === id ? { ...a, ...patch } : a)));
  }

  function handleRemoveAnnotation(id: string) {
    setAnnotations(prev => prev.filter(a => a.annotation_id !== id));
  }

  // 定位到正文中的锚定段落
  function handleLocate(anno: ChapterAnnotation) {
    const wrap = editorWrapRef.current;
    if (!wrap || !anno.anchor) return;
    const paras = wrap.querySelectorAll<HTMLElement>("p, h1, h2");
    const para = paras[anno.anchor.paragraph_index];
    if (!para) return;
    // 若有锚定文本，进一步定位到文本并高亮
    const walker = document.createTreeWalker(para, NodeFilter.SHOW_TEXT);
    let node: Node | null;
    while ((node = walker.nextNode())) {
      const idx = node.textContent?.indexOf(anno.anchor.text) ?? -1;
      if (idx >= 0) {
        const range = document.createRange();
        range.setStart(node, idx);
        range.setEnd(node, idx + anno.anchor.text.length);
        const rect = range.getBoundingClientRect();
        window.scrollBy({ top: rect.top - window.innerHeight / 3, behavior: "smooth" });
        return;
      }
    }
    para.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  // ── 按批注重写 ──
  const openAnnoCount = annotations.filter(a => a.status === "open").length;

  async function handleRewrite() {
    if (!chapter || openAnnoCount === 0 || rewriting) return;
    const ok = await confirmDialog(
      `按 ${openAnnoCount} 条待处理批注重写本章？重写后生成新版本，原稿保留在版本历史中可回滚。`
    );
    if (!ok) return;
    setRewriting(true);
    setRewriteResult(null);
    setRewriteMsg("正在整理修改计划…");
    try {
      // 先把未落盘的批注与正文保存，重写命令读后端已保存的批注，
      // 避免「批注后没点保存直接重写」读不到新批注
      const plainText = toPlain(content);
      const savedVersion = await saveChapter(chapter.chapter_id, plainText, chapter.version, annotations);
      if (savedVersion !== chapter.version) {
        setChapter(prev => (prev ? { ...prev, content: plainText, word_count: plainText.length, version: savedVersion, annotations } : prev));
      }
      const stageCfg = (projectData as any).workflowSkills?.chapter_writing;
      const res = await rewriteChapterWithAnnotations(
        chapter.chapter_id,
        stageCfg?.model ?? null,
        stageCfg?.cards ?? null
      );
      setRewriteResult(res);
      setRewriteMsg(null);
      // 从后端重载章节与版本历史
      const revs = await listChapterRevisions(chapter.chapter_id);
      setRevisions(revs);
      const updated: Chapter = {
        ...chapter,
        version: res.new_version,
        content: "",
        word_count: 0,
        annotations: chapter.annotations ?? [],
        revisions: revs,
      };
      // 用后端最新章节数据刷新（通过 getChapter 拉取更准确）
      const latest = await import("../ipc").then(m => m.getChapter(chapter.chapter_id));
      if (latest) {
        const fresh: Chapter = {
          ...chapter,
          content: latest.content ?? "",
          word_count: latest.word_count ?? 0,
          version: latest.version ?? res.new_version,
          annotations: latest.annotations ?? chapter.annotations ?? [],
          revisions: latest.revisions ?? revs,
        };
        setChapter(fresh);
        setAnnotations(fresh.annotations ?? []);
        setContent(applyAnnotations(toHtml(fresh.content), fresh.annotations ?? []));
        persistProjectData(prev => ({
          ...prev,
          volumes: prev.volumes.map(v => ({
            ...v,
            chapters: v.chapters.map(c => (c.chapter_id === chapter.chapter_id ? fresh : c)),
          })),
        }));
        const report = await analyzeChapterImpact(chapter.chapter_id);
        setImpact(report);
      }
      void updated;
    } catch (e: any) {
      setRewriteMsg(null);
      await messageDialog("批注重写失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    } finally {
      setRewriting(false);
    }
  }

  // ── 版本回滚 ──
  async function handleRollback(rev: ChapterRevision) {
    if (!chapter) return;
    const ok = await confirmDialog(`回滚到第 ${rev.version} 版？当前版不保留。`);
    if (!ok) return;
    try {
      const res = await rollbackChapter(chapter.chapter_id, rev.version);
      const newVersion = res.new_version;
      const nextRevs = res.revisions;
      const updated: Chapter = {
        ...chapter,
        content: rev.content,
        word_count: rev.word_count ?? rev.content.length,
        version: newVersion,
        revisions: nextRevs,
      };
      setChapter(updated);
      setContent(applyAnnotations(toHtml(rev.content), updated.annotations ?? []));
      setRevisions(nextRevs);
      persistProjectData(prev => ({
        ...prev,
        volumes: prev.volumes.map(v => ({
          ...v,
          chapters: v.chapters.map(c => (c.chapter_id === chapter.chapter_id ? updated : c)),
        })),
      }));
      const report = await analyzeChapterImpact(chapter.chapter_id);
      setImpact(report);
      setSaveMsg(`已回滚到第 ${newVersion} 版`);
      setTimeout(() => setSaveMsg(null), 4000);
    } catch (e: any) {
      await messageDialog("回滚失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    }
  }

  // 顶部「撤回上一版」：回滚到最近一次历史快照
  function handleUndoLatest() {
    if (!chapter || revisions.length === 0) return;
    const latest = revisions[revisions.length - 1];
    void handleRollback(latest);
  }

  // ── 去 AI 味重写（保真账本 → 有界改写 → 两步回读 → 建议删除清单） ──
  async function handleDeaiRewrite() {
    if (!chapter || deaiRewriting) return;
    if (toPlain(content).trim().length === 0) {
      await messageDialog("本章还没有正文，无法去 AI 味重写。");
      return;
    }
    const ok = await confirmDialog(
      "对本章做去 AI 味重写？\n\n规则：只做句内清洗，不新增/删除事实；整句空话进「建议删除清单」由你确认后再删；重写后生成新版本，原稿保留在版本历史中可回滚。"
    );
    if (!ok) return;
    setDeaiRewriting(true);
    setDeaiResult(null);
    setRewriteMsg("正在做保真账本与有界改写…");
    try {
      // 先保存未落盘的正文，重写命令读后端已保存内容
      const plainText = toPlain(content);
      const savedVersion = await saveChapter(chapter.chapter_id, plainText, chapter.version, annotations);
      if (savedVersion !== chapter.version) {
        setChapter(prev => (prev ? { ...prev, content: plainText, word_count: plainText.length, version: savedVersion, annotations } : prev));
      }
      const stageCfg = (projectData as any).workflowSkills?.chapter_writing;
      const res = await rewriteChapterDeai(
        chapter.chapter_id,
        stageCfg?.model ?? null,
        stageCfg?.cards ?? null
      );
      setDeaiResult(res);
      setRewriteMsg(null);
      // 从后端重载章节与版本历史
      const revs = await listChapterRevisions(chapter.chapter_id);
      setRevisions(revs);
      const latest = await import("../ipc").then(m => m.getChapter(chapter.chapter_id));
      if (latest) {
        const fresh: Chapter = {
          ...chapter,
          content: latest.content ?? "",
          word_count: latest.word_count ?? 0,
          version: latest.version ?? res.new_version,
          annotations: latest.annotations ?? chapter.annotations ?? [],
          revisions: latest.revisions ?? revs,
        };
        setChapter(fresh);
        setAnnotations(fresh.annotations ?? []);
        setContent(applyAnnotations(toHtml(fresh.content), fresh.annotations ?? []));
        persistProjectData(prev => ({
          ...prev,
          volumes: prev.volumes.map(v => ({
            ...v,
            chapters: v.chapters.map(c => (c.chapter_id === chapter.chapter_id ? fresh : c)),
          })),
        }));
      }
    } catch (e: any) {
      setRewriteMsg(null);
      await messageDialog("去 AI 味重写失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    } finally {
      setDeaiRewriting(false);
    }
  }

  // 删除章节：笔耕导航内直接删除（正文一并删除，不可恢复）
  async function handleDeleteChapter(ch: Chapter) {
    const hint = ch.word_count > 0
      ? `删除章节「${ch.title}」？已写入的 ${ch.word_count} 字正文将一并删除，不可恢复。`
      : `删除章节「${ch.title}」？`;
    if (!(await confirmDialog(hint))) return;
    const volId = projectData.volumes.find(v => v.chapters.some(c => c.chapter_id === ch.chapter_id))?.volume_id;
    if (!volId) return;
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId
        ? { ...v, chapters: v.chapters.filter(c => c.chapter_id !== ch.chapter_id), chapter_count: v.chapters.length - 1 }
        : v),
    }));
    if (selectedId === ch.chapter_id) {
      setSelectedId(null);
      setChapter(null);
      setContent("");
      setAnnotations([]);
    }
    deleteChapter(ch.chapter_id).catch(err => console.error("删除章节失败:", err));
  }

  function toggleVolume(volId: string) {
    setExpandedVolumes(prev => ({ ...prev, [volId]: !prev[volId] }));
  }

  const totalChapters = useMemo(
    () => writingVolumes.reduce((s, v) => s + v.chapters.length, 0),
    [writingVolumes]
  );

  return (
    <div className="writing-layout">
      {showNav && totalChapters > 0 && (
        <div className="writing-nav">
          <div className="writing-nav-header">
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-sm)", letterSpacing: "1px" }}>章节导航</span>
          </div>
          <div className="writing-nav-list">
            {writingVolumes.map(volume => (
              <div key={volume.volume_id} className="writing-nav-volume">
                <div className="writing-nav-vol-header" onClick={() => toggleVolume(volume.volume_id)}>
                  {expandedVolumes[volume.volume_id] ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                  <span className="writing-nav-vol-title">{volume.volume_id === "_default" ? "未分卷" : volume.title}</span>
                  <span className="writing-nav-vol-count">{volume.chapters.length} 章</span>
                </div>
                {expandedVolumes[volume.volume_id] && volume.chapters.map(ch => (
                  <div key={ch.chapter_id} className={`writing-nav-chapter ${selectedId === ch.chapter_id ? "active" : ""}`}
                    onClick={() => setSelectedId(ch.chapter_id)}>
                    <FileText size={12} />
                    <span className="writing-nav-ch-title">{ch.title}</span>
                    <span className="writing-nav-ch-words">{ch.word_count.toLocaleString()}</span>
                    <button
                      className="writing-nav-ch-del"
                      title="删除章节"
                      onClick={e => { e.stopPropagation(); handleDeleteChapter(ch); }}
                    >
                      <Trash2 size={11} />
                    </button>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}

      {totalChapters === 0 && (
        <div className="writing-nav" style={{ justifyContent: "center", alignItems: "center", display: "flex" }}>
          <div style={{ textAlign: "center", color: "var(--color-ink-faint)", fontSize: "var(--text-sm)" }}>
            <FileText size={24} strokeWidth={1} style={{ marginBottom: 8, opacity: 0.4 }} />
            <div>暂无写作中的章节</div>
            <div style={{ fontSize: "var(--text-xs)", marginTop: 4 }}>章节经工作流细写后会出现在这里</div>
          </div>
        </div>
      )}

      <div className="writing-main">
        <div className="view-header" style={{ paddingLeft: showNav && totalChapters > 0 ? "var(--space-sm)" : undefined }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
            {totalChapters > 0 && (
              <button className="btn btn-ghost" style={{ padding: "4px", flexShrink: 0 }}
                onClick={() => setShowNav(!showNav)} title={showNav ? "隐藏导航" : "显示导航"}>
                {showNav ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
              </button>
            )}
            <h2 style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {chapter?.title || (totalChapters > 0 ? "选择章节" : "笔耕")}
            </h2>
            {chapter && <span className={`badge badge-${chapter.status.toLowerCase()}`}>{chapter.status}</span>}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <button className="btn btn-secondary" onClick={handleDeaiRewrite} disabled={!chapter || deaiRewriting || saving} title="保真账本 + 有界改写 + 两步回读，整句空话进建议删除清单">
              {deaiRewriting ? <><Loader2 size={15} className="spinning" /> 去AI味中…</> : <><Wand2 size={15} /> 去AI味重写</>}
            </button>
            {chapter && openAnnoCount > 0 && (
              <button className="btn btn-accent" onClick={handleRewrite} disabled={rewriting}>
                {rewriting ? <><Loader2 size={15} className="spinning" /> 重写中…</> : <><Wand2 size={15} /> 按批注重写本章（{openAnnoCount}）</>}
              </button>
            )}
            {chapter && revisions.length > 0 && (
              <button className="btn btn-secondary" onClick={handleUndoLatest} disabled={saving || rewriting} title={`撤回最近一次保存（版本历史共 ${revisions.length} 条）`}>
                <RotateCcw size={15} /> 撤回上一版
              </button>
            )}
            <button className={"btn btn-primary" + (saving || !chapter ? " btn-disabled" : "")} onClick={handleSave} disabled={saving || !chapter} title="审核本章批注与修改后保存">
              <Save size={15} /> {saving ? "审核中..." : "保存并审核"}
            </button>
          </div>
        </div>
        {saveMsg && <div className="save-message success">{saveMsg}</div>}
        {rewriteMsg && <div className="save-message success"><Loader2 size={13} className="spinning" style={{ verticalAlign: -2, marginRight: 6 }} />{rewriteMsg}</div>}
        {review && (
          <ReviewConfirmModal
            review={review}
            verdicts={verdicts}
            setVerdicts={setVerdicts}
            applying={saving}
            onConfirm={handleApplyReview}
            onCancel={() => setReview(null)}
          />
        )}
        {rewriteResult && (
          <div className="save-message success" style={{ whiteSpace: "pre-wrap" }}>
            重写完成（第 {rewriteResult.new_version} 版）。{rewriteResult.plan_summary}
            {rewriteResult.accepted.length > 0 && ` 采纳 ${rewriteResult.accepted.length} 条；`}
            {rewriteResult.rejected.length > 0 && `拒绝 ${rewriteResult.rejected.length} 条；`}
            {rewriteResult.untouched.length > 0 && `未处理 ${rewriteResult.untouched.length} 条。`}
            {rewriteResult.lessons.length > 0 && ` 本次沉淀写作经验 ${rewriteResult.lessons.length} 条（注入后续审查）。`}
          </div>
        )}
        {deaiResult && (
          <div className="save-message success" style={{ whiteSpace: "pre-wrap" }}>
            去 AI 味重写完成（第 {deaiResult.new_version} 版，{deaiResult.original_word_count} → {deaiResult.word_count} 字）。
            {deaiResult.repaired ? ` 检测到 ${deaiResult.fidelity_issues.length} 项保真问题并已修复；` : " 保真回读未发现问题；"}
            {deaiResult.summary && ` ${deaiResult.summary}`}
            {deaiResult.suggested_deletions.length > 0 && (
              <>
                {"\n\n【建议删除清单】以下整句空话未删，确认无信息丢失后可手动删除：\n"}
                {deaiResult.suggested_deletions.map(d => `- ${d.sentence}（${d.reason}）`).join("\n")}
              </>
            )}
            {deaiResult.residual_issues.length > 0 && (
              <>
                {"\n\n【残留提示】不改写全文，仅提示后续写作注意：\n"}
                {deaiResult.residual_issues.map(r => `- ${r}`).join("\n")}
              </>
            )}
          </div>
        )}
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }} ref={editorWrapRef}>
          {chapter ? (
            <TipTapEditor
              key={chapter.chapter_id + ":" + chapter.version}
              content={content}
              onChange={setContent}
              annotations={annotations}
              onAddAnnotation={handleAddAnnotation}
              placeholder="落笔之处，便是江湖..."
            />
          ) : (
            <div className="empty-state" style={{ flex: 1 }}>
              <div className="empty-state-icon">笔</div>
              <div className="empty-state-text">{totalChapters > 0 ? "选择章节开始创作" : "笔墨未落，尚待挥毫"}</div>
              <div className="empty-state-sub">{totalChapters > 0 ? "从左侧导航选择一个章节" : "章节由工作流细写后，在此打磨正文"}</div>
            </div>
          )}
        </div>
      </div>

      {chapter && (
        <div className="card writing-info">
          <div className="card-header"><BookOpen size={15} color="var(--color-ink-3)" /><h3>章节信息</h3></div>

          <AnnotationPanel
            annotations={annotations}
            onAddChapterAnnotation={handleAddChapterAnnotation}
            onUpdate={handleUpdateAnnotation}
            onRemove={handleRemoveAnnotation}
            onLocate={handleLocate}
          />

          <div className="writing-info-section">
            <div className="writing-info-label">字数</div>
            <div className="writing-info-value-lg">{chapter.word_count.toLocaleString()}</div>
            <div className="writing-info-unit">字</div>
          </div>
          <div className="writing-info-section">
            <div className="writing-info-label">版本</div>
            <div className="writing-info-value">第 {chapter.version} 版</div>
          </div>
          <div className="writing-info-section">
            <div className="writing-info-label">进度</div>
            <span className={`badge badge-${chapter.status.toLowerCase()}`}>{chapter.status}</span>
          </div>

          {/* 版本历史（批注重写快照 / 回滚） */}
          {revisions.length > 0 && (
            <div className="writing-info-section">
              <div className="writing-info-label">
                版本历史
                <button className="pv-icon-btn" style={{ marginLeft: "auto" }}
                  onClick={() => setShowRevisions(!showRevisions)}>
                  {showRevisions ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                </button>
              </div>
              {showRevisions && (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {[...revisions].reverse().map(rev => (
                    <div key={rev.version} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-2xs)", color: "var(--color-ink-2)" }}>
                      <span style={{ flexShrink: 0 }}>第 {rev.version} 版</span>
                      <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "var(--color-ink-3)" }}>
                        {rev.reason || "快照"}
                      </span>
                      <button className="btn btn-secondary" style={{ padding: "1px 8px", fontSize: "var(--text-2xs)", flexShrink: 0 }}
                        onClick={() => handleRollback(rev)}>
                        <RotateCcw size={11} /> 回滚
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* 项目写作经验库（重写沉淀，注入审查避错） */}
          {lessons.length > 0 && (
            <div className="writing-info-section">
              <div className="writing-info-label">
                写作经验库
                <span style={{ fontWeight: 400, color: "var(--color-ink-3)", fontSize: "var(--text-2xs)", marginLeft: 6 }}>
                  {lessons.length} 条 · 注入章节审查
                </span>
                <button className="pv-icon-btn" style={{ marginLeft: "auto" }}
                  onClick={() => setShowLessons(!showLessons)}>
                  {showLessons ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                </button>
              </div>
              {showLessons && (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {lessons.map(l => (
                    <div key={l.lesson_id} style={{ fontSize: "var(--text-2xs)", lineHeight: 1.6, padding: "var(--space-xs) var(--space-sm)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <span style={{ padding: "0 6px", borderRadius: 8, background: "var(--color-accent-wash)", color: "var(--color-accent)", fontWeight: 600 }}>
                          {l.category}
                        </span>
                        {l.count && l.count > 1 && (
                          <span style={{ color: "var(--color-ochre)" }}>已发生 {l.count} 次</span>
                        )}
                        <button className="pv-icon-btn pv-icon-btn-danger" style={{ marginLeft: "auto" }}
                          title="删除经验" onClick={() => handleRemoveLesson(l.lesson_id)}>
                          <Trash2 size={12} />
                        </button>
                      </div>
                      <div style={{ color: "var(--color-ink-2)" }}>{l.problem}</div>
                      {l.fix && <div style={{ color: "var(--color-jade)" }}>改正：{l.fix}</div>}
                      {l.example && <div style={{ color: "var(--color-ink-3)" }}>出自{l.example}</div>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {impact && (
            <div className="writing-info-section">
              <div className="writing-info-label">
                影响分析
                <span style={{ fontWeight: 400, color: "var(--color-ink-3)", fontSize: "var(--text-2xs)", marginLeft: 6 }}>
                  第 {impact.chapter_no} 章修改后
                </span>
              </div>
              <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-2)", lineHeight: 1.7 }}>
                <div style={{ marginBottom: 6 }}>已同步：记忆 / 影响图 / 一致性状态 / 并发版本</div>
                {Array.isArray(impact.affected) && impact.affected.length > 0 ? (
                  <details style={{ marginBottom: 6 }}>
                    <summary style={{ color: "var(--color-accent)", cursor: "pointer" }}>受影响 {impact.affected.length} 处（点击展开）</summary>
                    <ul style={{ margin: "6px 0 0", paddingLeft: 16 }}>
                      {impact.affected.map((a, i) => (
                        <li key={i}>
                          {a.node_id ?? a.chapter_id ?? a.entity_id ?? "节点"}
                          {a.severity ? `（${a.severity}）` : ""}
                          {a.action ? `：${a.action}` : ""}
                        </li>
                      ))}
                    </ul>
                  </details>
                ) : (
                  <div style={{ marginBottom: 6 }}>未发现其他章节受影响</div>
                )}
                {Array.isArray(impact.consistency) && impact.consistency.length > 0 ? (
                  <details>
                    <summary style={{ color: "var(--color-ochre)", cursor: "pointer" }}>一致性提示 {impact.consistency.length} 项（点击展开）</summary>
                    <ul style={{ margin: "6px 0 0", paddingLeft: 16 }}>
                      {impact.consistency.map((v, i) => (
                        <li key={i}>
                          {v.rule_name ?? "一致性"}：{v.description ?? ""}
                          {v.suggested_fix ? `（建议：${v.suggested_fix}）` : ""}
                        </li>
                      ))}
                    </ul>
                  </details>
                ) : (
                  <div>一致性检查通过</div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
