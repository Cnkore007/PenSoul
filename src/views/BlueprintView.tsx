// BlueprintView — 蓝图（只读展示）

import { useState, useEffect } from "react";
import { getBlueprint, getProjectOverview } from "../ipc";

export default function BlueprintView() {
  const [blueprint, setBlueprint] = useState<any>(null);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    getProjectOverview().then(() => {
      getBlueprint().then(setBlueprint).catch(() => setMsg("加载失败"));
    }).catch(() => setMsg("加载失败"));
  }, []);

  if (!blueprint) return <div className="view-card"><p>{msg || "加载中..."}</p></div>;

  return (
    <div className="view-card">
      <h2>蓝图</h2>
      <div className="stats-row">
        <span className={blueprint.settled ? "text-ok" : "text-warn"}>
          {blueprint.settled ? "已定盘" : "未定盘"}
        </span>
        <span>承诺: {blueprint.commitment_count}</span>
        <span>卷: {blueprint.volume_count}</span>
        <span>伏笔: {blueprint.foreshadow_count}</span>
      </div>

      {blueprint.commitments?.length > 0 && (
        <div className="section">
          <h3>承诺</h3>
          <div className="entity-grid">
            {blueprint.commitments.map((c: any) => (
              <div key={c.id} className="entity-card">
                <span className="entity-type">{c.kind}</span>
                <span className="entity-name">{c.statement}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {blueprint.volumes?.length > 0 && (
        <div className="section">
          <h3>卷蓝图</h3>
          <div className="entity-grid">
            {blueprint.volumes.map((v: any) => (
              <div key={v.volume_no} className="entity-card">
                <span className="entity-type">卷{v.volume_no}</span>
                <span className="entity-name">{v.title}</span>
                <span className="entity-detail">{v.one_line}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {blueprint.commitment_count === 0 && blueprint.volume_count === 0 && (
        <p className="empty">蓝图为空，等待讨论成果定盘。</p>
      )}
    </div>
  );
}
