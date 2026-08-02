// 批注中心 —— 聚合全部实体的批注，统一处理与跳转
import { useCallback, useEffect, useState } from "react";
import { MessageSquarePlus, Navigation } from "lucide-react";
import {
  annotationResolve,
  annotationsAll,
  distillPendingLessons,
  getPendingEdits,
} from "../ipc";
import type { EditSample } from "../ipc";
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
  const [edits, setEdits] = useState<EditSample[]>([]);
  const [loading, setLoading] = useState(true);
  const [distilling, setDistilling] = useState(false);
  const [distillMsg, setDistillMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [g, e] = await Promise.all([annotationsAll(), getPendingEdits()]);
      setGroups(g);
      setEdits(e);
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

  async function handleDistill() {
    setDistilling(true);
    setDistillMsg(null);
    try {
      const lessons = await distillPendingLessons();
      setDistillMsg(lessons.length > 0 ? `已沉淀 ${lessons.length} 条经验` : "没有可沉淀的修改");
      await load();
    } catch (e) {
      setDistillMsg("沉淀失败：" + ((e as Error)?.message ?? e));
    } finally {
      setDistilling(false);
    }
  }

  const SCOPE_LABEL: Record<string, string> = {
    chapter: "正文",
    outline: "大纲",
    world: "世界观",
    character: "人物志",
  };

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
        {edits.length > 0 && (
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ochre)" }}>
            · {edits.length} 条修改待沉淀
          </span>
        )}
        <button className="btn btn-secondary" style={{ marginLeft: "auto" }} onClick={load}>
          刷新
        </button>
      </div>

      {edits.length > 0 && (
        <div className="card" style={{ marginBottom: 16, padding: "var(--space-sm) var(--space-md)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
            <span style={{ fontWeight: 600, fontSize: "var(--text-sm)" }}>编辑修改样本</span>
            <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
              你在各环节保存的修改会自动进入此队列，沉淀为写作经验注入后续审查
            </span>
            <span style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
              {distillMsg && <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{distillMsg}</span>}
              <button
                className="btn btn-primary"
                style={{ padding: "3px 12px", fontSize: "var(--text-xs)" }}
                onClick={handleDistill}
                disabled={distilling}
              >
                {distilling ? "沉淀中…" : `沉淀为经验（${edits.length}）`}
              </button>
            </span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 4, maxHeight: 220, overflowY: "auto" }}>
            {edits.map(e => (
              <div key={e.sample_id} style={{ fontSize: "var(--text-2xs)", lineHeight: 1.6, padding: "5px 8px", borderRadius: "var(--radius-sm)", background: "var(--color-paper-warm)" }}>
                <span style={{ fontWeight: 600 }}>{e.label}</span>
                <span style={{ marginLeft: 6, color: "var(--color-ink-3)" }}>
                  [{SCOPE_LABEL[e.scope] ?? e.scope}]
                </span>
                <div style={{ color: "var(--color-ink-3)", marginTop: 2 }}>
                  <span style={{ color: "var(--color-error)" }}>改前：</span>{e.before}
                </div>
                <div style={{ color: "var(--color-ink-3)" }}>
                  <span style={{ color: "var(--color-jade)" }}>改后：</span>{e.after}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

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
