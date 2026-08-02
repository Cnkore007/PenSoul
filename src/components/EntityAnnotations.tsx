// 实体批注入口 —— 挂载到任意条目旁的批注按钮 + 抽屉面板
import { useState } from "react";
import { MessageSquarePlus } from "lucide-react";
import { useAnnotations } from "../utils/annotations";
import { AnnotationPanel } from "./AnnotationPanel";

interface EntityAnnotationsProps {
  target: string;
  title?: string;
}

export function EntityAnnotations({ target, title }: EntityAnnotationsProps) {
  const { annotations, open, setOpen, add, update, remove, resolve } = useAnnotations(target);
  const [panelKey, setPanelKey] = useState(0);
  const openCount = annotations.filter(a => a.status === "open").length;

  return (
    <div style={{ position: "relative", display: "inline-block" }}>
      <button
        className="pv-icon-btn"
        title={title ?? "批注"}
        onClick={() => {
          setOpen(!open);
          setPanelKey(k => k + 1);
        }}
      >
        <MessageSquarePlus size={14} />
        {openCount > 0 && (
          <span style={{
            position: "absolute", top: -5, right: -5,
            background: "var(--color-ochre)", color: "#fff",
            borderRadius: 8, fontSize: 10, lineHeight: "14px", minWidth: 14,
            padding: "0 3px", textAlign: "center",
          }}>{openCount}</span>
        )}
      </button>
      {open && (
        <div key={panelKey} style={{
          position: "absolute", top: "calc(100% + 6px)", right: 0, zIndex: 50,
          width: 320, maxHeight: 380, overflowY: "auto",
          background: "var(--color-paper)", border: "1px solid var(--color-ink-3)",
          borderRadius: "var(--radius-sm)", padding: "var(--space-sm)",
          boxShadow: "0 8px 24px rgba(0,0,0,0.18)",
        }}>
          <AnnotationPanel
            annotations={annotations}
            onAddChapterAnnotation={add}
            onUpdate={(id, patch) => update(id, {
              kind: patch.kind,
              content: patch.content,
              status: patch.status as string | undefined,
            })}
            onRemove={remove}
            onResolve={resolve}
            onLocate={() => {}}
          />
        </div>
      )}
    </div>
  );
}
