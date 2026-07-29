import {
  ListTree, PenLine, Users, Globe, Workflow,
  ShieldCheck, Palette, Calendar,
  Sparkles, Play, Check, Settings,
  ChevronRight, Lightbulb,
} from "lucide-react";
import type { ProjectData, ProjectMeta, ViewType } from "../types";

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

export function ProjectDashboard({ project, projectData, onNavigate }: ProjectDashboardProps) {
  const totalChapters = projectData.volumes?.reduce((s, v) => s + v.chapters.length, 0) ?? 0;
  const totalWords = projectData.volumes?.reduce(
    (s, v) => s + v.chapters.reduce((s2, c) => s2 + c.word_count, 0), 0
  ) ?? 0;
  const totalVolumes = projectData.volumes?.length ?? 0;
  const totalCharacters = projectData.characters?.length ?? 0;
  const totalLocations = projectData.world?.locations?.length ?? 0;

  const polishedCount = projectData.volumes?.reduce(
    (s, v) => s + v.chapters.filter(c => c.status === "Polished" || c.status === "Published").length, 0
  ) ?? 0;

  const workflowId = projectData.workflow_id;
  const wfMeta = workflowId ? workflowMeta[workflowId] : null;
  const hasOutline = totalChapters > 0;
  const hasWorkflow = !!workflowId;

  const modules: Array<{
    id: ViewType;
    label: string;
    sublabel: string;
    icon: React.ReactNode;
    color: string;
    count?: string;
  }> = [
    {
      id: "concept",
      label: "灵魂萌芽",
      sublabel: "想法描述 / 创作设定 / 多维度讨论",
      icon: <Lightbulb size={20} />,
      color: "var(--color-accent)",
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
      id: "character",
      label: "人物志",
      sublabel: "角色创建 / 性格 / 关系",
      icon: <Users size={20} />,
      color: "var(--color-indigo)",
      count: `${totalCharacters} 位角色`,
    },
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
          <span>创作阶段：灵魂萌芽（种子）→ 世界观 + 人物志（铺开）→ 大纲·笔耕（骨架）</span>
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

      {/* 新手引导 */}
      {!wfMeta && (
        <div className="pd-onboard">
          <div className="pd-onboard-icon"><Sparkles size={24} /></div>
          <div className="pd-onboard-content">
            <h3>创作之旅 · 三阶段启程</h3>
            <p>建议按以下三阶段推进：</p>
            <div className="pd-steps">
              <div className="pd-step">
                <span className="pd-step-num" style={{ background: "var(--color-accent)" }}>1</span>
                <span>灵魂萌芽 — 描述想法 + 设定目标 + 多维度讨论</span>
                <button
                  className="pd-onboard-btn"
                  style={{ padding: "2px 12px", fontSize: "var(--text-xs)", marginLeft: "auto" }}
                  onClick={() => onNavigate("concept")}
                >
                  <Lightbulb size={12} /> 去萌芽
                </button>
              </div>
              <div className="pd-step">
                <span className="pd-step-num" style={{ background: hasOutline ? "var(--color-jade)" : "var(--color-indigo)" }}>
                  {hasOutline ? <Check size={12} /> : "2"}
                </span>
                <span style={{ color: hasOutline ? "var(--color-jade)" : undefined }}>
                  世界观 + 人物 — 同步铺开设定
                </span>
                <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginLeft: "auto" }}>
                  {totalCharacters} 角色 · {totalLocations} 处设定
                </span>
              </div>
              <div className="pd-step">
                <span className="pd-step-num" style={{ background: hasWorkflow ? "var(--color-jade)" : undefined }}>
                  {hasWorkflow ? <Check size={12} /> : "3"}
                </span>
                <span style={{ color: hasWorkflow ? "var(--color-jade)" : undefined }}>
                  骨架大纲 — 拉故事结构，确定起承转合
                </span>
                {!hasOutline && (
                  <button
                    className="pd-onboard-btn"
                    style={{ padding: "2px 12px", fontSize: "var(--text-xs)", marginLeft: "auto" }}
                    onClick={() => onNavigate("outline")}
                  >
                    <ListTree size={12} /> 去创建
                  </button>
                )}
                {hasOutline && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginLeft: "auto" }}>
                    {totalChapters} 章
                  </span>
                )}
              </div>
            </div>

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <button
                className="pd-onboard-btn"
                onClick={() => onNavigate(hasOutline ? "workflow" : "concept")}
              >
                {hasOutline ? <><Settings size={15} /> 配置工作流</> : <><Lightbulb size={15} /> 从灵感开始</>}
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
