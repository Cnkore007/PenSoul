import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Play,
  Pause,
  Square,
  Loader2,
  CheckCircle2,
  XCircle,
  Circle,
  PenLine,
  ShieldCheck,
  Database,
  AlertTriangle,
} from "lucide-react";
import type { ProjectData, ViewType, Chapter, PipelineEvent } from "../types";
import {
  listModels,
  runChapterPipeline,
  pausePipeline,
  resumePipeline,
  stopPipeline,
  getPipelineState,
} from "../ipc";

interface HarnessConsoleProps {
  projectData: ProjectData;
  onNavigate?: (view: ViewType) => void;
}

// 三阶段展示元数据
const STAGE_META: Record<string, { label: string; color: string }> = {
  chapter_writing: { label: "写作", color: "#7c6cf0" },
  chapter_review: { label: "审查", color: "#d99a3d" },
  state_injection: { label: "回灌", color: "#3f9e63" },
};

export function HarnessConsole({ projectData, onNavigate }: HarnessConsoleProps) {
  const [models, setModels] = useState<any[]>([]);
  const [writingModel, setWritingModel] = useState("");
  const [reviewModel, setReviewModel] = useState("");
  const [running, setRunning] = useState(false);
  const [paused, setPaused] = useState(false);
  const [events, setEvents] = useState<PipelineEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [activeChapterId, setActiveChapterId] = useState<string | null>(null);
  const [doneIds, setDoneIds] = useState<string[]>([]);
  const [failedIds, setFailedIds] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  // 快照加载前到达的实时事件先暂存，避免「快照→订阅」间隙丢事件
  const snapshotLoadedRef = useRef(false);
  const preSnapshotRef = useRef<PipelineEvent[]>([]);

  // 待写章节队列：有梗概的章节（后端只写正文为空的）
  const queue: Chapter[] = projectData.volumes
    .flatMap((v) => v.chapters)
    .filter((c) => (c.summary ?? "").trim().length > 0)
    .sort((a, b) => (a.chapter_no ?? 0) - (b.chapter_no ?? 0));

  // 统一的事件处理：实时事件与快照重放走同一条路径，保证状态一致
  const applyEvent = useCallback((e: PipelineEvent) => {
    setEvents((prev) => [...prev, e]);
    if (e.kind === "chapter_start") setActiveChapterId(e.chapter_id);
    if (e.kind === "chapter_done") {
      setDoneIds((prev) => (prev.includes(e.chapter_id) ? prev : [...prev, e.chapter_id]));
      setActiveChapterId((prev) => (prev === e.chapter_id ? null : prev));
    }
    if (e.kind === "chapter_failed") {
      setFailedIds((prev) => (prev.includes(e.chapter_id) ? prev : [...prev, e.chapter_id]));
      setActiveChapterId((prev) => (prev === e.chapter_id ? null : prev));
    }
    if (e.kind === "pipeline_done") {
      setRunning(false);
      setPaused(false);
      setActiveChapterId(null);
    }
  }, []);

  // 初始化：模型列表 + 快照恢复（运行状态/事件流/模型选择）+ 实时事件订阅
  useEffect(() => {
    listModels()
      .then((ms) => setModels(ms.filter((m: any) => m.is_available !== false)))
      .catch(() => {});

    let unlisten: (() => void) | null = null;
    listen<PipelineEvent>("harness-event", (evt) => {
      const e = evt.payload;
      if (!snapshotLoadedRef.current) {
        preSnapshotRef.current.push(e);
        return;
      }
      applyEvent(e);
    }).then((u) => {
      unlisten = u;
    });

    getPipelineState()
      .then((s) => {
        setRunning(s.running);
        setPaused(s.paused);
        // 恢复上次运行使用的模型选择（页面切换后下拉框不再跳回默认）
        if (s.writing_model) setWritingModel(s.writing_model);
        if (s.review_model) setReviewModel(s.review_model);
        // 重放后端事件缓冲 + 间隙期暂存的事件（按 seq 去重）
        const buffered = s.events ?? [];
        const maxSeq = buffered.reduce((m, e) => Math.max(m, e.seq ?? 0), 0);
        const extras = preSnapshotRef.current.filter((e) => (e.seq ?? 0) > maxSeq);
        [...buffered, ...extras].forEach(applyEvent);
        snapshotLoadedRef.current = true;
      })
      .catch(() => {
        preSnapshotRef.current.forEach(applyEvent);
        snapshotLoadedRef.current = true;
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, [applyEvent]);

  // 事件流自动滚到底部
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [events]);

  const handleStart = useCallback(async () => {
    setError(null);
    setEvents([]);
    setDoneIds([]);
    setFailedIds([]);
    setRunning(true);
    setPaused(false);
    try {
      await runChapterPipeline(null, writingModel || null, reviewModel || null);
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || String(e));
    } finally {
      setRunning(false);
      setPaused(false);
      setActiveChapterId(null);
    }
  }, [writingModel, reviewModel]);

  const handlePauseResume = useCallback(async () => {
    try {
      if (paused) {
        await resumePipeline();
        setPaused(false);
      } else {
        await pausePipeline();
        setPaused(true);
      }
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || String(e));
    }
  }, [paused]);

  const handleStop = useCallback(async () => {
    try {
      await stopPipeline();
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || String(e));
    }
  }, []);

  const pendingCount = queue.filter((c) => c.word_count === 0).length;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>造化工坊</h2>
        <p style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", margin: 0 }}>
          选择工作流后自动连写：每章走「写作 → 审查 → 回灌」闭环，正文落库后自动出现在笔耕
        </p>
      </div>

      {/* 控制面板 */}
      <div className="card" style={{ marginBottom: 16 }}>
        <div
          style={{
            display: "flex",
            gap: 16,
            alignItems: "flex-end",
            flexWrap: "wrap",
          }}
        >
          <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 13 }}>
            <span style={{ opacity: 0.7 }}>写作模型</span>
            <select
              value={writingModel}
              disabled={running}
              onChange={(e) => setWritingModel(e.target.value)}
              style={{ minWidth: 180, padding: "6px 8px" }}
            >
              <option value="">自动（第一个可用）</option>
              {models.map((m: any) => (
                <option key={m.model_id} value={m.model_id}>
                  {m.model_id}
                </option>
              ))}
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 13 }}>
            <span style={{ opacity: 0.7 }}>审查模型（尽量与写作不同）</span>
            <select
              value={reviewModel}
              disabled={running}
              onChange={(e) => setReviewModel(e.target.value)}
              style={{ minWidth: 180, padding: "6px 8px" }}
            >
              <option value="">自动（异模型优先）</option>
              {models.map((m: any) => (
                <option key={m.model_id} value={m.model_id}>
                  {m.model_id}
                </option>
              ))}
            </select>
          </label>

          <div style={{ display: "flex", gap: 8, marginLeft: "auto" }}>
            {!running ? (
              <button className="btn btn-primary" onClick={handleStart} disabled={pendingCount === 0}>
                <Play size={14} /> 开始写作（{pendingCount} 章待写）
              </button>
            ) : (
              <>
                <button className="btn btn-secondary" onClick={handlePauseResume}>
                  {paused ? (
                    <>
                      <Play size={14} /> 继续
                    </>
                  ) : (
                    <>
                      <Pause size={14} /> 暂停
                    </>
                  )}
                </button>
                <button
                  className="btn btn-secondary"
                  style={{ borderColor: "#c0392b", color: "#c0392b" }}
                  onClick={handleStop}
                >
                  <Square size={14} /> 停止
                </button>
              </>
            )}
          </div>
        </div>
        {error && (
          <div style={{ marginTop: 12, color: "#c0392b", fontSize: 13, display: "flex", gap: 6 }}>
            <AlertTriangle size={14} style={{ flexShrink: 0, marginTop: 2 }} />
            <span>{error}</span>
          </div>
        )}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "280px 1fr", gap: 16, alignItems: "start" }}>
        {/* 章节队列 */}
        <div className="card">
          <div className="card-header" style={{ fontSize: 14, fontWeight: 600 }}>
            章节队列（{queue.length}）
          </div>
          {queue.length === 0 ? (
            <div className="empty-state" style={{ padding: 24 }}>
              <div className="empty-state-text">还没有待写的章节</div>
              <div className="empty-state-sub">先去「大纲」展开情节脉络的细纲（或为章节填写梗概），再回来开始连写</div>
              <button
                className="btn btn-secondary"
                style={{ marginTop: 12 }}
                onClick={() => onNavigate?.("outline")}
              >
                前往大纲
              </button>
            </div>
          ) : (
            <div style={{ maxHeight: 520, overflowY: "auto" }}>
              {queue.map((ch) => {
                const isActive = ch.chapter_id === activeChapterId;
                const isDone = doneIds.includes(ch.chapter_id);
                const isFailed = failedIds.includes(ch.chapter_id);
                const hasContent = ch.word_count > 0;
                return (
                  <div
                    key={ch.chapter_id}
                    style={{
                      display: "flex",
                      gap: 8,
                      alignItems: "center",
                      padding: "8px 10px",
                      borderRadius: 6,
                      background: isActive ? "rgba(124,108,240,0.12)" : "transparent",
                      fontSize: 13,
                    }}
                  >
                    {isActive ? (
                      <Loader2 size={14} className="spinning" style={{ color: "#7c6cf0" }} />
                    ) : isDone || hasContent ? (
                      <CheckCircle2 size={14} style={{ color: "#3f9e63" }} />
                    ) : isFailed ? (
                      <XCircle size={14} style={{ color: "#c0392b" }} />
                    ) : (
                      <Circle size={14} style={{ opacity: 0.4 }} />
                    )}
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontWeight: isActive ? 600 : 400 }}>
                        {ch.chapter_no ? `第${ch.chapter_no}章 ` : ""}
                        {ch.title || "未命名"}
                      </div>
                      <div style={{ fontSize: 11, opacity: 0.55 }}>
                        {hasContent ? `${ch.word_count} 字已写` : "待写"}
                        {isFailed ? " · 审查熔断" : ""}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* 事件流 */}
        <div className="card">
          <div className="card-header" style={{ fontSize: 14, fontWeight: 600 }}>
            写作实况
            {running && (
              <span style={{ marginLeft: 8, fontSize: 12, color: "#7c6cf0" }}>
                <Loader2 size={12} className="spinning" style={{ verticalAlign: -2 }} />{" "}
                {paused ? "已暂停" : "运行中"}
              </span>
            )}
          </div>
          <div
            ref={scrollRef}
            style={{ maxHeight: 520, overflowY: "auto", padding: "4px 8px", fontSize: 13 }}
          >
            {events.length === 0 ? (
              <div className="empty-state" style={{ padding: 32 }}>
                <div className="empty-state-text">尚无写作记录</div>
                <div className="empty-state-sub">
                  点击「开始写作」后，这里会实时显示每一章的写作、审查与回灌过程
                </div>
              </div>
            ) : (
              events.map((ev, i) => <EventRow key={i} ev={ev} />)
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── 单条事件渲染 ──

function EventRow({ ev }: { ev: PipelineEvent }) {
  const meta = STAGE_META[ev.stage];
  const stageBadge = meta && (
    <span
      style={{
        display: "inline-block",
        padding: "1px 8px",
        borderRadius: 10,
        fontSize: 11,
        fontWeight: 600,
        color: "#fff",
        background: meta.color,
        marginRight: 6,
      }}
    >
      {meta.label}
    </span>
  );

  switch (ev.kind) {
    case "chapter_start":
      return (
        <div
          style={{
            margin: "14px 0 6px",
            paddingTop: 10,
            borderTop: "1px solid rgba(128,128,128,0.25)",
            fontWeight: 600,
          }}
        >
          <PenLine size={13} style={{ verticalAlign: -2, marginRight: 4 }} />
          {ev.content}
        </div>
      );

    case "stage_start":
      return (
        <div style={{ padding: "3px 0", opacity: 0.85 }}>
          {stageBadge}
          <span style={{ fontSize: 12, opacity: 0.7 }}>{ev.content}</span>
        </div>
      );

    case "llm_output":
      return (
        <div style={{ padding: "3px 0 3px 4px" }}>
          {stageBadge}
          <div
            style={{
              marginTop: 4,
              padding: "8px 10px",
              borderRadius: 6,
              background: "rgba(128,128,128,0.08)",
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              fontSize: 12,
              lineHeight: 1.6,
              maxHeight: 180,
              overflowY: "auto",
            }}
          >
            {ev.content}
          </div>
        </div>
      );

    case "review_report": {
      const passed = (ev.score ?? 0) >= 80;
      return (
        <div style={{ padding: "3px 0 3px 4px" }}>
          {stageBadge}
          <span
            style={{
              display: "inline-block",
              padding: "1px 8px",
              borderRadius: 10,
              fontSize: 11,
              fontWeight: 700,
              color: "#fff",
              background: passed ? "#3f9e63" : "#d99a3d",
            }}
          >
            {ev.score != null ? `${ev.score} 分` : "审查"}
          </span>
          <div
            style={{
              marginTop: 4,
              padding: "8px 10px",
              borderRadius: 6,
              border: `1px solid ${passed ? "rgba(63,158,99,0.4)" : "rgba(217,154,61,0.5)"}`,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              fontSize: 12,
              lineHeight: 1.6,
            }}
          >
            <ShieldCheck size={12} style={{ verticalAlign: -2, marginRight: 4 }} />
            {ev.content}
          </div>
        </div>
      );
    }

    case "gate":
      return (
        <div style={{ padding: "2px 0 2px 4px", fontSize: 12, opacity: 0.75 }}>
          <ShieldCheck size={11} style={{ verticalAlign: -1, marginRight: 4 }} />
          门控：{ev.content}
        </div>
      );

    case "effect":
      return (
        <div style={{ padding: "2px 0 2px 4px", fontSize: 12, color: "#3f9e63" }}>
          <Database size={11} style={{ verticalAlign: -1, marginRight: 4 }} />
          {ev.content}
        </div>
      );

    case "chapter_done":
      return (
        <div style={{ padding: "6px 0", color: "#3f9e63", fontWeight: 600 }}>
          <CheckCircle2 size={13} style={{ verticalAlign: -2, marginRight: 4 }} />
          {ev.content}
        </div>
      );

    case "chapter_failed":
      return (
        <div style={{ padding: "6px 0", color: "#c0392b", fontWeight: 600 }}>
          <XCircle size={13} style={{ verticalAlign: -2, marginRight: 4 }} />
          {ev.content}
        </div>
      );

    case "paused":
    case "resumed":
      return (
        <div style={{ padding: "4px 0", fontSize: 12, color: "#d99a3d" }}>
          {ev.kind === "paused" ? "⏸" : "▶"} {ev.content}
        </div>
      );

    case "pipeline_done":
      return (
        <div
          style={{
            marginTop: 12,
            padding: "10px 12px",
            borderRadius: 6,
            background: ev.status === "stopped" ? "rgba(217,154,61,0.12)" : "rgba(63,158,99,0.12)",
            fontWeight: 600,
          }}
        >
          {ev.content}
        </div>
      );

    default:
      return (
        <div style={{ padding: "2px 0", fontSize: 12, opacity: 0.7 }}>
          {ev.content}
        </div>
      );
  }
}
