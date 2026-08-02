import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Loader2, CheckCircle2, XCircle, Sparkles, GraduationCap } from "lucide-react";
import type { BookPackage, LlmModel } from "../types";
import { distillMethodology, getDistillState } from "../ipc";

// 方法论六个蒸馏维度（与后端 METHODOLOGY_DIMENSIONS 一致）
export const METHODOLOGY_DIMENSION_OPTIONS = [
  { slug: "style", label: "文风规则", hint: "语言铁律 / 去套话 / 去 AI 味" },
  { slug: "structure", label: "结构与编排", hint: "章节/场景编排 / 节拍表 / 钩子" },
  { slug: "character", label: "人物塑造", hint: "动机 / 对话 / 人物状态" },
  { slug: "tension", label: "冲突与张力", hint: "冲突升级 / 爽点 / 事件冷却" },
  { slug: "genre", label: "类型范式", hint: "题材惯例 / 开篇节奏" },
  { slug: "review", label: "审查标准", hint: "评分维度 / 门禁标准" },
];

interface PhaseEvent {
  phase: string;
  status: string;
  message: string;
  detail: string;
}

interface MethodologyDistillPanelProps {
  models: LlmModel[];
  // pkg 为 null 表示任务在后台完成（页面切换后重连场景），面板只需刷新列表
  onDistilled: (pkg: BookPackage | null) => void;
  onClose: () => void;
}

// 方法论蒸馏面板：标题 + 方法论文本 + 维度勾选 + 模型选择 + 分阶段进度
export function MethodologyDistillPanel({ models, onDistilled, onClose }: MethodologyDistillPanelProps) {
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [dims, setDims] = useState<string[]>(METHODOLOGY_DIMENSION_OPTIONS.map((d) => d.slug));
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

  // 页面切换后重连：若方法论蒸馏仍在后台进行，重放缓冲事件并订阅实时进度与终态
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const st = await getDistillState().catch(() => null);
      if (!st || cancelled || !st.running || st.kind !== "methodology") return;
      setRunning(true);
      (st.events ?? []).forEach(ev => {
        if (ev.phase === "__distill__") return;
        setPhases(prev => upsertPhase(prev, ev));
      });
      const unlisten = await listen<PhaseEvent>("methodology-distill-phase", (evt) => {
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

  async function startDistill() {
    if (!title.trim() || !text.trim() || dims.length === 0) return;
    setRunning(true);
    setPhases([]);

    unlistenRef.current = await listen<PhaseEvent>("methodology-distill-phase", (evt) => {
      setPhases(prev => upsertPhase(prev, evt.payload));
    });

    try {
      const pkg = await distillMethodology(title.trim(), text, dims, model || null);
      onDistilled(pkg);
      setTitle("");
      setText("");
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
        <GraduationCap size={18} style={{ color: "var(--color-accent)" }} />
        <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
          蒸馏方法论
        </span>
        <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          方法论：skills/pensoul-skill-Methodology
        </span>
      </div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginBottom: "var(--space-md)" }}>
        粘贴一段写作方法论（经验贴 / 讲稿 / 课程摘录），提炼为可绑定的写作技能卡，保存到 WritingCard 文件夹
      </div>

      <div style={{ display: "flex", gap: "var(--space-sm)", alignItems: "center", flexWrap: "wrap", marginBottom: "var(--space-sm)" }}>
        <input
          className="pm-input"
          style={{ marginBottom: 0, flex: 2, minWidth: 160 }}
          placeholder="方法论名称（必填），如：猫神写作经验、冲突设计方法论"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={running}
        />
        <select
          className="pm-input"
          style={{ marginBottom: 0, width: 200 }}
          value={model}
          onChange={(e) => setModel(e.target.value)}
          disabled={running}
        >
          <option value="">自动（第一个可用模型）</option>
          {availableModels.map((m) => (
            <option key={m.model_id} value={m.model_id}>
              {m.display_name || m.model_id}
            </option>
          ))}
        </select>
      </div>

      <textarea
        className="pm-textarea"
        rows={6}
        placeholder="粘贴方法论文本（必填，≤2 万字）：一段方法论只有落到「什么场景下怎么做、做完怎么判断完成」才配成为技能卡"
        value={text}
        onChange={(e) => setText(e.target.value)}
        disabled={running}
        style={{ marginBottom: "var(--space-sm)" }}
      />

      <div style={{ display: "flex", gap: "var(--space-sm)", flexWrap: "wrap", marginBottom: "var(--space-sm)" }}>
        {METHODOLOGY_DIMENSION_OPTIONS.map((d) => {
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
      </div>

      <div style={{ display: "flex", gap: "var(--space-sm)" }}>
        <button
          className="btn btn-primary"
          onClick={startDistill}
          disabled={running || !title.trim() || !text.trim() || dims.length === 0}
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
