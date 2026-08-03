import React, { useState } from "react";
import {
  PenLine, ListTree, Users, Globe, Palette, LayoutDashboard,
  FolderOpen, Settings, Workflow, Play, MessageSquareText,
  ChevronsLeft, ChevronsRight, ArrowLeft, Lightbulb, RefreshCw,
} from "lucide-react";
import type { ViewType, ProjectMeta } from "../types";
import { appVersion } from "../ipc";
import { UpdateDialog } from "./UpdateDialog";
import logoUrl from "../assets/logo.png";

interface SidebarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  currentProject: ProjectMeta | null;
  onExitProject: () => void;
}

// 全局导航 — 无项目上下文
const globalNav = [
  { id: "projects" as ViewType, label: "作品库", icon: <FolderOpen size={18} />, group: "works" as const },
  { id: "workflow-library" as ViewType, label: "工作流", icon: <Workflow size={18} />, group: "works" as const },
  { id: "llm-settings" as ViewType, label: "模型设置", icon: <Settings size={18} />, group: "system" as const },
  { id: "experts" as ViewType, label: "专家库", icon: <Lightbulb size={18} />, group: "system" as const },
];

// 项目空间导航 — 按创作阶段排序
const projectNav = [
  { id: "dashboard" as ViewType, label: "概览", icon: <LayoutDashboard size={18} />, group: "项目" as const },
  // 阶段一：核心概念（种子）
  { id: "concept" as ViewType, label: "灵魂萌芽", icon: <Lightbulb size={18} />, group: "创作" as const },
  // 阶段二：世界观 + 人物同步铺开
  { id: "world" as ViewType, label: "世界观", icon: <Globe size={18} />, group: "创作" as const },
  { id: "character" as ViewType, label: "人物志", icon: <Users size={18} />, group: "创作" as const },
  // 阶段三：骨架大纲 + 正文
  { id: "outline" as ViewType, label: "大纲", icon: <ListTree size={18} />, group: "创作" as const },
  { id: "writing" as ViewType, label: "笔耕", icon: <PenLine size={18} />, group: "创作" as const },
  { id: "annotations" as ViewType, label: "批注", icon: <MessageSquareText size={18} />, group: "创作" as const },
  // 自动化引擎
  { id: "harness" as ViewType, label: "造化工坊", icon: <Play size={18} />, group: "引擎" as const },
  // 辅助工具
  { id: "style" as ViewType, label: "墨韵", icon: <Palette size={18} />, group: "工具" as const },
];

const groupLabels: Record<string, string> = {
  works: "全局",
  system: "系统",
  "项目": "项目",
  "创作": "创作",
  "引擎": "引擎",
  "工具": "工具",
};

// 各分组对应的操作提示
const groupHints: Record<string, string> = {
  "创作": "概念→世界→人物→大纲→笔耕",
  "引擎": "造化工坊 + 智能体自动化",
  "工具": "墨韵（文风 + 审校 + 规则）",
};

export function Sidebar({ currentView, onViewChange, currentProject, onExitProject }: SidebarProps) {
  const [expanded, setExpanded] = useState(true);
  const [version, setVersion] = useState("");
  const [showUpdate, setShowUpdate] = useState(false);

  // 加载当前版本号（启动后静默获取，失败不影响使用）
  React.useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  const items = currentProject ? projectNav : globalNav;
  const groups = [...new Set(items.map(i => i.group))];

  return (
    <div className={`sidebar-spine ${expanded ? "" : "sidebar-collapsed"}`}>
      <div className="spine-brand">
        <img className="spine-brand-logo" src={logoUrl} alt="PenSoul" draggable={false} />
        {expanded && (
          <div className="spine-brand-text-wrap">
            <div className="spine-brand-text">PenSoul</div>
            <div className="spine-brand-sub">创意写作工坊</div>
          </div>
        )}
      </div>

      {/* 项目返回按钮 + 项目名称 */}
      {currentProject && (
        <>
          <button
            className="spine-nav-item"
            onClick={onExitProject}
            title="返回作品库"
            style={{ margin: "0 8px 4px", gap: 8, height: 32 }}
          >
            <span className="spine-nav-icon"><ArrowLeft size={16} /></span>
            {expanded && <span className="spine-nav-label">返回作品库</span>}
          </button>

          {expanded && (
            <div className="spine-project-banner">
              <div className="spine-project-icon">{currentProject.title.charAt(0)}</div>
              <div className="spine-project-info">
                <div className="spine-project-title">{currentProject.title}</div>
                <div className="spine-project-meta">
                  {currentProject.total_chapters} 章 · {currentProject.total_words.toLocaleString()} 字
                </div>
              </div>
            </div>
          )}
        </>
      )}

      <nav className="spine-nav">
        {groups.map((group) => {
          const groupItems = items.filter(i => i.group === group);
          return (
            <React.Fragment key={group}>
              {expanded && (
                <div className="spine-group-label">
                  {groupLabels[group] || group}
                  {groupHints[group] && (
                    <span className="spine-group-hint">{groupHints[group]}</span>
                  )}
                </div>
              )}
              {groupItems.map((item) => (
                <button
                  key={item.id}
                  className={`spine-nav-item ${currentView === item.id ? "active" : ""}`}
                  onClick={() => onViewChange(item.id)}
                  title={item.label}
                >
                  <span className="spine-nav-icon">{item.icon}</span>
                  {expanded && <span className="spine-nav-label">{item.label}</span>}
                </button>
              ))}
            </React.Fragment>
          );
        })}
      </nav>

      <div className="spine-collapse" style={{ position: "relative", zIndex: 2 }}>
        <button
          className="spine-nav-item"
          onClick={() => setExpanded(!expanded)}
          style={{ justifyContent: "center", gap: 0, height: 32 }}
          title={expanded ? "收起" : "展开"}
        >
          {expanded ? <ChevronsLeft size={16} /> : <ChevronsRight size={16} />}
        </button>
      </div>
      {/* 版本与更新入口 */}
      <button
        className="spine-version"
        onClick={() => setShowUpdate(true)}
        title="版本与检查更新"
        style={{ position: "relative", zIndex: 2 }}
      >
        <RefreshCw size={10} style={{ opacity: 0.7, marginRight: 5, verticalAlign: -1 }} />
        {expanded ? `v${version || "…"}` : ""}
      </button>
      <div className="spine-seal"><div className="seal-char">印</div></div>
      {showUpdate && <UpdateDialog onClose={() => setShowUpdate(false)} />}
    </div>
  );
}
