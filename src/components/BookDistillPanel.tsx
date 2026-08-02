import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { BookOpen, Loader2, CheckCircle2, XCircle, Sparkles, FileUp, X } from "lucide-react";
import type { BookPackage, LlmModel } from "../types";
import { distillBook, getDistillState } from "../ipc";

// 五个蒸馏维度（与后端 DIMENSIONS 一致）
export const DIMENSION_OPTIONS = [
  { slug: "style", label: "文风 DNA", hint: "句式 / 词汇 / 节奏 / 视角声音" },
  { slug: "structure", label: "叙事结构", hint: "篇章布局 / 信息揭示 / 伏笔回收" },
  { slug: "character", label: "人物塑造", hint: "登场 / 动机 / 弧线 / 群像" },
  { slug: "tension", label: "冲突与张力", hint: "悬念引擎 / 张力曲线 / 章末钩子" },
  { slug: "genre", label: "类型范式", hint: "题材惯例 / 期待管理 / 突破点" },
];

interface PhaseEvent {
  phase: string;
  status: string;
  message: string;
  detail: string;
}

interface BookDistillPanelProps {
  models: LlmModel[];
  // pkg 为 null 表示任务在后台完成（页面切换后重连场景），面板只需刷新列表
  onDistilled: (pkg: BookPackage | null) => void;
  onClose: () => void;
}

