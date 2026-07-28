import { useState, useEffect } from "react";
import { BarChart3, Activity, ShieldAlert, Feather } from "lucide-react";
import { getStyleMetrics } from "../ipc";
import type { StyleMetrics } from "../types";

export function StyleWorkshop() {
  const [metrics, setMetrics] = useState<StyleMetrics | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => { loadMetrics(); }, []);

  async function loadMetrics() {
    setLoading(true);
    const result = await getStyleMetrics();
    setMetrics(result || { avg_sentence_length: 18.5, vocabulary_richness: 0.72, dialogue_ratio: 0.35, pace_score: 0.68, ai_pattern_score: 0.12 });
    setLoading(false);
  }

  const antiAIRules = [
    { id: 1, rule: "避免过度使用「然而」、「不过」等转折词", status: "pass" as const },
    { id: 2, rule: "减少「值得一提的是」等套话", status: "pass" as const },
    { id: 3, rule: "避免过度整齐的排比句", status: "warning" as const },
    { id: 4, rule: "增加口语化表达", status: "pass" as const },
    { id: 5, rule: "避免重复使用相同形容词", status: "fail" as const },
  ];

  const paceData = [
    { chapter: "第一章", score: 0.65 }, { chapter: "第二章", score: 0.78 },
    { chapter: "第三章", score: 0.52 }, { chapter: "第四章", score: 0.81 },
    { chapter: "第五章", score: 0.69 },
  ];

  const barColors = [
    "var(--color-ink)", "var(--color-accent)", "var(--color-ink-2)",
    "var(--color-indigo)", "var(--color-ochre)",
  ];

  if (loading) return <div className="loading-state">加载文风数据...</div>;

  return (
    <div className="view-container">
      <div className="view-header"><h2>墨韵品鉴</h2></div>
      <div className="stat-grid stat-grid-4">
        <div className="stat-card">
          <div className="stat-card-icon"><BarChart3 size={16} className="stat-icon-indigo" /><span>平均句长</span></div>
          <div className="stat-card-value">{metrics?.avg_sentence_length.toFixed(1)}</div>
          <div className="stat-card-unit">字/句</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><Activity size={16} className="stat-icon-jade" /><span>词汇丰富度</span></div>
          <div className="stat-card-value stat-color-success">{((metrics?.vocabulary_richness || 0) * 100).toFixed(0)}%</div>
          <div className="stat-card-unit">TTR指数</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><Feather size={16} className="stat-icon-ochre" /><span>对话占比</span></div>
          <div className="stat-card-value stat-color-warning">{((metrics?.dialogue_ratio || 0) * 100).toFixed(0)}%</div>
          <div className="stat-card-unit">对话/叙述</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><ShieldAlert size={16} className="stat-icon-alert" /><span>AI痕迹</span></div>
          <div className={"stat-card-value " + ((metrics?.ai_pattern_score || 0) > 0.2 ? "stat-color-error" : "stat-color-success")}>
            {((metrics?.ai_pattern_score || 0) * 100).toFixed(0)}%
          </div>
          <div className="stat-card-unit">越低越好</div>
        </div>
      </div>
      <div className="grid-2">
        <div className="card">
          <div className="card-header"><BarChart3 size={15} color="var(--color-ink-3)" /><h3>叙事节奏</h3></div>
          <div className="bar-chart">
            {paceData.map((item, i) => (
              <div key={item.chapter} className="bar-group">
                <div className="bar-fill" style={{ height: `${item.score * 100}%`, background: barColors[i % barColors.length] }} />
                <span className="bar-label">{item.chapter}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="card">
          <div className="card-header"><ShieldAlert size={15} color="var(--color-ink-3)" /><h3>反AI检查</h3></div>
          <div className="flex-col flex-gap-sm">
            {antiAIRules.map((rule) => (
              <div key={rule.id} className={"rule-row rule-" + rule.status}>
                <div className="rule-icon">{rule.status === "pass" ? "✓" : rule.status === "warning" ? "!" : "✗"}</div>
                <span className="rule-text">{rule.rule}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
