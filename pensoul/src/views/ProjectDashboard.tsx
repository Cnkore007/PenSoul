import {
  ListTree, PenLine, Users, Globe, Workflow,
  ShieldCheck, Palette, Calendar,
  Sparkles, Play, Check, Settings, Edit3,
  ChevronRight,
} from "lucide-react";
import type { ProjectData, ProjectMeta, ViewType } from "../types";

// 工作流模板元数据（与 WorkflowView 保持一致）
const workflowMeta: Record<string, { name: string; stageCount: number }> = {
  "standard-novel": { name: "标准小说工作流", stageCount: 5 },
  "quick-novel": { name: "快速创作工作流", stageCount: 3 },
};

interface ProjectDashboardProps {
  project: ProjectMeta;
  projectData: ProjectData;
  onNavigate: (view: ViewType) => void;
  persistProjectData?: (updater: (prev: ProjectData) => ProjectData) => void;
}

export function ProjectDashboard({ project, projectData, onNavigate, persistProjectData }: ProjectDashboardProps) {
  // 统计
  const totalChapters = projectData.volumes.reduce((s, v) => s + v.chapters.length, 0);
  const totalWords = projectData.volumes.reduce(
    (s, v) => s + v.chapters.reduce((s2, c) => s2 + c.word_count, 0), 0
  );
  const totalVolumes = projectData.volumes.length;
  const totalCharacters = projectData.characters.length;
  const totalLocations = projectData.world.locations.length;

  // 各状态章节数
  const polishedCount = projectData.volumes.reduce(
    (s, v) => s + v.chapters.filter(c => c.status === "Polished" || c.status === "Published").length, 0
  );

  // 工作流状态
  const workflowId = projectData.workflow_id;
  const wfMeta = workflowId ? workflowMeta[workflowId] : null;

  // 新手引导步骤状态
  const hasOutline = totalChapters > 0;
  const hasWorkflow = !!workflowId;

  // 模块快捷入口
  const modules: Array<{
    id: ViewType;
    label: string;
    sublabel: string;
    icon: React.ReactNode;
    color: string;
    count?: string;
  }> = [
    {
      id: "outline",
      label: "大纲",
      sublabel: "卷·章结构 / 创建章节",
      icon: <ListTree size={20} />,
      color: "var(--color-ink)",
      count: `${totalVolumes} 卷 · ${totalChapters} 章`,
    },
    {
      id: "writing",
      label: "笔耕",
      sublabel: "正文 / 章节内容编辑",
      icon: <PenLine size={20} />,
      color: "var(--color-accent)",
      count: `${totalWords.toLocaleString()} 字`,
    },
    {
      id: "character",
      label: "人物志",
      sublabel: "角色创建 / 性格 / 关系",
      icon: <Users size={20} />,
      color: "var(--color-indigo)",
      count: `${totalCharacters} 位角色`,
    },
    {
      id: "world",
      label: "世界观",
      sublabel: "地点 · 时间线 · 设定规则",
      icon: <Globe size={20} />,
      color: "var(--color-jade)",
      count: `${totalLocations} 处设定`,
    },
    {
      id: "workflow",
      label: "工作流",
      sublabel: "选择模板 → 设置阶段",
      icon: <Workflow size={20} />,
      color: "var(--color-ochre)",
      count: wfMeta ? wfMeta.name : "未配置",
    },
    {
      id: "harness",
      label: "造化工坊",
      sublabel: "一键执行 → 自动写稿",
      icon: <Play size={20} />,
      color: "var(--color-accent)",
      count: hasWorkflow ? "可启动" : "需配置",
    },
    {
      id: "consistency",
      label: "审校",
      sublabel: "一致性检查",
      icon: <ShieldCheck size={20} />,
      color: "var(--color-jade)",
    },
    {
      id: "style",
      label: "墨韵",
      sublabel: "文风分析诊断",
      icon: <Palette size={20} />,
      color: "var(--color-ochre)",
    },
  ];

  return (
    <div className="project-dashboard">
      {/* 项目头部 */}
      <div className="pd-hero">
        <div className="pd-hero-content">
          <div className="pd-hero-initial">{project.title.charAt(0)}</div>
          <div className="pd-hero-info">
            <h1 className="pd-hero-title">{project.title}</h1>
            {project.description && (
              <p className="pd-hero-desc">{project.description}</p>
            )}
          </div>
        </div>
        <div className="pd-hero-meta">
          <div className="pd-meta-item"><Calendar size={13} /><span>创建 {project.created_at}</span></div>
          <div className="pd-meta-item"><Calendar size={13} /><span>更新 {project.updated_at}</span></div>
        </div>
      </div>

      {/* 项目空间提示 */}
      <div style={{
        display: "flex", gap: "var(--space-sm)", flexWrap: "wrap",
        marginBottom: "var(--space-md)", fontSize: "var(--text-2xs)",
        color: "var(--color-ink-3)", letterSpacing: "0.5px",
      }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 4,
          padding: "6px 12px", background: "var(--color-paper-warm)",
          borderRadius: "var(--radius-sm)", border: "1px solid var(--color-rule-light)",
        }}>
          <span style={{
            width: 6, height: 6, borderRadius: "50%",
            background: "var(--color-accent)", display: "inline-block",
          }} />
          <span>创作编辑：大纲（卷章结构）· 笔耕（正文）· 人物志（角色）· 世界观（设定）</span>
        </div>
        <div style={{
          display: "flex", alignItems: "center", gap: 4,
          padding: "6px 12px", background: "var(--color-paper-warm)",
          borderRadius: "var(--radius-sm)", border: "1px solid var(--color-rule-light)",
        }}>
          <span style={{
            width: 6, height: 6, borderRadius: "50%",
            background: "var(--color-ochre)", display: "inline-block",
          }} />
          <span>自动化引擎：工作流（配置模板）→ 造化工坊（Agent 自动写作）</span>
        </div>
      </div>

      {/* 统计概览 */}
      <div className="pd-stats">
        <div className="pd-stat">
          <div className="pd-stat-value">{totalVolumes}</div>
          <div className="pd-stat-label">卷</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{totalChapters}</div>
          <div className="pd-stat-label">章</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{totalWords.toLocaleString()}</div>
          <div className="pd-stat-label">字</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{totalCharacters}</div>
          <div className="pd-stat-label">角色</div>
        </div>
        <div className="pd-stat-sep" />
        <div className="pd-stat">
          <div className="pd-stat-value">{polishedCount}/{totalChapters}</div>
          <div className="pd-stat-label">已润色</div>
        </div>
      </div>

      {/* 创作规划 */}
      <div className="pd-section" style={{ marginBottom: "var(--space-lg)" }}>
        <div className="pd-section-header">
          <h2>创作规划</h2>
          <span className="pd-section-sub">设定创作目标，追踪进展</span>
        </div>
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
          gap: "var(--space-sm)",
        }}>
          {[
            { label: "目标总章数", key: "targetChapters", value: projectData.settings.targetChapters, current: totalChapters, unit: "章" },
            { label: "目标总字数", key: "targetWords", value: projectData.settings.targetWords, current: totalWords, unit: "字" },
            { label: "每章目标字数", key: "chapterTargetWords", value: projectData.settings.chapterTargetWords, current: totalChapters > 0 ? Math.round(totalWords / totalChapters) : 0, unit: "字/章" },
            { label: "预计卷数", key: "targetVolumes", value: projectData.settings.targetVolumes, current: totalVolumes, unit: "卷" },
          ].map(item => {
            const pct = item.value > 0 ? Math.min(100, Math.round((item.current / item.value) * 100)) : 0;
            return (
              <div key={item.key} style={{
                background: "var(--color-bg-soft)",
                borderRadius: "var(--radius-md)",
                padding: "var(--space-sm) var(--space-md)",
              }}>
                <div style={{
                  fontSize: "var(--text-xs)",
                  color: "var(--color-ink-3)",
                  marginBottom: 4,
                }}>
                  {item.label}
                </div>
                <div style={{
                  display: "flex",
                  alignItems: "baseline",
                  gap: 6,
                  marginBottom: 6,
                }}>
                  <span style={{
                    fontSize: "var(--text-lg)",
                    fontWeight: 600,
                    color: "var(--color-ink)",
                  }}>
                    {item.current.toLocaleString()}
                  </span>
                  {item.value > 0 && (
                    <>
                      <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-faint)" }}>
                        / {item.value.toLocaleString()} {item.unit}
                      </span>
                      <span style={{
                        fontSize: "var(--text-xs)",
                        fontWeight: 500,
                        color: pct >= 100 ? "var(--color-jade)" : "var(--color-ochre)",
                        marginLeft: "auto",
                      }}>
                        {pct}%
                      </span>
                    </>
                  )}
                  {item.value === 0 && (
                    <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-faint)" }}>
                      未设定
                    </span>
                  )}
                </div>
                {item.value > 0 && (
                  <div style={{
                    width: "100%",
                    height: 4,
                    background: "var(--color-border)",
                    borderRadius: 2,
                    overflow: "hidden",
                  }}>
                    <div style={{
                      width: `${pct}%`,
                      height: "100%",
                      background: pct >= 100 ? "var(--color-jade)" : "var(--color-accent)",
                      borderRadius: 2,
                      transition: "width 0.3s",
                    }} />
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {/* 规划设置 — 直接展示编辑区域 */}
        <div style={{
          marginTop: "var(--space-sm)",
          padding: "var(--space-sm) var(--space-md)",
          background: "var(--color-bg-soft)",
          borderRadius: "var(--radius-md)",
          border: "1px solid var(--color-border)",
        }}>
          <div style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            marginBottom: "var(--space-sm)",
          }}>
            <Edit3 size={14} style={{ color: "var(--color-ink-3)" }} />
            <span style={{
              fontSize: "var(--text-xs)",
              fontWeight: 600,
              color: "var(--color-ink-2)",
              letterSpacing: "0.5px",
            }}>
              创作规划目标
            </span>
            <span style={{
              fontSize: "var(--text-2xs)",
              color: "var(--color-ink-faint)",
              marginLeft: 8,
            }}>
              设定后可追踪完成进度
            </span>
          </div>
          <div style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
            gap: "var(--space-sm)",
          }}>
            {([
              ["targetChapters", "目标总章数", "0"],
              ["targetWords", "目标总字数", "0"],
              ["chapterTargetWords", "每章目标字数", "0"],
              ["targetVolumes", "预计卷数", "0"],
              ["genre", "故事类型", "例：玄幻、言情、科幻"],
            ] as const).map(([key, label, placeholder]) => (
              <div key={key} style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{label}</label>
                <input
                  className="pm-input"
                  style={{ marginBottom: 0, fontSize: "var(--text-xs)", padding: "4px 8px" }}
                  type={key === "genre" ? "text" : "number"}
                  min={key === "genre" ? undefined : "0"}
                  placeholder={placeholder}
                  value={(projectData.settings as any)[key] ?? ""}
                  onChange={(e) => {
                    const val = key === "genre" ? e.target.value : (parseInt(e.target.value) || 0);
                    persistProjectData?.(prev => ({
                      ...prev,
                      settings: { ...prev.settings, [key]: val },
                    }));
                  }}
                />
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* 创作空间 — 模块入口 */}
      <div className="pd-section">
        <div className="pd-section-header">
          <h2>创作空间</h2>
          <span className="pd-section-sub">选择模块开始创作</span>
        </div>
        <div className="pd-modules">
          {modules.map(m => (
            <button
              key={m.id}
              className="pd-module"
              onClick={() => onNavigate(m.id)}
            >
              <div className="pd-module-icon" style={{ color: m.color }}>{m.icon}</div>
              <div className="pd-module-info">
                <div className="pd-module-label">{m.label}</div>
                <div className="pd-module-sublabel">{m.sublabel}</div>
              </div>
              {m.count && <div className="pd-module-count">{m.count}</div>}
              <ChevronRight size={14} className="pd-module-arrow" />
            </button>
          ))}
        </div>
      </div>

      {/* 工作流快速状态 */}
      {wfMeta && (
        <div className="pd-section">
          <div className="pd-section-header">
            <h2>工作流状态</h2>
            <button className="pd-link-btn" onClick={() => onNavigate("harness")}>前往造化工坊</button>
          </div>
          <div className="pd-workflow-card">
            <div className="pd-wf-info">
              <Play size={16} style={{ color: "var(--color-accent)" }} />
              <span className="pd-wf-name">{wfMeta.name}</span>
              <span className="pd-wf-stages">{wfMeta.stageCount} 个阶段</span>
            </div>
            <p className="pd-wf-desc">
              Agent 已就绪。前往「造化工坊」一键启动自动创作流程。
            </p>
            <button
              className="pd-onboard-btn"
              style={{ marginTop: "var(--space-sm)" }}
              onClick={() => onNavigate("harness")}
            >
              <Play size={15} /> 启动造化工坊
            </button>
          </div>
        </div>
      )}

      {/* 新手引导 — 未配置工作流时 */}
      {!wfMeta && (
        <div className="pd-onboard">
          <div className="pd-onboard-icon"><Sparkles size={24} /></div>
          <div className="pd-onboard-content">
            <h3>创作之旅 · 三步启程</h3>
            <p>建议按以下顺序完成初始设置：</p>
            <div className="pd-steps">
              <div className="pd-step">
                <span className="pd-step-num" style={{ background: hasOutline ? "var(--color-jade)" : "var(--color-accent)" }}>
                  {hasOutline ? <Check size={12} /> : "1"}
                </span>
                <span style={{ color: hasOutline ? "var(--color-jade)" : undefined }}>
                  创建卷和章节
                </span>
                {hasOutline && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
                    ({totalChapters} 章)
                  </span>
                )}
                {!hasOutline && (
                  <button
                    className="pd-onboard-btn"
                    style={{ padding: "2px 12px", fontSize: "var(--text-xs)", marginLeft: "auto" }}
                    onClick={() => onNavigate("outline")}
                  >
                    <ListTree size={12} /> 去创建
                  </button>
                )}
              </div>
              <div className="pd-step">
                <span className="pd-step-num" style={{ background: hasWorkflow ? "var(--color-jade)" : undefined }}>
                  {hasWorkflow ? <Check size={12} /> : "2"}
                </span>
                <span style={{ color: hasWorkflow ? "var(--color-jade)" : undefined }}>
                  配置工作流模板
                </span>
                {hasWorkflow && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
                    (已配置)
                  </span>
                )}
                {!hasWorkflow && (
                  <button
                    className="pd-onboard-btn"
                    style={{ padding: "2px 12px", fontSize: "var(--text-xs)", marginLeft: "auto" }}
                    onClick={() => onNavigate("workflow")}
                  >
                    <Settings size={12} /> 去配置
                  </button>
                )}
              </div>
              <div className="pd-step">
                <span className="pd-step-num">3</span>
                <span>启动 Agent 自动写作</span>
                {hasWorkflow && (
                  <button
                    className="pd-onboard-btn"
                    style={{ padding: "2px 12px", fontSize: "var(--text-xs)", marginLeft: "auto" }}
                    onClick={() => onNavigate("harness")}
                  >
                    <Play size={12} /> 去启动
                  </button>
                )}
                {!hasWorkflow && !hasOutline && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginLeft: "auto" }}>
                    先完成前两步
                  </span>
                )}
              </div>
            </div>

            {/* 快速入口 */}
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <button
                className="pd-onboard-btn"
                onClick={() => onNavigate(hasOutline ? "workflow" : "outline")}
              >
                {hasOutline ? <><Settings size={15} /> 配置工作流</> : <><ListTree size={15} /> 从大纲开始</>}
              </button>
              {hasOutline && (
                <>
                  <button
                    className="pd-onboard-btn"
                    style={{ background: "transparent", color: "var(--color-ink-2)", border: "1px solid var(--color-rule)" }}
                    onClick={() => onNavigate("writing")}
                  >
                    <PenLine size={15} /> 手动创作
                  </button>
                  <button
                    className="pd-onboard-btn"
                    style={{ background: "transparent", color: "var(--color-ink-2)", border: "1px solid var(--color-rule)" }}
                    onClick={() => onNavigate("harness")}
                  >
                    <Play size={15} /> 去造化工坊
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
