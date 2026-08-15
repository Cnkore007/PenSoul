// StatusBar — 底部状态栏（P2-13 修复：显示真实后端/LLM 状态，不再恒为"就绪"）

import { useState, useEffect } from "react";
import { getLlmStatus } from "../ipc";

export default function StatusBar() {
  const [status, setStatus] = useState("连接中…");

  useEffect(() => {
    let alive = true;
    getLlmStatus()
      .then((s) => {
        if (!alive) return;
        setStatus(
          s.has_default
            ? `就绪 · LLM 已配置（${s.configured_count} 个配置）`
            : "就绪 · LLM 未配置默认模型",
        );
      })
      .catch(() => {
        if (alive) setStatus("后端未连接");
      });
    return () => {
      alive = false;
    };
  }, []);

  return (
    <footer className="status-bar">
      <span className="status-left">PenSoul 2.0</span>
      <span className="status-right">{status}</span>
    </footer>
  );
}
