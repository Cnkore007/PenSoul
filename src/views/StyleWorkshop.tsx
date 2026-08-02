import { useState, useEffect, useCallback } from "react";
import {
  BarChart3, Activity, ShieldAlert, Feather, AlertTriangle, AlertCircle,
  Info, CheckCircle2, RefreshCw, Save,
} from "lucide-react";
import {
  analyzeAiFlavor,
  checkConsistency,
  getAntiAiRules,
  getStyleFingerprint,
  getStyleMetrics,
  saveAntiAiRules,
  type AntiAiRuleConfig,
} from "../ipc";
import type { AiFlavorReport, ConsistencyViolation, ProjectData, StyleFingerprint, StyleMetrics } from "../types";

interface StyleWorkshopProps {
  projectData: ProjectData;
}

export function StyleWorkshop({ projectData }: StyleWorkshopProps) {
  const [metrics, setMetrics] = useState<StyleMetrics | null>(null);
  const [flavor, setFlavor] = useState<AiFlavorReport | null>(null);
  const [fingerprint, setFingerprint] = useState<StyleFingerprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [checkedChapter, setCheckedChapter] = useState("");

  // 一致性检查（原审校页）
  const [violations, setViolations] = useState<ConsistencyViolation[]>([]);
  const [checking, setChecking] = useState(false);

  // 反 AI 味规则配置
  const [rules, setRules] = useState<AntiAiRuleConfig | null>(null);
  const [savingRules, setSavingRules] = useState(false);
  const [rulesMsg, setRulesMsg] = useState("");

  const loadMetrics = useCallback(async () => {
    setLoading(true);
    try {
      const result = await getStyleMetrics();
      setMetrics(result);
      getStyleFingerprint().then(setFingerprint).catch(() => {});
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
  }, [projectData.volumes]);

  const loadConsistency = useCallback(async (silent = false) => {
    if (!silent) setChecking(true);
    try {
      setViolations(await checkConsistency());
    } finally {
      setChecking(false);
    }
  }, []);

  const loadRules = useCallback(async () => {
    try {
      setRules(await getAntiAiRules());
    } catch (e) {
      console.error("加载反 AI 规则失败:", e);
    }
  }, []);

  useEffect(() => {
    void loadMetrics();
    void loadConsistency(true);
    void loadRules();
  }, [loadMetrics, loadConsistency, loadRules]);

  async function handleSaveRules() {
    if (!rules) return;
    setSavingRules(true);
    setRulesMsg("");
    try {
      await saveAntiAiRules(rules);
      setRulesMsg("已保存，写作与审查工作流将按新规则执行");
      setTimeout(() => setRulesMsg(""), 5000);
    } catch (e) {
      setRulesMsg("保存失败：" + ((e as Error)?.message ?? e));
    } finally {
      setSavingRules(false);
    }
  }

  function updateCategory(index: number, patch: Partial<AntiAiRuleConfig["categories"][number]>) {
    if (!rules) return;
    setRules({
      ...rules,
      categories: rules.categories.map((c, i) => (i === index ? { ...c, ...patch } : c)),
    });
  }

  const severityConfig = {
    Error: { icon: <AlertTriangle size={14} />, label: "错误" },
    Warning: { icon: <AlertCircle size={14} />, label: "警告" },
    Info: { icon: <Info size={14} />, label: "提示" },
  };
  const stats = {
    Error: violations.filter(v => v.severity === "Error").length,
    Warning: violations.filter(v => v.severity === "Warning").length,
    Info: violations.filter(v => v.severity === "Info").length,
  };

  if (loading && !metrics) return <div className="loading-state">加载墨韵数据...</div>;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>墨韵</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
          文风 · 反 AI 味 · 一致性审校 · 规则配置
        </span>
      </div>

      {metrics && (
        <>
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

          <div className="grid-2" style={{ marginTop: 0 }}>
            <div className="card">
              <div className="card-header"><ShieldAlert size={15} color="var(--color-ink-3)" /><h3>反AI检查</h3></div>
              {!flavor ? (
                <div className="empty-state-sub" style={{ padding: "12px 0" }}>
                  暂无正文可检测：完成至少一个章节的写作后，这里会按当前规则自动检测 AI 痕迹
                </div>
              ) : (
                <>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 8 }}>
                    检测对象：{checkedChapter} · 共 {flavor.total_hits} 处命中 · {flavor.suggestion}
                  </div>
                  {flavor.rhythm && (
                    <div className="rule-row" style={{ marginBottom: 8, fontSize: "var(--text-2xs)", color: flavor.rhythm.flagged ? "var(--color-warning)" : "var(--color-jade)" }}>
                      <span className="rule-icon">{flavor.rhythm.flagged ? "!" : "✓"}</span>
                      <span className="rule-text">
                        节奏：均句 {flavor.rhythm.avg_sentence_length.toFixed(1)} 字 · 句长方差 {flavor.rhythm.sentence_var.toFixed(1)} · 段落均匀度 {flavor.rhythm.paragraph_uniformity.toFixed(2)}。{flavor.rhythm.note}
                      </span>
                    </div>
                  )}
                  <div className="flex-col flex-gap-sm">
                    {flavor.categories.map(cat => {
                      const status = cat.hits === 0 ? "pass" : cat.score >= cat.max_score * 0.8 ? "fail" : "warning";
                      return (
                        <div key={cat.key} className={"rule-row rule-" + status}>
                          <div className="rule-icon">{status === "pass" ? "✓" : status === "warning" ? "!" : "✗"}</div>
                          <span className="rule-text">
                            {cat.label}
                            <span style={{ marginLeft: 6, padding: "0 6px", borderRadius: 8, background: "rgba(128,128,128,0.15)", fontSize: "var(--text-2xs)" }}>
                              T{cat.tier}
                            </span>
                            ：命中 {cat.hits} 处，扣 {cat.score.toFixed(0)}/{cat.max_score.toFixed(0)} 分
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

            <div className="card">
              <div className="card-header">
                <ShieldAlert size={15} color="var(--color-ink-3)" /><h3>一致性审校</h3>
                <button className="btn btn-secondary" style={{ marginLeft: "auto", padding: "2px 10px", fontSize: "var(--text-2xs)" }} onClick={() => loadConsistency()} disabled={checking}>
                  <RefreshCw size={13} className={checking ? "spinning" : ""} /> 刷新
                </button>
              </div>
              <div className="stat-grid stat-grid-3" style={{ marginBottom: 10 }}>
                {(["Error", "Warning", "Info"] as const).map(s => (
                  <div key={s} className={"stat-card flex-center flex-gap-sm" + (s === "Error" ? " stat-bg-error" : s === "Warning" ? " stat-bg-warning" : " stat-bg-info")}>
                    <div className={"stat-card-icon " + (s === "Error" ? "stat-color-error" : s === "Warning" ? "stat-color-warning" : "stat-color-info")}>
                      {severityConfig[s].icon}
                    </div>
                    <div>
                      <div className={"stat-card-value " + (s === "Error" ? "stat-color-error" : s === "Warning" ? "stat-color-warning" : "stat-color-info")}>{stats[s]}</div>
                      <div className="stat-card-unit">{severityConfig[s].label}</div>
                    </div>
                  </div>
                ))}
              </div>
              <div style={{ maxHeight: 240, overflowY: "auto" }}>
                {violations.length === 0 ? (
                  <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--color-jade)", fontSize: "var(--text-xs)" }}>
                    <CheckCircle2 size={16} /> 文理通达，前后文脉贯通无碍
                  </div>
                ) : violations.map(v => (
                  <div key={v.violation_id} className="list-item consistency-violation-item" style={{ padding: "6px 8px" }}>
                    <div className={"severity-icon-box severity-" + v.severity.toLowerCase()}>{severityConfig[v.severity].icon}</div>
                    <div className="consistency-violation-content">
                      <div className="consistency-violation-meta">
                        <span className="severity-badge">{v.entity_type}</span>
                        <span className="consistency-violation-chapters">章节 {v.chapter_a} ↔ 章节 {v.chapter_b}</span>
                      </div>
                      <p className="detail-desc" style={{ margin: 0 }}>{v.description}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {fingerprint && (
            <div className="card" style={{ marginTop: 16 }}>
              <div className="card-header">
                <Feather size={15} color="var(--color-ink-3)" /><h3>本书文风指纹</h3>
                <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                  采样 {fingerprint.sampled_chapters} 章 · 注入写作/审查作为文风基线，防止越改越模板
                </span>
              </div>
              <div className="stat-grid stat-grid-4">
                <div className="stat-card">
                  <div className="stat-card-value">{fingerprint.avg_sentence_length.toFixed(1)}</div>
                  <div className="stat-card-unit">平均句长（字）</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{fingerprint.sentence_var.toFixed(1)}</div>
                  <div className="stat-card-unit">句长方差（越小越齐）</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{(fingerprint.connector_per_1k).toFixed(1)}</div>
                  <div className="stat-card-unit">连接词/千字</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{fingerprint.quote_style || "—"}</div>
                  <div className="stat-card-unit">引号习惯</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{(fingerprint.dialogue_ratio * 100).toFixed(0)}%</div>
                  <div className="stat-card-unit">对话占比</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{(fingerprint.vocabulary_richness * 100).toFixed(0)}%</div>
                  <div className="stat-card-unit">词汇丰富度</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{fingerprint.avg_paragraph_length.toFixed(0)}</div>
                  <div className="stat-card-unit">平均段落长（字）</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-value">{fingerprint.paragraph_uniformity.toFixed(2)}</div>
                  <div className="stat-card-unit">段落均匀度</div>
                </div>
              </div>
            </div>
          )}
        </>
      )}

      {/* 反 AI 味规则配置 */}
      <div className="card" style={{ marginTop: 16 }}>
        <div className="card-header">
          <ShieldAlert size={15} color="var(--color-ink-3)" /><h3>反 AI 味规则配置</h3>
          <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 8, fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
            保存后带入写作 / 审查 / 批注重写工作流
            {rulesMsg && <span style={{ color: rulesMsg.startsWith("失败") ? "var(--color-error)" : "var(--color-jade)" }}>{rulesMsg}</span>}
            <button className="btn btn-primary" style={{ padding: "3px 12px", fontSize: "var(--text-xs)" }} onClick={handleSaveRules} disabled={savingRules || !rules}>
              <Save size={13} /> {savingRules ? "保存中..." : "保存规则"}
            </button>
          </span>
        </div>
        {rules ? (
          <>
            <div style={{ marginBottom: 12 }}>
              <div className="writing-info-label" style={{ marginBottom: 4 }}>语言铁律（提示词，注入写作/审查）</div>
              <textarea
                className="pm-textarea"
                rows={7}
                style={{ fontFamily: "monospace", fontSize: "var(--text-xs)", lineHeight: 1.7 }}
                value={rules.prompt}
                onChange={e => setRules({ ...rules, prompt: e.target.value })}
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {rules.categories.map((c, i) => (
                <div key={c.key} style={{ padding: "8px 10px", borderRadius: "var(--radius-sm)", background: "var(--color-paper-warm)" }}>
                  <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6, flexWrap: "wrap" }}>
                    <span style={{ fontWeight: 600, fontSize: "var(--text-xs)" }}>{c.label}</span>
                    <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "inline-flex", gap: 4, alignItems: "center" }}>
                      分级T
                      <input className="pm-input" type="number" min={1} max={3} style={{ marginBottom: 0, width: 44, padding: "1px 4px" }}
                        value={c.tier} onChange={e => updateCategory(i, { tier: Math.max(1, Math.min(3, Number(e.target.value) || 1)) })} />
                    </label>
                    <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "inline-flex", gap: 4, alignItems: "center" }}>
                      单次扣分
                      <input className="pm-input" type="number" step="0.5" style={{ marginBottom: 0, width: 56, padding: "1px 4px" }}
                        value={c.score_per_hit} onChange={e => updateCategory(i, { score_per_hit: Number(e.target.value) || 0 })} />
                    </label>
                    <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "inline-flex", gap: 4, alignItems: "center" }}>
                      扣分上限
                      <input className="pm-input" type="number" step="1" style={{ marginBottom: 0, width: 56, padding: "1px 4px" }}
                        value={c.max_score} onChange={e => updateCategory(i, { max_score: Number(e.target.value) || 0 })} />
                    </label>
                    <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "inline-flex", gap: 4, alignItems: "center" }}>
                      每千字豁免
                      <input className="pm-input" type="number" step="1" style={{ marginBottom: 0, width: 56, padding: "1px 4px" }}
                        value={c.exempt_per_1k} onChange={e => updateCategory(i, { exempt_per_1k: Number(e.target.value) || 0 })} />
                    </label>
                    <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{c.suggestion}</span>
                  </div>
                  <textarea
                    className="pm-textarea"
                    rows={2}
                    style={{ fontSize: "var(--text-2xs)" }}
                    placeholder="命中词，用顿号或逗号分隔"
                    value={c.words.join("、")}
                    onChange={e => updateCategory(i, { words: e.target.value.split(/[、,，\s]+/).filter(Boolean) })}
                  />
                  <textarea
                    className="pm-textarea"
                    rows={2}
                    style={{ fontSize: "var(--text-2xs)", marginTop: 6 }}
                    placeholder="正则模式（如结构骨架/翻译腔），用换行或逗号分隔；留空表示仅用词表"
                    value={c.patterns.join("\n")}
                    onChange={e => updateCategory(i, { patterns: e.target.value.split(/[\n,，]+/).map(s => s.trim()).filter(Boolean) })}
                  />
                </div>
              ))}
            </div>
          </>
        ) : (
          <div className="empty-state-sub" style={{ padding: "12px 0" }}>加载规则配置中...</div>
        )}
      </div>
    </div>
  );
}
