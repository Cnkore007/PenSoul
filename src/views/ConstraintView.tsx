// ConstraintView — 约束管理

import { useState } from "react";
import { checkConstraints } from "../ipc";
import type { ConstraintReport } from "../types";

export default function ConstraintView() {
  const [report, setReport] = useState<ConstraintReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  const handleCheck = async () => {
    setLoading(true);
    try { setReport(await checkConstraints()); setMsg(""); }
    catch (e: any) { setMsg(`检查失败: ${e}`); }
    finally { setLoading(false); }
  };

  return (
    <div className="view-card">
      <h2>约束管理</h2>
      <button className="btn-primary" onClick={handleCheck} disabled={loading}>
        {loading ? "检查中..." : "执行全量检查"}
      </button>
      {msg && <p className="msg">{msg}</p>}
      {report && (
        <div className="report">
          <div className="stats-row">
            <span>检查实体: {report.checked_entities}</span>
            <span className={report.has_issues ? "text-warn" : "text-ok"}>
              {report.has_issues ? `${report.error_count} 错误 / ${report.warning_count} 警告` : "全部通过"}
            </span>
          </div>
        </div>
      )}
      <div className="section" style={{ marginTop: "1rem" }}>
        <h3>内置约束</h3>
        <ul className="constraint-list">
          <li><span className="tag-hard">硬约束</span> 角色状态一致性</li>
          <li><span className="tag-hard">硬约束</span> 时间线顺序</li>
          <li><span className="tag-hard">硬约束</span> 设定规则</li>
          <li><span className="tag-hard">硬约束</span> 事件连续性</li>
          <li><span className="tag-hard">硬约束</span> 伏笔跟踪</li>
          <li><span className="tag-soft">软约束</span> 风格一致性</li>
          <li><span className="tag-soft">软约束</span> 伏笔平衡</li>
        </ul>
      </div>
    </div>
  );
}
