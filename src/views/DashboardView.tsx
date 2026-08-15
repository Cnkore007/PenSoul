// DashboardView — 仪表盘

import { useState, useEffect } from "react";
import { getProjectOverview } from "../ipc";
import type { ProjectOverview } from "../types";

export default function DashboardView() {
  const [overview, setOverview] = useState<ProjectOverview | null>(null);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    getProjectOverview().then(setOverview).catch(() => setMsg("加载失败"));
  }, []);

  if (!overview) return <div className="view-card"><p>{msg || "加载中..."}</p></div>;

  return (
    <div className="view-card">
      <h2>仪表盘</h2>
      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-value">{overview.character_count}</div>
          <div className="stat-label">角色</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{overview.event_count}</div>
          <div className="stat-label">事件</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{overview.setting_count}</div>
          <div className="stat-label">地点</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{overview.foreshadow_count}</div>
          <div className="stat-label">伏笔</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{overview.chapter_count}</div>
          <div className="stat-label">章节</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{overview.total_words.toLocaleString()}</div>
          <div className="stat-label">总字数</div>
        </div>
      </div>
        {overview.pipeline && (
          <div className="section">
            <h3>创作流水线</h3>
            <div className="pipeline-list">
              {overview.pipeline.stages.map((stage) => (
                <div key={stage.id} className={`pipeline-stage ${stage.ready ? "ready" : "pending"}`}>
                  <span className="pipeline-status">{stage.ready ? "✓" : "○"}</span>
                  <b>{stage.label}</b>
                  <span className="llm-hint">{stage.detail}</span>
                </div>
              ))}
            </div>
            <p className="msg">{overview.pipeline.next_action}</p>
          </div>
        )}
      {overview.high_concept && (
        <div className="section">
          <h3>核心概念</h3>
          <p>{overview.high_concept}</p>
        </div>
      )}
      {overview.tone && (
        <div className="section">
          <h3>基调</h3>
          <p>{overview.tone}</p>
        </div>
      )}
    </div>
  );
}
