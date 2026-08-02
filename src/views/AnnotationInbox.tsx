// 批注中心 —— 聚合全部实体的批注，统一处理与跳转
import { useCallback, useEffect, useState } from "react";
import { MessageSquarePlus, Navigation } from "lucide-react";
import { annotationResolve, annotationsAll } from "../ipc";
import type { ChapterAnnotation, ViewType } from "../types";

interface AnnotationGroup {
  target: string;
  label: string;
  annotations: ChapterAnnotation[];
}

const KIND_LABEL: Record<string, string> = {
  issue: "问题",
  suggestion: "建议",
  note: "备注",
};

// target 前缀 → 目标视图
function viewForTarget(target: string): ViewType {
  if (target.startsWith("chapter:") && target.endsWith(":body")) return "writing";
  if (target.startsWith("chapter:") || target.startsWith("outline_arc:")) return "outline";
  if (target.startsWith("character:")) return "character";
  return "world";
}

export function AnnotationInbox({ onNavigate }: { onNavigate: (v: ViewType) => void }) {
  const [groups, setGroups] = useState<AnnotationGroup[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setGroups(await annotationsAll());
    } catch (e) {
      console.error("加载批注汇总失败:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleResolve(group: AnnotationGroup, id: string, accept: boolean) {
    try {
      await annotationResolve(group.target, [{ annotation_id: id, accept }]);
      await load();
    } catch (e) {
      console.error("处理批注失败:", e);
    }
  }

  const totalOpen = groups.reduce(
    (n, g) => n + g.annotations.filter(a => a.status === "open").length,
    0
  );

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>批注中心</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
          {totalOpen > 0 ? `${totalOpen} 条待处理` : "全部处理完毕"}
        </span>
        <button className="btn btn-secondary" style={{ marginLeft: "auto" }} onClick={load}>
          刷新
        </button>
      </div>

      {loading ? (
        <div className="empty-state">
          <div className="empty-state-icon">注</div>
          <div className="empty-state-text">正在汇总批注…</div>
        </div>
      ) : groups.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">闲</div>
          <div className="empty-state-text">暂无批注</div>
          <div className="empty-state-sub">在笔耕、大纲、人物志、世界观中选中内容即可批注</div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {groups.map(group => (
            <div key={group.target} className="card" style={{ padding: "var(--space-sm) var(--space-md)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <MessageSquarePlus size={14} style={{ color: "var(--color-ink-3)" }} />
                <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>{group.label}</span>
                <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", fontFamily: "monospace" }}>
                  {group.target}
                </span>
                <span style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
                  <button
                    className="btn btn-secondary"
                    style={{ padding: "2px 10px", fontSize: "var(--text-2xs)" }}
                    onClick={() => onNavigate(viewForTarget(group.target))}
                  >
                    <Navigation size={11} /> 前往
                  </button>
                </span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {group.annotations.map(a => (
                  <div
                    key={a.annotation_id}
                    style={{
                      display: "flex", alignItems: "flex-start", gap: 8,
                      padding: "6px 10px", borderRadius: "var(--radius-sm)",
                      background: "var(--color-paper-warm)", fontSize: "var(--text-2xs)", lineHeight: 1.6,
                    }}
                  >
                    <span style={{
                      padding: "0 6px", borderRadius: 8, background: "var(--color-accent-wash)",
                      color: "var(--color-accent)", fontWeight: 600, flexShrink: 0,
                    }}>
                      {KIND_LABEL[a.kind] ?? a.kind}
                    </span>
                    <span style={{ flex: 1 }}>{a.content}</span>
                    <span style={{ color: a.status === "accepted" ? "var(--color-jade)" : a.status === "rejected" ? "var(--color-ink-3)" : "var(--color-ochre)", flexShrink: 0 }}>
                      {a.status === "open" ? "待处理" : a.status === "accepted" ? "已采纳" : "已拒绝"}
                    </span>
                    {a.status === "open" && (
                      <span style={{ display: "flex", gap: 4, flexShrink: 0 }}>
                        <button
                          className="btn btn-primary"
                          style={{ padding: "1px 8px", fontSize: "var(--text-2xs)" }}
                          onClick={() => handleResolve(group, a.annotation_id, true)}
                        >
                          采纳
                        </button>
                        <button
                          className="btn btn-secondary"
                          style={{ padding: "1px 8px", fontSize: "var(--text-2xs)" }}
                          onClick={() => handleResolve(group, a.annotation_id, false)}
                        >
                          拒绝
                        </button>
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
