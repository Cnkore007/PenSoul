import { useState, useEffect } from "react";
import { AlertTriangle, AlertCircle, Info, RefreshCw, CheckCircle2 } from "lucide-react";
import { checkConsistency } from "../ipc";
import type { ConsistencyViolation } from "../types";

export function ConsistencyView() {
  const [violations, setViolations] = useState<ConsistencyViolation[]>([]);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  useEffect(() => { loadViolations(); }, []);

  async function loadViolations() {
    setLoading(true);
    const result = await checkConsistency();
    setViolations(result);
    setLoading(false);
  }

  async function handleRefresh() {
    setChecking(true);
    const result = await checkConsistency();
    setViolations(result);
    setChecking(false);
  }

  const severityConfig = {
    Error: { icon: <AlertTriangle size={14} />, label: "错误" },
    Warning: { icon: <AlertCircle size={14} />, label: "警告" },
    Info: { icon: <Info size={14} />, label: "提示" },
  };

  const stats = {
    Error: violations.filter((v) => v.severity === "Error").length,
    Warning: violations.filter((v) => v.severity === "Warning").length,
    Info: violations.filter((v) => v.severity === "Info").length,
  };

  if (loading) return <div className="loading-state">审校中...</div>;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>审校录</h2>
        <button className={"btn btn-primary" + (checking ? " btn-disabled" : "")} onClick={handleRefresh} disabled={checking}>
          <RefreshCw size={15} className={checking ? "spinning" : ""} /> 刷新
        </button>
      </div>
      <div className="stat-grid stat-grid-3">
        {(["Error", "Warning", "Info"] as const).map((severity) => (
          <div key={severity} className={"stat-card flex-center flex-gap-sm" + (severity === "Error" ? " stat-bg-error" : severity === "Warning" ? " stat-bg-warning" : " stat-bg-info")}>
            <div className={"stat-card-icon " + (severity === "Error" ? "stat-color-error" : severity === "Warning" ? "stat-color-warning" : "stat-color-info")}>
              {severityConfig[severity].icon}
            </div>
            <div>
              <div className={"stat-card-value " + (severity === "Error" ? "stat-color-error" : severity === "Warning" ? "stat-color-warning" : "stat-color-info")}>
                {stats[severity]}
              </div>
              <div className="stat-card-unit">{severityConfig[severity].label}</div>
            </div>
          </div>
        ))}
      </div>
      <div className="tab-content">
        {violations.length === 0 ? (
          <div className="empty-state consistency-success">
            <div className="consistency-success-icon"><CheckCircle2 size={42} strokeWidth={1.5} /></div>
            <div className="empty-state-text">文理通达</div>
            <div className="empty-state-sub">前后文脉贯通无碍</div>
          </div>
        ) : violations.map((v) => (
          <div key={v.violation_id} className="list-item consistency-violation-item">
            <div className={"severity-icon-box severity-" + v.severity.toLowerCase()}>
              {severityConfig[v.severity].icon}
            </div>
            <div className="consistency-violation-content">
              <div className="consistency-violation-meta">
                <span className="severity-badge">{v.entity_type}</span>
                <span className="consistency-violation-chapters">章节 {v.chapter_a} ↔ 章节 {v.chapter_b}</span>
              </div>
              <p className="detail-desc">{v.description}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