// 书籍蒸馏面板：书名/作者/样章/维度勾选/模型选择 + 分阶段进度（book-distill-phase 事件）
export function BookDistillPanel({ models, onDistilled, onClose }: BookDistillPanelProps) {
  const [title, setTitle] = useState("");
  const [author, setAuthor] = useState("");
  const [filePath, setFilePath] = useState<string | null>(null);
  const [sampleText, setSampleText] = useState("");
  const [showSample, setShowSample] = useState(false);
  const [dims, setDims] = useState<string[]>(DIMENSION_OPTIONS.map((d) => d.slug));
  const [model, setModel] = useState("");
  const [running, setRunning] = useState(false);
  const [phases, setPhases] = useState<PhaseEvent[]>([]);
  const unlistenRef = useRef<(() => void) | null>(null);
  // 保存最新回调，重连订阅期间避免闭包捕获旧版本
  const onDistilledRef = useRef(onDistilled);
  onDistilledRef.current = onDistilled;

  // 合并/更新阶段事件（按阶段名去重，保留最新状态）
  const upsertPhase = (prev: PhaseEvent[], ev: PhaseEvent) => {
    const i = prev.findIndex((p) => p.phase === ev.phase);
    if (i >= 0) {
      const u = [...prev];
      u[i] = ev;
      return u;
    }
    return [...prev, ev];
  };

  // 页面切换后重连：若书籍蒸馏仍在后台进行，重放缓冲事件并订阅实时进度与终态
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const st = await getDistillState().catch(() => null);
      if (!st || cancelled || !st.running || st.kind !== "book") return;
      setRunning(true);
      (st.events ?? []).forEach(ev => {
        if (ev.phase === "__distill__") return;
        setPhases(prev => upsertPhase(prev, ev));
      });
      const unlisten = await listen<PhaseEvent>("book-distill-phase", (evt) => {
        const e = evt.payload;
        if (e.phase === "__distill__") {
          if (e.status === "finished") {
            setRunning(false);
            onDistilledRef.current(null);
          } else {
            setPhases(prev => [...prev, e]);
            setRunning(false);
          }
          return;
        }
        setPhases(prev => upsertPhase(prev, e));
      });
      if (cancelled) { unlisten(); return; }
      // 订阅间隙任务可能已结束：复查一次，避免错过终态
      const st2 = await getDistillState().catch(() => null);
      if (cancelled) { unlisten(); return; }
      if (st2 && !st2.running) {
        setRunning(false);
        onDistilledRef.current(null);
        unlisten();
      }
    })();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    return () => {
      if (unlistenRef.current) unlistenRef.current();
    };
  }, []);

  function toggleDim(slug: string) {
    setDims((prev) =>
      prev.includes(slug) ? prev.filter((d) => d !== slug) : [...prev, slug]
    );
  }

  // 从路径取文件名展示（兼容 Windows / Unix 分隔符）
  const fileName = filePath ? (filePath.split(/[\\/]/).pop() ?? filePath) : null;

  async function pickFile() {
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "书籍", extensions: ["txt", "md", "markdown", "epub", "pdf"] }],
    });
    if (typeof selected === "string") setFilePath(selected);
  }

  async function startDistill() {
    const t = title.trim();
    // 上传文件时书名可留空（后端自动取文件名）
    if ((!t && !filePath) || dims.length === 0) return;
    setRunning(true);
    setPhases([]);

    unlistenRef.current = await listen<PhaseEvent>("book-distill-phase", (evt) => {
      setPhases(prev => upsertPhase(prev, evt.payload));
    });

    try {
      const pkg = await distillBook(
        t,
        author.trim() || null,
        filePath,
        sampleText.trim() || null,
        dims,
        model || null
      );
      onDistilled(pkg);
      setTitle("");
      setAuthor("");
      setFilePath(null);
      setSampleText("");
    } catch (e: any) {
      setPhases((prev) => [
        ...prev,
        { phase: "错误", status: "error", message: `蒸馏失败: ${typeof e === "string" ? e : e?.message || String(e)}`, detail: "" },
      ]);
    } finally {
      setRunning(false);
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  }

  const availableModels = models.filter((m) => m.is_available !== false);

  return (
    <div
      style={{
        background: "var(--color-paper)",
        border: "1px solid var(--color-accent)",
        borderRadius: "var(--radius-md)",
        padding: "var(--space-lg) var(--space-xl)",
        marginBottom: "var(--space-xl)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)" }}>
        <BookOpen size={18} style={{ color: "var(--color-accent)" }} />
        <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
          蒸馏一本书
        </span>
        <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          方法论：skills/pensoul-skill-Books
        </span>
      </div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginBottom: "var(--space-md)" }}>
        提炼这本书的「写法」为写作技能卡，保存到 WritingCard 文件夹，可在下方绑定到工作流环节
      </div>

      {/* 上传书籍文件（优先来源） */}
      <div style={{ display: "flex", gap: "var(--space-sm)", alignItems: "center", flexWrap: "wrap", marginBottom: "var(--space-sm)" }}>
        <button
          className="btn btn-secondary"
          style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}
          onClick={pickFile}
          disabled={running}
        >
          <FileUp size={14} /> 上传书籍文件（txt / md / epub / pdf）
        </button>
        {fileName && (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              fontSize: "var(--text-xs)",
              color: "var(--color-accent)",
              border: "1px solid var(--color-accent)",
              borderRadius: "var(--radius-sm)",
              padding: "3px 10px",
            }}
          >
            {fileName}
            {!running && <X size={12} style={{ cursor: "pointer" }} onClick={() => setFilePath(null)} />}
          </span>
        )}
        <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          上传后按全书抽样蒸馏（开头+中段+结尾约 2 万字），比纯知识模式更准
        </span>
      </div>

      {/* 书名 / 作者 / 模型 */}
      <div style={{ display: "flex", gap: "var(--space-sm)", alignItems: "center", flexWrap: "wrap", marginBottom: "var(--space-sm)" }}>
        <input
          className="pm-input"
          style={{ marginBottom: 0, flex: 2, minWidth: 160 }}
          placeholder={filePath ? "书名（可选，默认取文件名）" : "书名（必填），如：雪崩、三体、故事"}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={running}
        />
        <input
          className="pm-input"
          style={{ marginBottom: 0, flex: 1, minWidth: 120 }}
          placeholder="作者（可选）"
          value={author}
          onChange={(e) => setAuthor(e.target.value)}
          disabled={running}
        />
        <select
          className="pm-input"
          style={{ marginBottom: 0, width: 200 }}
          value={model}
          onChange={(e) => setModel(e.target.value)}
          disabled={running}
        >
          <option value="">自动（默认模型）</option>
          {availableModels.map((m) => (
            <option key={m.model_id} value={m.model_id}>
              {m.display_name || m.model_id}
            </option>
          ))}
        </select>
      </div>

      {/* 维度勾选 */}
      <div style={{ display: "flex", gap: "var(--space-sm)", flexWrap: "wrap", marginBottom: "var(--space-sm)" }}>
        {DIMENSION_OPTIONS.map((d) => {
          const checked = dims.includes(d.slug);
          return (
            <label
              key={d.slug}
              title={d.hint}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                padding: "4px 10px",
                cursor: running ? "default" : "pointer",
                borderRadius: "var(--radius-sm)",
                border: `1px solid ${checked ? "var(--color-accent)" : "var(--color-rule-light)"}`,
                background: checked ? "var(--color-accent-wash)" : "transparent",
                fontSize: "var(--text-xs)",
              }}
            >
              <input type="checkbox" checked={checked} onChange={() => toggleDim(d.slug)} disabled={running} />
              {d.label}
            </label>
          );
        })}
        <button
          className="btn btn-secondary"
          style={{ padding: "2px 10px", fontSize: "var(--text-2xs)" }}
          onClick={() => setShowSample((s) => !s)}
          disabled={running}
        >
          {showSample ? "收起样章" : "粘贴样章（可选，更准）"}
        </button>
      </div>

      {/* 样章文本（可选） */}
      {showSample && (
        <textarea
          className="pm-textarea"
          rows={5}
          placeholder="粘贴样章文本（可选，≤2 万字）：提供后走「样章增强」高精度模式；若已上传书籍文件则以文件为准；都留空则基于模型知识储备蒸馏"
          value={sampleText}
          onChange={(e) => setSampleText(e.target.value)}
          disabled={running}
          style={{ marginBottom: "var(--space-sm)" }}
        />
      )}

      <div style={{ display: "flex", gap: "var(--space-sm)" }}>
        <button
          className="btn btn-primary"
          onClick={startDistill}
          disabled={running || (!title.trim() && !filePath) || dims.length === 0}
        >
          {running ? (
            <>
              <Loader2 size={14} className="spinning" /> 蒸馏中…（逐维度构卡，约需数分钟）
            </>
          ) : (
            <>
              <Sparkles size={14} /> 开始蒸馏（{dims.length} 个维度）
            </>
          )}
        </button>
        {!running && (
          <button className="btn btn-secondary" onClick={onClose}>
            取消
          </button>
        )}
      </div>

      {/* 分阶段进度 */}
      {phases.length > 0 && (
        <div style={{ marginTop: "var(--space-lg)", display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
          {phases.map((p, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: "var(--space-sm)",
                alignItems: "flex-start",
                padding: "var(--space-sm) var(--space-md)",
                background:
                  p.status === "error"
                    ? "var(--color-error-wash)"
                    : p.status === "done"
                      ? "var(--color-jade-wash)"
                      : "var(--color-subtle-bg)",
                borderRadius: "var(--radius-sm)",
                border: `1px solid ${
                  p.status === "error"
                    ? "var(--color-error)"
                    : p.status === "done"
                      ? "var(--color-jade)"
                      : "var(--color-rule-light)"
                }`,
              }}
            >
              <div style={{ flexShrink: 0, marginTop: 2 }}>
                {p.status === "running" && <Loader2 size={14} className="spinning" style={{ color: "var(--color-accent)" }} />}
                {p.status === "done" && <CheckCircle2 size={14} style={{ color: "var(--color-jade)" }} />}
                {p.status === "error" && <XCircle size={14} style={{ color: "var(--color-error)" }} />}
              </div>
              <div style={{ flex: 1, minWidth: 0 }}>
                <span style={{ fontSize: "var(--text-xs)", fontWeight: 600 }}>{p.phase}</span>
                <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginLeft: 8 }}>{p.message}</span>
                {p.detail && p.status === "done" && (
                  <details style={{ marginTop: 4 }}>
                    <summary style={{ fontSize: "var(--text-2xs)", color: "var(--color-accent)", cursor: "pointer" }}>详情</summary>
                    <pre
                      style={{
                        fontSize: "var(--text-2xs)",
                        marginTop: 4,
                        padding: "var(--space-sm)",
                        background: "var(--color-subtle-bg)",
                        borderRadius: "var(--radius-xs)",
                        whiteSpace: "pre-wrap",
                        maxHeight: 200,
                        overflow: "auto",
                      }}
                    >
                      {p.detail}
                    </pre>
                  </details>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
