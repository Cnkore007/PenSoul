// 受控保存控制组 —— 「保存并审核」：分析批注/修改 → LLM 判定有效性 + 影响评估
// → 二次确认应用 → 快照记录可撤回 → 有效样本沉淀经验
import { useEffect, useState } from "react";
import { Save, Loader2, Undo2 } from "lucide-react";
import type { LlmModel } from "../types";
import {
  applyPageReview,
  listModels,
  pageUndoAvailable,
  reviewPageChanges,
  undoPageChange,
  type PageReview,
} from "../ipc";
import { ReviewConfirmModal } from "./ReviewConfirmModal";

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
        <ReviewConfirmModal
          review={review}
          verdicts={verdicts}
          setVerdicts={setVerdicts}
          applying={applying}
          onConfirm={handleApply}
          onCancel={() => setReview(null)}
        />
      )}
    </div>
  );
}
