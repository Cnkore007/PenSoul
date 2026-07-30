import { useEffect, useState } from "react";
import { Wand2, Loader2, Undo2 } from "lucide-react";
import type { LlmModel } from "../types";
import { listModels } from "../ipc";
import {
  startOptimize, undoOptimize, isOptimizing, hasOptimizeBackup, subscribeOptimize,
  type OptimizeType,
} from "../utils/optimize";

interface OptimizeControlsProps {
  type: OptimizeType;
  /** 优化前的页面内容 JSON（快照），随内容变化实时传入 */
  contentJson: string;
  /** 把优化/撤回结果写回项目数据（须为函数式更新，组件卸载后仍生效） */
  apply: (parsed: any) => void;
  /** 页面无内容时禁用 */
  disabled: boolean;
}

/**
 * 「优化」控制组：模型选择 + 优化 + 撤回。
 * 优化在全局管理器中执行，切换页面不中断；完成后结果自动写入项目数据。
 */
export function OptimizeControls({ type, contentJson, apply, disabled }: OptimizeControlsProps) {
  const [models, setModels] = useState<LlmModel[]>([]);
  const [modelId, setModelId] = useState("");
  const [optimizing, setOptimizing] = useState(isOptimizing(type));
  const [canUndo, setCanUndo] = useState(hasOptimizeBackup(type));
  const [msg, setMsg] = useState("");

  useEffect(() => {
    listModels().then(ms => {
      setModels(ms);
      const first = ms.find(m => m.is_available) || ms[0];
      if (first) setModelId(first.model_id);
    }).catch(() => {});
  }, []);

  // 订阅全局优化事件（只响应本类型）
  useEffect(() => {
    const unsub = subscribeOptimize(evt => {
      if (evt.type !== type) return;
      setOptimizing(evt.kind === "start");
      setCanUndo(hasOptimizeBackup(type));
      if (evt.message) {
        setMsg(evt.message);
        setTimeout(() => setMsg(""), evt.kind === "error" ? 6000 : 5000);
      }
    });
    return unsub;
  }, [type]);

  const handleOptimize = () => {
    setMsg("");
    startOptimize(type, contentJson, modelId || null, apply);
  };

  const handleUndo = () => {
    setMsg("");
    undoOptimize(type, apply);
  };

  return (
    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
      {msg && (
        <span style={{
          fontSize: "var(--text-xs)",
          color: msg.startsWith("优化失败") || msg.startsWith("撤回失败") ? "var(--color-error)" : "var(--color-jade)",
        }}>
          {msg}
        </span>
      )}
      <select
        className="pm-input"
        style={{ marginBottom: 0, width: 170, fontSize: "var(--text-xs)", padding: "4px 8px" }}
        value={modelId}
        onChange={e => setModelId(e.target.value)}
        disabled={optimizing}
        title="选择用于优化的模型"
      >
        {models.length === 0 && <option value="">默认模型</option>}
        {models.map(m => (
          <option key={m.model_id} value={m.model_id} disabled={!m.is_available}>
            {m.display_name}{!m.is_available ? "（未配置）" : ""}
          </option>
        ))}
      </select>
      <button className="btn btn-ghost" onClick={handleOptimize} disabled={optimizing || disabled} title="优化整理本页内容">
        {optimizing ? <Loader2 size={15} className="spinning" /> : <Wand2 size={15} />} 优化
      </button>
      {canUndo && (
        <button className="btn btn-secondary" style={{ padding: "4px 10px", fontSize: "var(--text-xs)" }} onClick={handleUndo} disabled={optimizing} title="恢复到优化之前">
          <Undo2 size={13} /> 撤回
        </button>
      )}
    </div>
  );
}
