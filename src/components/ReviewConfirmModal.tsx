// 受控保存确认面板 —— 「保存并审核」的二次确认模态
// 展示 LLM 判定（每条批注/修改的有效性 + 理由）与影响评估，确认后应用
import { Loader2, X } from "lucide-react";
import type { PageReview } from "../ipc";

interface ReviewConfirmModalProps {
  review: PageReview;
  verdicts: Record<string, string>;
  setVerdicts: (v: Record<string, string>) => void;
  applying: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ReviewConfirmModal({
  review,
  verdicts,
  setVerdicts,
  applying,
  onConfirm,
  onCancel,
}: ReviewConfirmModalProps) {
  const validCount = Object.values(verdicts).filter(v => v === "valid").length;
  return (
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
          <button className="pv-icon-btn" style={{ marginLeft: "auto" }} onClick={onCancel} title="取消">
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
                      onChange={e => setVerdicts({ ...verdicts, [item.id]: e.target.value })}
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
          <button className="btn btn-secondary" onClick={onCancel} disabled={applying}>取消</button>
          <button className="btn btn-primary" onClick={onConfirm} disabled={applying}>
            {applying ? <Loader2 size={14} className="spinning" /> : null} 确认保存
          </button>
        </div>
      </div>
    </div>
  );
}
