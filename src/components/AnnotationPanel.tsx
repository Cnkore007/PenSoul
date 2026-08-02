import { useState } from "react";
import { MessageSquarePlus, Trash2, Check, X, PenLine, Pin } from "lucide-react";
import type { ChapterAnnotation } from "../types";

interface AnnotationPanelProps {
  annotations: ChapterAnnotation[];
  onAddChapterAnnotation: (kind: ChapterAnnotation["kind"], content: string) => void;
  onUpdate: (id: string, patch: Partial<ChapterAnnotation>) => void;
  onRemove: (id: string) => void;
  onLocate: (anno: ChapterAnnotation) => void;
}

const KIND_STYLE: Record<string, { label: string; color: string; bg: string }> = {
  issue: { label: "问题", color: "var(--color-error)", bg: "var(--color-error-wash)" },
  suggestion: { label: "修改建议", color: "var(--color-accent)", bg: "var(--color-accent-wash)" },
  note: { label: "备注", color: "var(--color-jade)", bg: "var(--color-jade-wash)" },
};

const STATUS_LABEL: Record<string, string> = {
  open: "待处理",
  accepted: "已采纳",
  rejected: "已拒绝",
};

export function AnnotationPanel({
  annotations,
  onAddChapterAnnotation,
  onUpdate,
  onRemove,
  onLocate,
}: AnnotationPanelProps) {
  const [showAdd, setShowAdd] = useState(false);
  const [addKind, setAddKind] = useState<ChapterAnnotation["kind"]>("issue");
  const [addText, setAddText] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");

  const openCount = annotations.filter(a => a.status === "open").length;

  function handleAdd() {
    if (!addText.trim()) return;
    onAddChapterAnnotation(addKind, addText.trim());
    setAddText("");
    setShowAdd(false);
  }

  return (
    <div className="writing-info-section">
      <div className="writing-info-label">
        批注
        <span style={{ fontWeight: 400, color: "var(--color-ink-3)", fontSize: "var(--text-2xs)", marginLeft: 6 }}>
          {openCount > 0 ? `${openCount} 条待处理` : "无待处理"}
        </span>
        <button className="pv-icon-btn" style={{ marginLeft: "auto" }} title="添加整体批注"
          onClick={() => { setShowAdd(!showAdd); setAddText(""); }}>
          <MessageSquarePlus size={13} />
        </button>
      </div>

      {showAdd && (
        <div style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", gap: 6, marginBottom: 4 }}>
            <select
              className="pm-input"
              style={{ marginBottom: 0, width: 110, padding: "2px 6px", fontSize: "var(--text-2xs)" }}
              value={addKind}
              onChange={e => setAddKind(e.target.value as ChapterAnnotation["kind"])}
            >
              <option value="issue">问题</option>
              <option value="suggestion">修改建议</option>
              <option value="note">备注</option>
            </select>
          </div>
          <textarea
            className="pm-textarea"
            rows={2}
            style={{ fontSize: "var(--text-2xs)" }}
            placeholder="整章意见，如：节奏太拖，结尾钩子不明确…"
            value={addText}
            onChange={e => setAddText(e.target.value)}
          />
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 6, marginTop: 4 }}>
            <button className="btn btn-primary" style={{ padding: "2px 10px", fontSize: "var(--text-2xs)" }}
              onClick={handleAdd} disabled={!addText.trim()}>添加</button>
          </div>
        </div>
      )}

      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {annotations.length === 0 && (
          <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
            选中正文文字可添加行内批注；也可在此添加整章批注
          </div>
        )}
        {annotations.map(a => {
          const st = KIND_STYLE[a.kind] ?? KIND_STYLE.note;
          return (
            <div key={a.annotation_id} style={{ fontSize: "var(--text-2xs)", lineHeight: 1.6, padding: "var(--space-xs) var(--space-sm)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                <span style={{ padding: "0 6px", borderRadius: 8, background: st.bg, color: st.color, fontWeight: 600, fontSize: "var(--text-2xs)" }}>
                  {st.label}
                </span>
                <span style={{ color: a.status === "accepted" ? "var(--color-jade)" : a.status === "rejected" ? "var(--color-ink-3)" : "var(--color-ochre)", fontWeight: 500 }}>
                  {STATUS_LABEL[a.status] ?? a.status}
                </span>
                <span style={{ marginLeft: "auto", display: "inline-flex", gap: 2 }}>
                  {a.anchor && (
                    <button className="pv-icon-btn" title="定位到正文" onClick={() => onLocate(a)}>
                      <Pin size={12} />
                    </button>
                  )}
                  <button className="pv-icon-btn" title="编辑"
                    onClick={() => { setEditingId(a.annotation_id); setEditText(a.content); }}>
                    <PenLine size={12} />
                  </button>
                  <button className="pv-icon-btn pv-icon-btn-danger" title="删除"
                    onClick={() => onRemove(a.annotation_id)}>
                    <Trash2 size={12} />
                  </button>
                </span>
              </div>
              {a.anchor && (
                <div style={{ color: "var(--color-ink-3)", marginBottom: 2, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  「{a.anchor.text.length > 24 ? a.anchor.text.slice(0, 24) + "…" : a.anchor.text}」
                </div>
              )}
              {editingId === a.annotation_id ? (
                <div>
                  <textarea
                    className="pm-textarea"
                    rows={2}
                    style={{ fontSize: "var(--text-2xs)" }}
                    value={editText}
                    autoFocus
                    onChange={e => setEditText(e.target.value)}
                  />
                  <div style={{ display: "flex", gap: 6, marginTop: 4, justifyContent: "flex-end" }}>
                    <button className="btn btn-primary" style={{ padding: "2px 10px", fontSize: "var(--text-2xs)" }}
                      onClick={() => { onUpdate(a.annotation_id, { content: editText.trim() }); setEditingId(null); }}>
                      <Check size={12} /> 保存
                    </button>
                    <button className="btn btn-secondary" style={{ padding: "2px 10px", fontSize: "var(--text-2xs)" }}
                      onClick={() => setEditingId(null)}>
                      <X size={12} /> 取消
                    </button>
                  </div>
                </div>
              ) : (
                <div style={{ color: "var(--color-ink-2)" }}>{a.content}</div>
              )}
              {a.status === "open" && (
                <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
                  <button className="btn btn-secondary" style={{ padding: "1px 8px", fontSize: "var(--text-2xs)" }}
                    onClick={() => onUpdate(a.annotation_id, { status: "accepted" })}>
                    标记已采纳
                  </button>
                  <button className="btn btn-secondary" style={{ padding: "1px 8px", fontSize: "var(--text-2xs)" }}
                    onClick={() => onUpdate(a.annotation_id, { status: "rejected" })}>
                    标记已拒绝
                  </button>
                </div>
              )}
              {a.status !== "open" && (
                <button className="btn btn-secondary" style={{ padding: "1px 8px", fontSize: "var(--text-2xs)", marginTop: 4 }}
                  onClick={() => onUpdate(a.annotation_id, { status: "open", processed_in_version: 0 })}>
                  重开
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
