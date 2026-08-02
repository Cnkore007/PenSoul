import { useState, useEffect } from "react";
import { BarChart3, Activity, ShieldAlert, Feather } from "lucide-react";
import { getStyleMetrics, analyzeAiFlavor } from "../ipc";
import type { AiFlavorReport, ProjectData, StyleMetrics } from "../types";

interface StyleWorkshopProps {
  projectData: ProjectData;
}

export function StyleWorkshop({ projectData }: StyleWorkshopProps) {
  const [metrics, setMetrics] = useState<StyleMetrics | null>(null);
  const [flavor, setFlavor] = useState<AiFlavorReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [checkedChapter, setCheckedChapter] = useState("");

  useEffect(() => { loadMetrics(); }, []);

  async function loadMetrics() {
    setLoading(true);
    try {
      const result = await getStyleMetrics();
      setMetrics(result);
      // 取最新一篇有正文的章节做反 AI 味检测（规则统计，非 LLM）
      const chapters = projectData.volumes
        .flatMap(v => v.chapters)
        .filter(c => (c.word_count ?? 0) > 0)
        .sort((a, b) => (b.chapter_no ?? 0) - (a.chapter_no ?? 0));
      if (chapters.length > 0) {
        const latest = chapters[0];
        const report = await analyzeAiFlavor(latest.content ?? "");
        setFlavor(report);
        setCheckedChapter(`第${latest.chapter_no}章《${latest.title || "未命名"}》`);
      }
    } finally {
      setLoading(false);
    }
  }

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

  if (!metrics) {
    return (
      <div className="view-container">
        <div className="view-header"><h2>墨韵</h2></div>
        <div className="empty-state">
          <div className="empty-state-icon">墨</div>
          <div className="empty-state-text">暂无文风数据</div>
          <div className="empty-state-sub">完成至少一个章节的写作后，系统将自动分析文风指标</div>
        </div>
      </div>
    );
  }

  return (
    <div className="view-container">
      <div className="view-header"><h2>墨韵</h2></div>
      <div className="stat-grid stat-grid-4">
        <div className="stat-card">
          <div className="stat-card-icon"><BarChart3 size={16} className="stat-icon-indigo" /><span>平均句长</span></div>
          <div className="stat-card-value">{metrics.avg_sentence_length.toFixed(1)}</div>
          <div className="stat-card-unit">字/句</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><Activity size={16} className="stat-icon-jade" /><span>词汇丰富度</span></div>
          <div className="stat-card-value stat-color-success">{(metrics.vocabulary_richness * 100).toFixed(0)}%</div>
          <div className="stat-card-unit">TTR指数</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><Feather size={16} className="stat-icon-ochre" /><span>对话占比</span></div>
          <div className="stat-card-value stat-color-warning">{(metrics.dialogue_ratio * 100).toFixed(0)}%</div>
          <div className="stat-card-unit">对话/叙述</div>
        </div>
        <div className="stat-card">
          <div className="stat-card-icon"><ShieldAlert size={16} className="stat-icon-alert" /><span>AI痕迹</span></div>
          <div className={"stat-card-value " + ((flavor?.score ?? 0) > 35 ? "stat-color-error" : (flavor?.score ?? 0) > 15 ? "stat-color-warning" : "stat-color-success")}>
            {flavor ? flavor.score.toFixed(0) : "—"}
          </div>
          <div className="stat-card-unit">{flavor ? `${flavor.level} · 越低越好` : "暂无正文可检测"}</div>
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
          {!flavor ? (
            <div className="empty-state-sub" style={{ padding: "12px 0" }}>
              暂无正文可检测：完成至少一个章节的写作后，这里会按标准自动检测 AI 痕迹
            </div>
          ) : (
            <>
              <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 8 }}>
                检测对象：{checkedChapter} · 共 {flavor.total_hits} 处命中 · {flavor.suggestion}
              </div>
              <div className="flex-col flex-gap-sm">
                {flavor.categories.map((cat) => {
                  const status = cat.hits === 0 ? "pass" : cat.score >= cat.max_score * 0.8 ? "fail" : "warning";
                  return (
                    <div key={cat.key} className={"rule-row rule-" + status}>
                      <div className="rule-icon">{status === "pass" ? "✓" : status === "warning" ? "!" : "✗"}</div>
                      <span className="rule-text">
                        {cat.label}：命中 {cat.hits} 处，扣 {cat.score.toFixed(0)}/{cat.max_score.toFixed(0)} 分
                        {cat.examples.length > 0 && (
                          <details style={{ marginTop: 4 }}>
                            <summary style={{ fontSize: "var(--text-2xs)", color: "var(--color-accent)", cursor: "pointer" }}>违例样例</summary>
                            <ul style={{ margin: "6px 0 0", paddingLeft: 18, fontSize: "var(--text-2xs)", color: "var(--color-ink-2)" }}>
                              {cat.examples.map((ex, i) => <li key={i}>{ex}</li>)}
                            </ul>
                          </details>
                        )}
                      </span>
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
