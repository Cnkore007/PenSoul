// 受控保存控制组 —— 「保存并审核」：分析批注/修改 → LLM 判定有效性 + 影响评估
// → 二次确认应用 → 快照记录可撤回 → 有效样本沉淀经验
import { useEffect, useState } from "react";
import { Save, Loader2, Undo2, X } from "lucide-react";
import type { LlmModel } from "../types";
import {
  applyPageReview,
  listModels,
  pageUndoAvailable,
  reviewPageChanges,
  undoPageChange,
  type PageReview,
} from "../ipc";

interface SaveControlsProps {
  type: "world" | "character";
  /** 当前页面数据 JSON（保存/判定对象） */
  contentJson: string;
  /** 把确认后的数据写回项目数据（函数式更新） */
  apply: (parsed: any) => void;
  disabled: boolean;
}

export function SaveControls({ type, contentJson, apply, disabled }: SaveControlsProps) {
  const [models, setModels] = useState<LlmModel[]>([]);
  const [modelId, setModelId] = useState("");
  const [review, setReview] = useState<PageReview | null>(null);
  const [verdicts, setVerdicts] = useState<Record<string, string>>({});
  const [analyzing, setAnalyzing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [canUndo, setCanUndo] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    listModels().then(ms => {
      setModels(ms);
      const first = ms.find(m => m.is_available) || ms[0];
      if (first) setModelId(first.model_id);
    }).catch(() => {});
    pageUndoAvailable(type).then(setCanUndo).catch(() => {});
  }, [type]);

  function flash(m: string, isError = false) {
    setMsg(m);
    setTimeout(() => setMsg(""), isError ? 6000 : 5000);
  }

  async function handleAnalyze() {
    setAnalyzing(true);
    setMsg("");
    try {
      const r = await reviewPageChanges(type, contentJson);
      setReview(r);
      const v: Record<string, string> = {};
      for (const item of r.items) v[item.id] = item.verdict || "uncertain";
      setVerdicts(v);
      if (r.items.length === 0) flash("本页没有待审核的批注或修改");
    } catch (e) {
      flash("分析失败：" + ((e as Error)?.message ?? e), true);
    } finally {
      setAnalyzing(false);
    }
  }

  async function handleApply() {
    setApplying(true);
    try {
      const confirmations = review!.items.map(item => ({ id: item.id, verdict: verdicts[item.id] ?? "uncertain" }));
      const result = await applyPageReview(type, contentJson, confirmations);
      apply(JSON.parse(contentJson));
      setReview(null);
      setCanUndo(result.can_undo);
      flash(`已保存并沉淀 ${result.lessons.length} 条经验，可撤回`);
    } catch (e) {
      flash("应用失败：" + ((e as Error)?.message ?? e), true);
    } finally {
      setApplying(false);
    }
  }

  async function handleUndo() {
    setMsg("");
    try {
      const before = await undoPageChange(type);
      apply(before);
      setCanUndo(false);
      flash("已撤回上次受控保存");
    } catch (e) {
      flash("撤回失败：" + ((e as Error)?.message ?? e), true);
    }
  }

  const validCount = Object.values(verdicts).filter(v => v === "valid").length;

  return (
    <div style={{ display: "flex", gap: 8, alignItems: "center", position: "relative" }}>
      {msg && (
        <span style={{ fontSize: "var(--text-xs)", color: msg.startsWith("失败") ? "var(--color-error)" : "var(--color-jade)" }}>
          {msg}
        </span>
      )}
      <select
        className="pm-input"
        style={{ marginBottom: 0, width: 150, fontSize: "var(--text-xs)", padding: "4px 8px" }}
        value={modelId}
        onChange={e => setModelId(e.target.value)}
        disabled={analyzing || applying}
        title="选择用于判定的模型"
      >
        {models.length === 0 && <option value="">默认模型</option>}
        {models.map(m => (
          <option key={m.model_id} value={m.model_id} disabled={!m.is_available}>
            {m.display_name}{!m.is_available ? "（未配置）" : ""}
          </option>
        ))}
      </select>
      <button className="btn btn-primary" onClick={handleAnalyze} disabled={analyzing || applying || disabled} title="分析本页批注与修改，判定有效性并评估对全文影响">
        {analyzing ? <Loader2 size={15} className="spinning" /> : <Save size={15} />} 保存并审核
      </button>
      {canUndo && (
        <button className="btn btn-secondary" style={{ padding: "4px 10px", fontSize: "var(--text-xs)" }} onClick={handleUndo} disabled={analyzing || applying} title="恢复到上次受控保存之前">
          <Undo2 size={13} /> 撤回
        </button>
      )}

      {/* 判定面板：二次确认 */}
      {review && (
        <div style={{
          position: "fixed", inset: 0, zIndex: 100,
          display: "flex", alignItems: "center", justifyContent: "center",
          background: "rgba(0,0,0,0.35)",
        }}>
          <div style={{
            width: 720, maxHeight: "80vh", overflowY: "auto",
            background: "var(--color-paper)", borderRadius: "var(--radius-md)",
            padding: "var(--space-md)", boxShadow: "0 16px 48px rgba(0,0,0,0.28)",
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
              <span style={{ fontWeight: 700, fontSize: "var(--text-md)" }}>保存确认</span>
              <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                {review.items.length} 条待判定 · {validCount} 条有效
              </span>
              <button className="pv-icon-btn" style={{ marginLeft: "auto" }} onClick={() => setReview(null)} title="取消">
                <X size={15} />
              </button>
            </div>

            {review.items.length === 0 ? (
              <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", padding: "12px 0" }}>
                {review.impact}
              </div>
            ) : (
              <>
                <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
                  {review.items.map(item => (
                    <div key={item.id} style={{
                      padding: "8px 10px", borderRadius: "var(--radius-sm)",
                      background: "var(--color-paper-warm)", fontSize: "var(--text-2xs)", lineHeight: 1.6,
                    }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                        <span style={{ fontWeight: 600 }}>{item.label}</span>
                        <span style={{ color: "var(--color-ink-3)", fontFamily: "monospace" }}>{item.source}</span>
                        <select
                          className="pm-input"
                          style={{ marginBottom: 0, marginLeft: "auto", width: 92, padding: "1px 4px", fontSize: "var(--text-2xs)" }}
                          value={verdicts[item.id] ?? "uncertain"}
                          onChange={e => setVerdicts(prev => ({ ...prev, [item.id]: e.target.value }))}
                        >
                          <option value="valid">有效</option>
                          <option value="invalid">无效</option>
                          <option value="uncertain">待商榷</option>
                        </select>
                      </div>
                      <div style={{ whiteSpace: "pre-wrap", color: "var(--color-ink-2)" }}>{item.content}</div>
                      <div style={{ color: "var(--color-ink-3)", marginTop: 2 }}>
                        <span style={{ fontWeight: 600 }}>判定理由：</span>{item.reason}
                      </div>
                    </div>
                  ))}
                </div>
                <div style={{
                  padding: "8px 10px", borderRadius: "var(--radius-sm)",
                  background: "var(--color-accent-wash)", color: "var(--color-accent)",
                  fontSize: "var(--text-2xs)", lineHeight: 1.6, marginBottom: 14,
                }}>
                  <span style={{ fontWeight: 600 }}>对全文的影响：</span>{review.impact}
                </div>
              </>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
              <button className="btn btn-secondary" onClick={() => setReview(null)} disabled={applying}>取消</button>
              <button className="btn btn-primary" onClick={handleApply} disabled={applying}>
                {applying ? <Loader2 size={14} className="spinning" /> : null} 确认保存
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
