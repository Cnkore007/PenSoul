import { useState, useCallback } from "react";
import { Sparkles, Lightbulb, RefreshCw, ChevronDown, ChevronUp } from "lucide-react";
import type { InspirationItem } from "../types";
import { generateInspiration } from "../ipc";

interface InspirationPanelProps {
  /** 创作上下文类型：character | world | outline | writing */
  contextType: string;
  /** 当前项目上下文数据（JSON），用于 LLM 生成灵感 */
  contextData: string;
  /** 外部控制展开状态（可选，默认内部管理） */
  externalExpanded?: boolean;
  /** 展开状态变化回调 */
  onToggle?: () => void;
  /** 是否隐藏触发按钮（标题栏已有入口时使用） */
  hideTrigger?: boolean;
}

export function InspirationPanel({ contextType, contextData, externalExpanded, onToggle, hideTrigger }: InspirationPanelProps) {
  const [internalExpanded, setInternalExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [items, setItems] = useState<InspirationItem[]>([]);
  const [hasGenerated, setHasGenerated] = useState(false);

  // 支持外部或内部展开控制
  const expanded = externalExpanded !== undefined ? externalExpanded : internalExpanded;

  const toggleExpanded = useCallback(() => {
    if (onToggle) {
      onToggle();
    } else {
      setInternalExpanded(prev => !prev);
    }
  }, [onToggle]);

  const handleGenerate = useCallback(async () => {
    setLoading(true);
    try {
      const result = await generateInspiration(contextType, contextData);
      setItems(result);
      setHasGenerated(true);
    } finally {
      setLoading(false);
    }
  }, [contextType, contextData]);

  const handleToggle = useCallback(async () => {
    if (!hasGenerated) {
      setLoading(true);
      try {
        const result = await generateInspiration(contextType, contextData);
        setItems(result);
        setHasGenerated(true);
      } finally {
        setLoading(false);
      }
    }
    toggleExpanded();
  }, [hasGenerated, contextType, contextData, toggleExpanded]);

  return (
    <div className="inspiration-panel" style={{
      borderTop: hasGenerated ? "1px solid var(--color-border)" : "none",
      marginTop: hasGenerated ? "var(--space-md)" : 0,
    }}>
      {/* 触发按钮（可被标题栏按钮替代） */}
      {!hideTrigger && (
        <div style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 0",
          cursor: "pointer",
          userSelect: "none",
        }}
          onClick={handleToggle}
        >
          <Sparkles size={16} style={{ color: "var(--color-accent)" }} />
          <span style={{
            fontSize: "var(--text-sm)",
            fontWeight: 500,
            color: "var(--color-accent)",
            flex: 1,
          }}>
            灵感
          </span>
          {loading ? (
            <RefreshCw size={14} className="spin" style={{ color: "var(--color-ink-faint)" }} />
          ) : hasGenerated ? (
            <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-faint)" }}>
              {items.length} 条建议
            </span>
          ) : null}
          {hasGenerated && (
            expanded ? <ChevronUp size={14} style={{ flexShrink: 0 }} /> : <ChevronDown size={14} style={{ flexShrink: 0 }} />
          )}
        </div>
      )}

      {/* 灵感内容 */}
      {expanded && hasGenerated && (
        <div style={{
          display: "flex",
          flexDirection: "column",
          gap: 10,
          paddingBottom: "var(--space-sm)",
        }}>
          {items.length === 0 && !loading && (
            <div style={{
              fontSize: "var(--text-sm)",
              color: "var(--color-ink-faint)",
              padding: "12px 0",
              textAlign: "center",
            }}>
              暂无灵感建议，点击 <RefreshCw size={12} style={{ display: "inline", verticalAlign: "middle" }} /> 重新生成
            </div>
          )}
          {items.map((item, i) => (
            <div key={i} className="inspiration-card" style={{
              background: "var(--color-bg-soft)",
              borderRadius: 8,
              padding: "10px 12px",
              borderLeft: "3px solid var(--color-accent)",
            }}>
              <div style={{
                display: "flex",
                alignItems: "flex-start",
                gap: 6,
              }}>
                <Lightbulb size={14} style={{
                  color: "var(--color-accent)",
                  flexShrink: 0,
                  marginTop: 2,
                }} />
                <div>
                  <div style={{
                    fontSize: "var(--text-sm)",
                    fontWeight: 600,
                    marginBottom: 2,
                    color: "var(--color-ink)",
                  }}>
                    {item.title}
                  </div>
                  <div style={{
                    fontSize: "var(--text-xs)",
                    color: "var(--color-ink-2)",
                    lineHeight: 1.6,
                    whiteSpace: "pre-wrap",
                  }}>
                    {item.content}
                  </div>
                </div>
              </div>
            </div>
          ))}
          {!loading && hasGenerated && items.length > 0 && (
            <button
              className="btn btn-ghost"
              style={{
                fontSize: "var(--text-xs)",
                padding: "4px 8px",
                alignSelf: "flex-end",
                gap: 4,
              }}
              onClick={handleGenerate}
            >
              <RefreshCw size={12} /> 换一批
            </button>
          )}
        </div>
      )}
    </div>
  );
}
