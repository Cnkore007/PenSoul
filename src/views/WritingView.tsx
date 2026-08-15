// WritingView — 笔耕（章节正文编辑）
// 集成：AI 初稿/续写（记忆检索+约束+技巧注入）、AI 审校、写作风格笔记

import { useState, useEffect, useCallback } from "react";
import {
  extractFacts,
  listChapters,
  getChapterContent,
  saveChapterContent,
  generateWriting,
  listTechniques,
  reviewWriting,
  getWorldStyle,
  updateWorldStyle,
  aiRewrite,
  addAnnotation,
  updateAnnotationStatus,
  deleteAnnotation,
  cascadeAnalyze,
  cascadeApply,
  batchWrite,
} from "../ipc";
import type { Chapter } from "../types";
import type { WritingResult, Technique, ReviewResult, StyleNotes, RewriteResult, ChapterAnnotation, CascadeAnalyzeResult, CascadeApplyResult, BatchWriteItem } from "../ipc";
import { chapterStatusLabels, label } from "../labels";

export default function WritingView() {
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [selected, setSelected] = useState<Chapter | null>(null);
  const [content, setContent] = useState("");
  const [revisionCount, setRevisionCount] = useState(0);
  const [msg, setMsg] = useState("");
  const [saving, setSaving] = useState(false);
  const [genLoading, setGenLoading] = useState(false);
  const [genMeta, setGenMeta] = useState<WritingResult | null>(null);

  // 技巧库（F12/F15）
  const [techniques, setTechniques] = useState<Technique[]>([]);
  const [selectedTechniques, setSelectedTechniques] = useState<string[]>([]);

  // AI 审校（F3 完整版 / F4 / F8）
  const [reviewing, setReviewing] = useState(false);
  const [reviewResult, setReviewResult] = useState<ReviewResult | null>(null);

  // 写作风格笔记（F13，正典 AestheticLayer）
  const [style, setStyle] = useState<StyleNotes>({ style_notes: "", pacing_notes: "" });
  const [styleSaving, setStyleSaving] = useState(false);

  // P2 批注 + AI 审核改写
  const [annotations, setAnnotations] = useState<ChapterAnnotation[]>([]);
  const [annotationKind, setAnnotationKind] = useState("建议");
  const [annotationInput, setAnnotationInput] = useState("");
  const [rewriteInstructions, setRewriteInstructions] = useState("");
  const [rewriting, setRewriting] = useState(false);
  const [rewriteResult, setRewriteResult] = useState<RewriteResult | null>(null);

  // P4 级联同步
  const [initialContent, setInitialContent] = useState("");
  const [cascadeResult, setCascadeResult] = useState<CascadeAnalyzeResult | null>(null);
  const [cascadeSelected, setCascadeSelected] = useState<string[]>([]);
  const [cascadeBusy, setCascadeBusy] = useState(false);
  const [cascadeApplied, setCascadeApplied] = useState<CascadeApplyResult | null>(null);

  // P5 批量写作（每 3 章一批，检查点）
  const [batchDrafts, setBatchDrafts] = useState<BatchWriteItem[] | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setChapters(await listChapters());
    } catch {
      setMsg("加载失败");
    }
  }, []);

  useEffect(() => {
    refresh();
    listTechniques()
      .then(setTechniques)
      .catch(() => setMsg("技巧库加载失败"));
  }, [refresh]);

  const handleSelect = async (c: Chapter) => {
    // P1-2 修复：未保存修改丢失前必须显式确认
    if (selected && content !== initialContent) {
      const ok = confirm(
        `第${selected.chapter_no}章有未保存的修改，切换后修改将丢失。确定切换？`,
      );
      if (!ok) return;
    }
    setSelected(c);
    setContent("");
    setRevisionCount(0);
    setGenMeta(null);
    setReviewResult(null);
    setAnnotations([]);
    setRewriteResult(null);
    setRewriteInstructions("");
    setCascadeResult(null);
    setCascadeApplied(null);
    setCascadeSelected([]);
    setBatchDrafts(null);
    try {
      const detail = await getChapterContent(c.chapter_id);
      setContent(detail.content);
      setInitialContent(detail.content);
      setRevisionCount(detail.revision_count);
      setAnnotations(detail.annotations || []);
    } catch (e: any) {
      setMsg(`正文加载失败: ${e}`);
    }
    getWorldStyle().then(setStyle).catch(() => { /* 未打开项目时忽略 */ });
  };

  const toggleTechnique = (id: string) => {
    setSelectedTechniques((prev) =>
      prev.includes(id) ? prev.filter((t) => t !== id) : [...prev, id],
    );
    setReviewResult(null);
  };

  const refreshChapterMeta = async (chapterId: string) => {
    // P2-11 修复：保存后刷新正典元信息（版本/修订/一致性/批注），头部不再 stale。
    // 仅当保存的是当前打开章节时才切换 selected 并刷新编辑器元信息；
    // 批量/级联保存其他章节时不得把编辑器偷偷切走。
    try {
      const cs = await listChapters();
      setChapters(cs);
      if (selected?.chapter_id === chapterId) {
        const cur = cs.find((c) => c.chapter_id === chapterId);
        if (cur) setSelected(cur);
        const detail = await getChapterContent(chapterId);
        setRevisionCount(detail.revision_count);
        setAnnotations(detail.annotations || []);
      }
    } catch {
      /* 元信息刷新失败不阻塞保存结果 */
    }
  };

  const formatExtractReport = (report: any, label: string) => {
    const bits: string[] = [];
    if (report.applied.length > 0) bits.push(`已更新 ${report.applied.length} 项档案`);
    if (report.warnings.length > 0) bits.push(`警告 ${report.warnings.length} 条`);
    const base = `${label}已保存。档案事实提取完成：${bits.join("，") || "无新事实"}。`;
    setMsg(report.warnings.length > 0 ? `${label}已保存。${bits.join("，")}：${report.warnings[0]}` : base);
  };

  const runFactExtraction = (chapterId: string, label: string) => {
    // P1 全自动事实提取：任何路径写入正典后都要触发，档案随小说推进自动更新
    setMsg(`${label}已保存，正在提取档案事实…`);
    extractFacts(chapterId)
      .then((report) => formatExtractReport(report, label))
      .catch((e: any) =>
        setMsg(`${label}已保存；档案事实提取失败：${e}（可到图谱手动维护）。`),
      );
  };

  const handleSave = async () => {
    if (!selected) return;
    setSaving(true);
    try {
      await saveChapterContent(selected.chapter_id, content);
      // 保存成功：更新脏检查基准（P1-2 联动）
      setInitialContent(content);
      await refreshChapterMeta(selected.chapter_id);
      runFactExtraction(selected.chapter_id, `第${selected.chapter_no}章`);
    } catch (e: any) {
      setMsg(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDraft = async () => {
    if (!selected || genLoading) return;
    if (content.trim() && !confirm("章节已有内容，AI 初稿将覆盖编辑器内容（未保存的内容会丢失）。继续？")) {
      return;
    }
    setGenLoading(true);
    setMsg("");
    setGenMeta(null);
    try {
      const result = await generateWriting(selected.chapter_id, "draft", undefined, selectedTechniques);
      setContent(result.content);
      setGenMeta(result);
      setMsg(`AI 初稿已生成（${result.model}）。修改满意后点「保存」才会写入正典。`);
    } catch (e: any) {
      setMsg(`AI 初稿生成失败: ${e}`);
    } finally {
      setGenLoading(false);
    }
  };

  const handleContinue = async () => {
    if (!selected || genLoading) return;
    if (!content.trim()) {
      setMsg("请先生成初稿或输入一些正文，再使用续写。");
      return;
    }
    setGenLoading(true);
    setMsg("");
    setGenMeta(null);
    try {
      const result = await generateWriting(selected.chapter_id, "continue", content, selectedTechniques);
      setContent((prev) => `${prev.trimEnd()}\n\n${result.content}`);
      setGenMeta(result);
      setMsg(`AI 续写已追加（${result.model}）。修改满意后点「保存」才会写入正典。`);
    } catch (e: any) {
      setMsg(`AI 续写失败: ${e}`);
    } finally {
      setGenLoading(false);
    }
  };

  const handleReview = async () => {
    if (!selected || reviewing) return;
    if (!content.trim()) {
      setMsg("正文为空，无法审校。");
      return;
    }
    setReviewing(true);
    setMsg("");
    setReviewResult(null);
    try {
      const result = await reviewWriting(selected.chapter_id, content, selectedTechniques);
      setReviewResult(result);
      // P2-6：LLM 降级原因显式展示，不再让用户误以为只是"没配模型"
      if (result.mode === "full") {
        setMsg("AI 审校完成（深度模式）。以下均为建议，采纳与否由你决定。");
      } else if (result.llm_error) {
        setMsg(`AI 审校完成（本地模式）：${result.llm_error}`);
      } else {
        setMsg("AI 审校完成（本地模式：仅做了启发式检测）。");
      }
    } catch (e: any) {
      setMsg(`AI 审校失败: ${e}`);
    } finally {
      setReviewing(false);
    }
  };

  // ---- P2 批注与改写 ----

  const handleAddAnnotation = async () => {
    if (!selected) return;
    if (!annotationInput.trim()) {
      setMsg("批注内容不能为空。");
      return;
    }
    try {
      await addAnnotation(selected.chapter_id, annotationKind, annotationInput.trim());
      setAnnotationInput("");
      const detail = await getChapterContent(selected.chapter_id);
      setAnnotations(detail.annotations || []);
      setMsg("批注已添加。");
    } catch (e: any) {
      setMsg(`批注添加失败: ${e}`);
    }
  };

  const handleResolveAnnotation = async (a: ChapterAnnotation) => {
    if (!selected) return;
    const next = a.status === "已解决" ? "已指派" : "已解决";
    try {
      await updateAnnotationStatus(selected.chapter_id, a.annotation_id, next);
      setAnnotations((prev) =>
        prev.map((x) => (x.annotation_id === a.annotation_id ? { ...x, status: next } : x)),
      );
    } catch (e: any) {
      setMsg(`批注状态更新失败: ${e}`);
    }
  };

  const handleDeleteAnnotation = async (a: ChapterAnnotation) => {
    if (!selected) return;
    if (!confirm(`删除批注「${a.content.slice(0, 20)}…」？`)) return;
    try {
      await deleteAnnotation(selected.chapter_id, a.annotation_id);
      setAnnotations((prev) => prev.filter((x) => x.annotation_id !== a.annotation_id));
    } catch (e: any) {
      setMsg(`批注删除失败: ${e}`);
    }
  };

  /** 未解决批注 → 拼成改写指令 */
  const pendingAnnotationText = () =>
    annotations
      .filter((a) => a.status !== "已解决")
      .map((a) => `[${a.kind}] ${a.content}`)
      .join("\n");

  const handleRewrite = async (mode: "audit" | "de-slop") => {
    if (!selected || rewriting) return;
    if (!content.trim()) {
      setMsg("正文为空，无法改写。");
      return;
    }
    setRewriting(true);
    setMsg("");
    setRewriteResult(null);
    const instructions =
      mode === "audit"
        ? [rewriteInstructions.trim(), pendingAnnotationText()].filter(Boolean).join("\n")
        : undefined;
    try {
      const result = await aiRewrite(selected.chapter_id, content, instructions || undefined, mode);
      setRewriteResult(result);
      setMsg(
        mode === "audit"
          ? `AI 审核改写完成（${result.model}）。请检查下方 diff，确认后「应用改写」才会进入编辑器。`
          : `消痕改写完成（${result.model}）。命中词：${result.de_slop_hits.join("、") || "无"}。`,
      );
    } catch (e: any) {
      setMsg(`AI 改写失败: ${e}`);
    } finally {
      setRewriting(false);
    }
  };

  const handleApplyRewrite = () => {
    if (!rewriteResult) return;
    setContent(rewriteResult.rewritten);
    setMsg("改写稿已应用到编辑器。确认满意后点「保存」写入正典；如需同步到后续章节，点下方「级联分析」。");
    setRewriteResult(null);
  };

  // ---- P4 级联同步 ----

  const handleCascadeAnalyze = async () => {
    if (!selected || cascadeBusy) return;
    if (initialContent === content) {
      setMsg("正文相对正典没有变化（保存后即可），无需级联分析。");
      return;
    }
    setCascadeBusy(true);
    setMsg("");
    setCascadeResult(null);
    setCascadeApplied(null);
    try {
      const result = await cascadeAnalyze(selected.chapter_id, initialContent, content);
      setCascadeResult(result);
      setCascadeSelected(result.affected.map((a) => a.chapter_id));
      setMsg(result.note);
    } catch (e: any) {
      setMsg(`级联分析失败: ${e}`);
    } finally {
      setCascadeBusy(false);
    }
  };

  const handleCascadeToggle = (id: string) => {
    setCascadeSelected((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  };

  const handleCascadeApply = async () => {
    if (!selected || cascadeBusy || !cascadeResult) return;
    if (cascadeSelected.length === 0) {
      setMsg("请至少勾选一个章节。");
      return;
    }
    setCascadeBusy(true);
    setMsg("");
    try {
      const result = await cascadeApply(
        selected.chapter_id,
        cascadeSelected,
        cascadeResult.changed_facts,
      );
      setCascadeApplied(result);
      setMsg(
        result.log_warning
          ? `级联修订完成：${result.results.length} 章。${result.log_warning}`
          : `级联修订完成：${result.results.length} 章。逐章确认后保存。`,
      );
    } catch (e: any) {
      setMsg(`级联同步失败: ${e}`);
    } finally {
      setCascadeBusy(false);
    }
  };

  const handleCascadeSaveOne = async (chapterId: string, rewritten: string) => {
    try {
      await saveChapterContent(chapterId, rewritten);
      await refreshChapterMeta(chapterId);
      const target = chapters.find((c) => c.chapter_id === chapterId);
      runFactExtraction(chapterId, `第${target?.chapter_no ?? "?"}章`);
    } catch (e: any) {
      setMsg(`保存失败: ${e}`);
    }
  };

  // ---- P5 批量写作 ----

  const handleBatchNext = async () => {
    if (!selected || batchBusy) return;
    const written = new Set(
      (batchDrafts || []).filter((d) => d.chapter_id).map((d) => d.chapter_id),
    );
    // P1-3 修复：只选「已细纲且为空」的章节，绝不覆盖已有正文；
    // 没有细纲的占位章由后端再次拦截，这里提前给出明确下一步。
    const readyTargets = chapters.filter(
      (c) => (c.word_count || 0) === 0 && c.summary.trim().length > 0,
    );
    const targets = readyTargets
      .filter((c) => !written.has(c.chapter_id))
      .slice(0, 3);
    if (targets.length === 0) {
      setMsg(
        readyTargets.length === 0
          ? "没有已细纲的空章节。请先到大纲页「生成细纲 → 导入笔耕」。"
          : "没有更多空章节需要批量写作（已有内容的章节不会被覆盖）。",
      );
      return;
    }
    setBatchBusy(true);
    setMsg("");
    try {
      const result = await batchWrite(
        targets.map((c) => c.chapter_id),
        selectedTechniques.length > 0 ? selectedTechniques : undefined,
      );
      setBatchDrafts((prev) => [...(prev || []), ...result.results]);
      const pending = result.results.length;
      setMsg(
        `本批已完成 ${pending} 章草稿（${result.results[0]?.model || ""}）。第 ${Math.min(chapters.length, 3)} 章检查点：请审阅草稿，保存后再继续下一批。`,
      );
    } catch (e: any) {
      setMsg(`批量写作失败: ${e}`);
    } finally {
      setBatchBusy(false);
    }
  };

  const handleBatchSaveOne = async (item: BatchWriteItem) => {
    // P1-3 修复：保存覆盖前必须确认（与「AI 初稿」覆盖确认保持一致的防护）
    if (!confirm(`保存将覆盖第 ${item.chapter_no} 章正文，确定？`)) return;
    try {
      await saveChapterContent(item.chapter_id, item.content);
      setBatchDrafts((prev) => (prev ? prev.filter((d) => d.chapter_id !== item.chapter_id) : prev));
      await refreshChapterMeta(item.chapter_id);
      refresh();
      runFactExtraction(item.chapter_id, `第${item.chapter_no}章`);
    } catch (e: any) {
      setMsg(`保存失败: ${e}`);
    }
  };

  const handleBatchDiscard = (chapterId: string) => {
    setBatchDrafts((prev) => (prev ? prev.filter((d) => d.chapter_id !== chapterId) : prev));
  };

  const handleCascadeSaveAll = async () => {
    if (!cascadeApplied) return;
    const savedCount = cascadeApplied.results.length;
    try {
      for (const item of cascadeApplied.results) {
        await saveChapterContent(item.chapter_id, item.rewritten);
      }
      // 所有写入正典的路径统一触发事实提取，避免级联修改让档案落后于正文
      const warnings: string[] = [];
      for (const item of cascadeApplied.results) {
        try {
          await extractFacts(item.chapter_id);
        } catch (e: any) {
          warnings.push(`第${item.chapter_no}章: ${e}`);
        }
      }
      await refreshChapterMeta(selected?.chapter_id || "");
      setCascadeApplied(null);
      setCascadeResult(null);
      setMsg(
        warnings.length > 0
          ? `已保存全部级联修订；部分档案事实提取失败：${warnings[0]}`
          : `已保存全部 ${savedCount} 章节的级联修订，档案事实已同步。`,
      );
    } catch (e: any) {
      setMsg(`批量保存失败: ${e}`);
    }
  };

  const handleStyleSave = async () => {
    setStyleSaving(true);
    try {
      await updateWorldStyle(style);
      setMsg("风格笔记已保存到正典（会影响后续 AI 生成）。");
    } catch (e: any) {
      setMsg(`风格笔记保存失败: ${e}`);
    } finally {
      setStyleSaving(false);
    }
  };

  const narrativeTechniques = techniques.filter((t) => t.category === "叙事技巧");
  const rhythmTechniques = techniques.filter((t) => t.category !== "叙事技巧");

  return (
    <div className="view-card">
      <h2>笔耕</h2>
      {msg && <p className="msg">{msg}</p>}

      <div className="writing-layout">
        <div className="chapter-sidebar">
          <h3>章节</h3>
          {chapters.length > 0 ? (
            <ul className="chapter-nav">
              {chapters.map((c) => (
                <li
                  key={c.chapter_id}
                  className={`chapter-nav-item ${selected?.chapter_id === c.chapter_id ? "active" : ""}`}
                  onClick={() => handleSelect(c)}
                >
                  <span className="chapter-nav-no">第{c.chapter_no}章</span>
                  <span className="chapter-nav-title">{c.title || "未命名"}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="empty">暂无章节，请先在大纲中创建。</p>
          )}
        </div>

        <div className="writing-area">
          {selected ? (
            <div>
              <div className="writing-header">
                <h3>第{selected.chapter_no}章 {selected.title}</h3>
                <div className="btn-group">
                  <button className="btn-sm" onClick={handleDraft} disabled={genLoading}>
                    {genLoading ? "生成中…" : "AI 生成初稿"}
                  </button>
                  <button className="btn-sm" onClick={handleContinue} disabled={genLoading}>
                    AI 续写
                  </button>
                  <button className="btn-sm" onClick={handleReview} disabled={reviewing || genLoading}>
                    {reviewing ? "审校中…" : "AI 审校"}
                  </button>
                  <button className="btn-primary" onClick={handleSave} disabled={saving || genLoading}>
                    {saving ? "保存中..." : "保存"}
                  </button>
                </div>
              </div>
              <div className="writing-meta">
                <span>状态: {label(chapterStatusLabels, selected.status)}</span>
                <span>字数: {content.length}</span>
                <span>版本: v{selected.version}</span>
                <span>修订: {revisionCount} 次</span>
                <span>一致性: {selected.consistency_score}</span>
              </div>

              {/* 技巧选择（F12/F15，软约束） */}
              {(narrativeTechniques.length > 0 || rhythmTechniques.length > 0) && (
                <div className="section technique-picker">
                  <h4>本章写作技巧（可选，建议制）</h4>
                  {narrativeTechniques.length > 0 && (
                    <div className="tech-group">
                      <span className="tech-cat">叙事技巧</span>
                      {narrativeTechniques.map((t) => (
                        <button
                          key={t.id}
                          className={`tech-chip ${selectedTechniques.includes(t.id) ? "active" : ""}`}
                          title={t.description}
                          onClick={() => toggleTechnique(t.id)}
                        >
                          {t.name}
                        </button>
                      ))}
                    </div>
                  )}
                  {rhythmTechniques.length > 0 && (
                    <div className="tech-group">
                      <span className="tech-cat">网文节奏</span>
                      {rhythmTechniques.map((t) => (
                        <button
                          key={t.id}
                          className={`tech-chip ${selectedTechniques.includes(t.id) ? "active" : ""}`}
                          title={t.description}
                          onClick={() => toggleTechnique(t.id)}
                        >
                          {t.name}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* P5 批量写作（细纲导入后使用，每 3 章一个检查点） */}
              <div className="section batch-panel">
                <h4>批量写作（每批 3 章，检查点制）</h4>
                <p className="llm-hint">
                  按章节顺序串行生成初稿（建议制，不落盘）。每批 3 章——生成后请审阅草稿并保存，
                  确认无误再点「写下一批」。技巧 chips 中的勾选会应用到本批。
                </p>
                <div className="btn-group">
                  <button className="btn-sm" onClick={handleBatchNext} disabled={batchBusy || chapters.length === 0}>
                    {batchBusy ? "写作中…" : "写下一批（3 章）"}
                  </button>
                </div>
                {batchDrafts && batchDrafts.length > 0 && (
                  <div className="batch-drafts">
                    {batchDrafts.map((item) => (
                      <div key={item.chapter_id} className="batch-draft-item">
                        <p>
                          <b>第{item.chapter_no}章 {item.title}</b>
                          <span className="llm-hint"> · {item.model}</span>
                          {item.anti_slop_warnings.length > 0 && (
                            <span className="gen-meta-warn"> ⚠ AI 味: {item.anti_slop_warnings.join("、")}</span>
                          )}
                        </p>
                        <pre className="batch-draft-content">{item.content.slice(0, 400)}{item.content.length > 400 ? "…" : ""}</pre>
                        <div className="btn-group">
                          <button className="btn-sm" onClick={() => handleBatchSaveOne(item)}>保存此章</button>
                          <button className="btn-sm" onClick={() => handleBatchDiscard(item.chapter_id)}>丢弃</button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* P2 批注 + AI 审核改写 */}
              <div className="section annotation-panel">
                <h4>批注与 AI 改写</h4>
                <div className="annotation-add">
                  <select
                    className="ps-input annotation-kind"
                    value={annotationKind}
                    onChange={(e) => setAnnotationKind(e.target.value)}
                  >
                    {["建议", "批评", "疑问", "指令"].map((k) => (
                      <option key={k} value={k}>{k}</option>
                    ))}
                  </select>
                  <input
                    className="ps-input annotation-input"
                    value={annotationInput}
                    onChange={(e) => setAnnotationInput(e.target.value)}
                    placeholder="添加批注，例如：这段太啰嗦，删一半…"
                  />
                  <button className="btn-sm" onClick={handleAddAnnotation}>添加批注</button>
                </div>
                {annotations.length > 0 && (
                  <ul className="annotation-list">
                    {annotations.map((a) => (
                      <li key={a.annotation_id} className={`annotation-item ${a.status === "已解决" ? "done" : ""}`}>
                        <span className="annotation-kind-tag">{a.kind}</span>
                        <span className="annotation-text">{a.content}</span>
                        <button className="btn-mini" onClick={() => handleResolveAnnotation(a)}>
                          {a.status === "已解决" ? "撤销解决" : "标记解决"}
                        </button>
                        <button className="btn-mini danger" onClick={() => handleDeleteAnnotation(a)}>删</button>
                      </li>
                    ))}
                  </ul>
                )}
                <div className="rewrite-bar">
                  <input
                    className="ps-input rewrite-instructions"
                    value={rewriteInstructions}
                    onChange={(e) => setRewriteInstructions(e.target.value)}
                    placeholder="追加改写指令（可选，未解决批注会自动带上）…"
                  />
                  <button className="btn-sm" onClick={() => handleRewrite("audit")} disabled={rewriting}>
                    {rewriting ? "改写中…" : "AI 审核并改写"}
                  </button>
                  <button className="btn-sm" onClick={() => handleRewrite("de-slop")} disabled={rewriting}>
                    消痕改写
                  </button>
                </div>
                {rewriteResult && (
                  <div className="rewrite-result">
                    <p className="gen-meta-item">
                      模式: {rewriteResult.mode === "de-slop" ? "消痕改写" : "审核改写"} · 模型: {rewriteResult.model}
                    </p>
                    {rewriteResult.changes.length > 0 && (
                      <ul className="review-list">
                        {rewriteResult.changes.map((c, i) => (
                          <li key={i}><b>{c.what}</b> — {c.why}</li>
                        ))}
                      </ul>
                    )}
                    <div className="diff-view">
                      {rewriteResult.diff.map((d, i) => (
                        <p key={i} className={`diff-line diff-${d.kind}`}>{d.text}</p>
                      ))}
                    </div>
                    <div className="btn-group">
                      <button className="btn-primary" onClick={handleApplyRewrite}>应用改写</button>
                      <button className="btn-sm" onClick={() => setRewriteResult(null)}>放弃</button>
                    </div>
                  </div>
                )}
              </div>

                {/* P4 级联同步（仅向后，受控传播） */}
                <div className="section cascade-panel">
                  <h4>级联同步（改写 → 后续章节）</h4>
                  <div className="btn-group">
                    <button className="btn-sm" onClick={handleCascadeAnalyze} disabled={cascadeBusy}>
                      {cascadeBusy ? "分析中…" : "级联分析"}
                    </button>
                  </div>
                  <p className="llm-hint">
                    先「应用改写」（或直接编辑正文），再点「级联分析」。系统对比正典原稿与当前内容，
                    提取事实变更，找出受影响的后续章节（仅向后，上限 {cascadeResult?.limit ?? 20} 章）。
                  </p>
                  {cascadeResult && cascadeResult.changed_facts.length > 0 && (
                    <p className="gen-meta-item">
                      变更事实：
                      {cascadeResult.changed_facts.map((f) => `${f.entity}「${f.attribute}」：${f.old_value} → ${f.new_value}`).join("；")}
                    </p>
                  )}
                  {cascadeResult && cascadeResult.affected.length > 0 && (
                    <ul className="cascade-list">
                      {cascadeResult.affected.map((a) => (
                        <li key={a.chapter_id} className="cascade-item">
                          <label>
                            <input
                              type="checkbox"
                              checked={cascadeSelected.includes(a.chapter_id)}
                              onChange={() => handleCascadeToggle(a.chapter_id)}
                            />
                            <b>第{a.chapter_no}章 {a.title}</b>
                            <span className="cascade-match">涉及：{a.matched_entities.join("、")}</span>
                            <span className="cascade-snippet">{a.snippet}</span>
                          </label>
                        </li>
                      ))}
                    </ul>
                  )}
                  {cascadeResult && cascadeResult.affected.length > 0 && (
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleCascadeApply} disabled={cascadeBusy || cascadeSelected.length === 0}>
                        {cascadeBusy ? "同步中…" : `同步到所选 ${cascadeSelected.length} 章`}
                      </button>
                    </div>
                  )}
                  {cascadeApplied && cascadeApplied.results.length > 0 && (
                    <div className="cascade-results">
                      {cascadeApplied.results.map((item) => (
                        <div key={item.chapter_id} className="cascade-result-item">
                          <p><b>第{item.chapter_no}章 {item.title}</b></p>
                          <pre className="cascade-draft">{item.rewritten}</pre>
                          <button className="btn-sm" onClick={() => handleCascadeSaveOne(item.chapter_id, item.rewritten)}>
                            保存此章
                          </button>
                        </div>
                      ))}
                      <button className="btn-primary" onClick={handleCascadeSaveAll}>
                        全部保存
                      </button>
                    </div>
                  )}
                </div>

              {/* 生成元信息（记忆检索/约束/技巧/消痕） */}
              {genMeta && (
                <div className="section gen-meta">
                  <h4>生成元信息</h4>
                  <div className="gen-meta-grid">
                    {genMeta.memory_stats && (
                      <span className="gen-meta-item">
                        记忆检索: {genMeta.memory_stats.entity_count} 个相关实体 · {genMeta.memory_stats.total_tokens}/{genMeta.memory_stats.budget_total} tokens
                      </span>
                    )}
                    {genMeta.constraints_applied && genMeta.constraints_applied.length > 0 && (
                      <span className="gen-meta-item">
                        硬约束注入: {genMeta.constraints_applied.length} 条
                      </span>
                    )}
                    {genMeta.techniques_applied && genMeta.techniques_applied.length > 0 && (
                      <span className="gen-meta-item">
                        技巧注入: {genMeta.techniques_applied.length} 项
                      </span>
                    )}
                  </div>
                  {genMeta.anti_slop_warnings && genMeta.anti_slop_warnings.length > 0 ? (
                    <p className="gen-meta-warn">
                      ⚠ 检测到高频 AI 味表达: {genMeta.anti_slop_warnings.join("、")}。可酌情调整（建议制，不强制）。
                    </p>
                  ) : genMeta.anti_slop_warnings ? (
                    <p className="gen-meta-ok">✓ 未检测到高频 AI 味表达。</p>
                  ) : null}
                </div>
              )}

              {/* AI 审校报告（建议制） */}
              {reviewResult && (
                <div className="section review-report">
                  <h4>AI 审校报告（{reviewResult.mode === "full" ? "深度模式" : "本地模式"}）</h4>
                  <div className="gen-meta-grid">
                    <span className="gen-meta-item">字数: {reviewResult.local.char_count}</span>
                    <span className="gen-meta-item">说教密度: {reviewResult.local.tell_density.toFixed(1)}/千字</span>
                    {reviewResult.techniques_checked.length > 0 && (
                      <span className="gen-meta-item">技巧检查: {reviewResult.techniques_checked.length} 项</span>
                    )}
                  </div>
                  {reviewResult.local.meta_narration_hits.length > 0 && (
                    <p className="gen-meta-warn">
                      ⚠ 元叙述（AI 自指）: {reviewResult.local.meta_narration_hits.join("、")}
                    </p>
                  )}
                  {reviewResult.local.anti_slop_hits.length > 0 && (
                    <p className="gen-meta-warn">
                      ⚠ 高频 AI 味表达: {reviewResult.local.anti_slop_hits.join("、")}
                    </p>
                  )}
                  {reviewResult.local.tell_counts.length > 0 && (
                    <p className="gen-meta-item">
                      说教式表达: {reviewResult.local.tell_counts.map((t) => `${t.word}×${t.count}`).join("、")}（建议改为行为展示）
                    </p>
                  )}
                  {reviewResult.llm && (
                    <>
                      {reviewResult.llm.hard_constraint_issues.length > 0 && (
                        <p className="gen-meta-warn">
                          ⛔ 硬约束问题: {reviewResult.llm.hard_constraint_issues.join("；")}
                        </p>
                      )}
                      {reviewResult.llm.entity_conflicts.length > 0 && (
                        <p className="gen-meta-warn">
                          ⚠ 实体冲突: {reviewResult.llm.entity_conflicts.join("；")}
                        </p>
                      )}
                      {reviewResult.llm.failure_modes.length > 0 && (
                        <ul className="review-list">
                          {reviewResult.llm.failure_modes.map((f, i) => (
                            <li key={i}>
                              [{f.severity}] {f.dimension}: {f.detail}
                            </li>
                          ))}
                        </ul>
                      )}
                      {reviewResult.llm.suggestions.length > 0 && (
                        <p className="gen-meta-item">
                          建议: {reviewResult.llm.suggestions.join("；")}
                        </p>
                      )}
                    </>
                  )}
                </div>
              )}

              {selected.summary && (
                <div className="section">
                  <h4>摘要</h4>
                  <p>{selected.summary}</p>
                </div>
              )}
              <div className="section">
                <h4>正文</h4>
                <textarea
                  className="ps-input ps-textarea editor-textarea"
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  placeholder="在此输入正文内容..."
                />
              </div>

              {/* 写作风格笔记（F13） */}
              <details className="section style-panel">
                <summary>写作风格笔记（注入 AI 生成，正典 AestheticLayer）</summary>
                <div className="style-field">
                  <label>风格笔记（文风/语气/修辞偏好）</label>
                  <textarea
                    className="ps-input ps-textarea"
                    value={style.style_notes}
                    onChange={(e) => setStyle({ ...style, style_notes: e.target.value })}
                    placeholder="如：冷峻克制，短句推进，克制抒情，动作描写优先……"
                  />
                </div>
                <div className="style-field">
                  <label>节奏笔记（叙事节奏/情绪曲线偏好）</label>
                  <textarea
                    className="ps-input ps-textarea"
                    value={style.pacing_notes}
                    onChange={(e) => setStyle({ ...style, pacing_notes: e.target.value })}
                    placeholder="如：先抑后扬，每 3-5 章一个小高潮，章末断在悬念处……"
                  />
                </div>
                <button className="btn-sm" onClick={handleStyleSave} disabled={styleSaving}>
                  {styleSaving ? "保存中..." : "保存风格笔记"}
                </button>
              </details>
            </div>
          ) : (
            <p className="empty">选择一个章节开始编辑。</p>
          )}
        </div>
      </div>
    </div>
  );
}
