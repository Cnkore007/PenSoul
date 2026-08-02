import type { ViewType } from "../types";

interface StatusBarProps {
  currentView: ViewType;
  wordCount: number;
  connected: boolean;
}

const viewLabels: Record<ViewType, string> = {
  dashboard: "概览",
  concept: "灵魂萌芽",
  writing: "笔耕",
  outline: "大纲",
  character: "人物志",
  world: "世界观",
  consistency: "审校",
  harness: "造化工坊",
  style: "墨韵",
  projects: "作品库",
  "llm-settings": "模型设置",
  "workflow-library": "工作流",
  experts: "专家库",
};

export function StatusBar({ currentView, wordCount, connected }: StatusBarProps) {
  return (
    <div className="status-bar">
      <div className="status-bar-left">
        <span className="status-view-name">{viewLabels[currentView]}</span>
        <span className="status-sep">&middot;</span>
        <span>已书 {wordCount.toLocaleString()} 字</span>
      </div>
      <div className="status-bar-right">
        <div className={`status-dot ${connected ? "connected" : ""}`} />
        <span>{connected ? "后端已连" : "后端断开"}</span>
      </div>
    </div>
  );
}
