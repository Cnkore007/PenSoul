import { useMemo, useCallback, useState } from "react";
import type { ProjectData, ProjectMeta } from "../types";
import {
  Target, BookOpen, FileText, PenLine, Layers,
  TrendingUp, CheckCircle2, AlertCircle, BarChart3,
  Save,
} from "lucide-react";
import { saveSettings, loadSettings } from "../ipc";

interface CreationSettingsProps {
  project: ProjectMeta;
  projectData: ProjectData;
  persistProjectData?: (updater: (prev: ProjectData) => ProjectData) => void;
}

export function CreationSettings({ project, projectData, persistProjectData }: CreationSettingsProps) {
  void project;
  const { settings } = projectData;
  const [backendSynced, setBackendSynced] = useState<boolean | null>(null);

  // 实时统计
  const stats = useMemo(() => {
    // 细纲不计入章节数与字数：只统计已开始细写（有正文）的章节
    const written = projectData.volumes.flatMap(v => v.chapters).filter(c => (c.word_count ?? 0) > 0);
    const totalChapters = written.length;
    const totalWords = written.reduce((s, c) => s + (c.word_count ?? 0), 0);
    const totalVolumes = projectData.volumes.length;
    const avgWordsPerChapter = totalChapters > 0 ? Math.round(totalWords / totalChapters) : 0;
    const polishedCount = projectData.volumes.reduce(
      (s, v) => s + v.chapters.filter(c => c.status === "Polished" || c.status === "Published").length, 0
    );
    return { totalChapters, totalWords, totalVolumes, avgWordsPerChapter, polishedCount };
  }, [projectData]);

  // 自动计算总字数：目标章数 × 每章字数
  const autoTargetWords = useMemo(() => {
    if (settings.targetChapters > 0 && settings.chapterTargetWords > 0) {
      return settings.targetChapters * settings.chapterTargetWords;
    }
    return 0;
  }, [settings.targetChapters, settings.chapterTargetWords]);

  // 检测是否运行在 Tauri 环境
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // 同步到后端（Tauri IPC），浏览器模式回退到 localStorage
  const syncToBackend = useCallback(async () => {
    const payload = {
      target_chapters: settings.targetChapters,
      target_words: autoTargetWords,
      chapter_target_words: settings.chapterTargetWords,
      target_volumes: settings.targetVolumes,
      genre: settings.genre || "",
    };

    try {
      if (isTauri) {
        await saveSettings(payload);
      } else {
        localStorage.setItem('pensoul_engine_settings', JSON.stringify(payload));
      }
      setBackendSynced(true);
    } catch {
      setBackendSynced(false);
    }
    setTimeout(() => setBackendSynced(null), 3000);
  }, [settings, autoTargetWords, isTauri]);

  // 从后端加载（Tauri IPC），浏览器模式回退到 localStorage
  const loadFromBackend = useCallback(async () => {
    let loaded: Awaited<ReturnType<typeof loadSettings>> = null;

    if (isTauri) {
      loaded = await loadSettings();
    } else {
      // 浏览器模式：从 localStorage 读取
      try {
        const raw = localStorage.getItem('pensoul_engine_settings');
        if (raw) loaded = JSON.parse(raw);
      } catch {}
    }

    if (loaded && persistProjectData) {
      persistProjectData(prev => ({
        ...prev,
        settings: {
          targetChapters: loaded.target_chapters,
          targetWords: loaded.target_words,
          chapterTargetWords: loaded.chapter_target_words,
          targetVolumes: loaded.target_volumes,
          genre: loaded.genre,
        },
      }));
    }
  }, [persistProjectData, isTauri]);

  // 更新本地设置并自动同步到后端
  const updateSetting = useCallback((key: string, value: number | string) => {
    persistProjectData?.(prev => ({
      ...prev,
      settings: { ...prev.settings, [key]: value },
    }));
  }, [persistProjectData]);

  const goals = useMemo(() => [
    {
      key: "targetChapters",
      label: "章节进度",
      icon: <FileText size={16} />,
      current: stats.totalChapters,
      target: settings.targetChapters,
      unit: "章",
    },
    {
      key: "targetWords",
      label: "字数进度",
      icon: <PenLine size={16} />,
      current: stats.totalWords,
      target: autoTargetWords,
      unit: "字",
    },
    {
      key: "chapterTargetWords",
      label: "均章字数",
      icon: <BarChart3 size={16} />,
      current: stats.avgWordsPerChapter,
      target: settings.chapterTargetWords,
      unit: "字/章",
    },
    {
      key: "targetVolumes",
      label: "卷数进度",
      icon: <Layers size={16} />,
      current: stats.totalVolumes,
      target: settings.targetVolumes,
      unit: "卷",
    },
  ], [settings, stats, autoTargetWords]);

  return (
    <div className="pd-section" style={{ padding: "var(--space-lg) 0" }}>
      <div className="pd-section-header">
        <h2>创作设定</h2>
        <span className="pd-section-sub">设定创作目标，追踪完成进度，同步到工作流引擎</span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="btn btn-accent" onClick={syncToBackend} title="保存设定到后端，智能体工作流可读取">
            <Save size={14} />
            {isTauri ? "同步到引擎" : "保存到缓存"}
          </button>
          {isTauri && (
            <button className="btn btn-secondary" onClick={loadFromBackend}>
              从引擎加载
            </button>
          )}
          {!isTauri && (
            <span style={{
              fontSize: "var(--text-2xs)", color: "var(--color-ink-3)",
              padding: "4px 8px", fontStyle: "italic",
            }}>
              浏览器模式 · 设定存本地缓存
            </span>
          )}
          {backendSynced !== null && (
            <span style={{
              fontSize: "var(--text-xs)", padding: "4px 8px", borderRadius: "var(--radius-sm)",
              background: backendSynced ? "var(--color-jade-wash)" : "var(--color-error-wash)",
              color: backendSynced ? "var(--color-jade)" : "var(--color-error)",
            }}>
              {backendSynced
                ? (isTauri ? "已同步" : "已保存")
                : "保存失败"}
            </span>
          )}
        </div>
      </div>

      {/* 当前概览统计 */}
      <div className="pd-stats" style={{ marginBottom: "var(--space-xl)" }}>
        <div className="pd-stat">
          <div className="pd-stat-value">{stats.totalVolumes}</div>
          <div className="pd-stat-label">卷</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{stats.totalChapters}</div>
          <div className="pd-stat-label">章</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{stats.totalWords.toLocaleString()}</div>
          <div className="pd-stat-label">字</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{stats.avgWordsPerChapter.toLocaleString()}</div>
          <div className="pd-stat-label">均章字数</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{stats.polishedCount}/{stats.totalChapters}</div>
          <div className="pd-stat-label">已润色</div>
        </div>
      </div>

      {/* 目标设定区域 */}
      <div style={{
        background: "var(--color-paper)",
        border: "1px solid var(--color-rule-light)",
        borderRadius: "var(--radius-md)",
        padding: "var(--space-lg) var(--space-xl)",
        marginBottom: "var(--space-xl)",
        boxShadow: "var(--shadow-subtle)",
      }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)",
          borderBottom: "1px solid var(--color-rule-light)",
        }}>
          <Target size={18} style={{ color: "var(--color-accent)" }} />
          <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
            创作目标设定
          </span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px", marginLeft: "auto" }}>
            数值为 0 表示未设定目标
          </span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: "var(--space-md)" }}>
          {/* 目标总章数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <FileText size={13} /> 目标总章数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder="例：100"
              value={settings.targetChapters || ""}
              onChange={(e) => updateSetting("targetChapters", parseInt(e.target.value) || 0)}
            />
          </div>

          {/* 每章目标字数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <BarChart3 size={13} /> 每章目标字数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder="例：3000"
              value={settings.chapterTargetWords || ""}
              onChange={(e) => updateSetting("chapterTargetWords", parseInt(e.target.value) || 0)}
            />
          </div>

          {/* 自动计算总字数（只读） */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <PenLine size={13} /> 预计总字数（自动计算）
            </label>
            <div style={{
              padding: "8px 12px", fontSize: "var(--text-sm)",
              background: "var(--color-paper-warm)", color: "var(--color-ink-2)",
              border: "1px solid var(--color-rule)", borderRadius: "var(--radius-sm)",
              minHeight: 38, display: "flex", alignItems: "center",
            }}>
              {settings.targetChapters > 0 && settings.chapterTargetWords > 0 ? (
                <span style={{ fontWeight: 600 }}>
                  {autoTargetWords.toLocaleString()} 字
                  <span style={{ fontWeight: 400, color: "var(--color-ink-3)", marginLeft: 6, fontSize: "var(--text-xs)" }}>
                    （{settings.targetChapters} 章 × {settings.chapterTargetWords.toLocaleString()} 字）
                  </span>
                </span>
              ) : (
                <span style={{ color: "var(--color-ink-faint)", fontStyle: "italic" }}>
                  输入目标章数和每章字数后自动计算
                </span>
              )}
            </div>
          </div>

          {/* 预计卷数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <Layers size={13} /> 预计卷数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder="例：5"
              value={settings.targetVolumes || ""}
              onChange={(e) => updateSetting("targetVolumes", parseInt(e.target.value) || 0)}
            />
          </div>

          {/* 故事类型 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <BookOpen size={13} /> 故事类型
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="text" placeholder="例：玄幻、言情、科幻"
              value={settings.genre || ""}
              onChange={(e) => updateSetting("genre", e.target.value)}
            />
          </div>
        </div>
      </div>

      {/* 进度追踪卡片 */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-xs)" }}>
          <TrendingUp size={18} style={{ color: "var(--color-jade)" }} />
          <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
            目标进度追踪
          </span>
        </div>

        {goals.map(g => {
          const hasTarget = g.target > 0;
          const pct = hasTarget ? Math.min(100, Math.round((g.current / g.target) * 100)) : 0;
          const isCompleted = hasTarget && g.current >= g.target;

          return (
            <div key={g.key} style={{
              background: "var(--color-paper)",
              border: `1px solid ${isCompleted ? "var(--color-jade)" : "var(--color-rule-light)"}`,
              borderRadius: "var(--radius-md)", padding: "var(--space-md) var(--space-lg)",
              boxShadow: "var(--shadow-subtle)", transition: "border-color var(--dur-short) var(--ease-out)",
            }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "var(--space-sm)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{
                    width: 32, height: 32, borderRadius: "var(--radius-sm)",
                    background: isCompleted ? "var(--color-jade-wash)" : "var(--color-paper-warm)",
                    display: "flex", alignItems: "center", justifyContent: "center",
                    color: isCompleted ? "var(--color-jade)" : "var(--color-ink-3)", flexShrink: 0,
                  }}>
                    {isCompleted ? <CheckCircle2 size={16} /> : g.icon}
                  </span>
                  <div>
                    <div style={{ fontSize: "var(--text-sm)", fontWeight: 500, color: "var(--color-ink)", letterSpacing: "0.5px" }}>
                      {g.label}
                    </div>
                    <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", letterSpacing: "0.3px" }}>
                      {isCompleted ? "目标已完成" : hasTarget ? "进行中" : "未设定目标"}
                    </div>
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: "var(--text-lg)", fontWeight: 600, color: isCompleted ? "var(--color-jade)" : "var(--color-ink)", lineHeight: 1.2 }}>
                    {g.current.toLocaleString()}
                    {hasTarget && <span style={{ fontSize: "var(--text-xs)", fontWeight: 400, color: "var(--color-ink-3)", marginLeft: 4 }}>
                      / {g.target.toLocaleString()}
                    </span>}
                  </div>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", letterSpacing: "0.3px" }}>
                    {g.unit}
                  </div>
                </div>
              </div>

              {hasTarget && (
                <div>
                  <div style={{ width: "100%", height: 6, background: "var(--color-rule-light)", borderRadius: 3, overflow: "hidden" }}>
                    <div style={{
                      width: `${pct}%`, height: "100%",
                      background: isCompleted ? "var(--color-jade)" : "linear-gradient(90deg, var(--color-accent-soft), var(--color-accent))",
                      borderRadius: 3, transition: "width 0.4s var(--ease-out)",
                    }} />
                  </div>
                  <div style={{ display: "flex", justifyContent: "space-between", marginTop: 4 }}>
                    <span style={{ fontSize: "var(--text-2xs)", color: isCompleted ? "var(--color-jade)" : "var(--color-accent)", fontWeight: 500 }}>
                      {pct}%
                    </span>
                    <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
                      {isCompleted ? "已达标" : `剩余 ${(g.target - g.current).toLocaleString()} ${g.unit}`}
                    </span>
                  </div>
                </div>
              )}

              {!hasTarget && (
                <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", fontStyle: "italic", padding: "4px 0", letterSpacing: "0.3px" }}>
                  在上方设定目标后自动追踪进度
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* 整体概览 */}
      {goals.some(g => g.target > 0) && (
        <div style={{
          marginTop: "var(--space-xl)", padding: "var(--space-md) var(--space-lg)",
          background: "var(--color-paper-warm)", border: "1px solid var(--color-rule-light)",
          borderRadius: "var(--radius-md)", display: "flex", alignItems: "center", gap: "var(--space-md)",
        }}>
          <div style={{ width: 44, height: 44, borderRadius: "var(--radius-sm)", background: "var(--color-accent-wash)", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
            <AlertCircle size={22} style={{ color: "var(--color-accent)" }} />
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--color-ink)", fontWeight: 500, letterSpacing: "0.5px" }}>
              创作进度概览
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 2, letterSpacing: "0.3px" }}>
              {settings.targetChapters > 0 && `${stats.totalChapters}/${settings.targetChapters} 章 · `}
              {autoTargetWords > 0 && `${stats.totalWords.toLocaleString()}/${autoTargetWords.toLocaleString()} 字 · `}
              {settings.genre && `类型: ${settings.genre}`}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
