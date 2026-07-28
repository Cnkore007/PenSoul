import type { ViewType } from "../types";

interface StatusBarProps {
  currentView: ViewType;
  wordCount: number;
  connected: boolean;
}

const viewLabels: Record<ViewType, string> = {
  dashboard: "项目概览",
  writing: "笔耕",
  outline: "经纬",
  character: "人物",
  world: "山河",
  consistency: "审校",
  harness: "造化引擎",
  style: "墨韵品鉴",
  projects: "作品库",
  "llm-settings": "模型设置",
  plugins: "造化工坊",
  workflow: "工作流",
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
