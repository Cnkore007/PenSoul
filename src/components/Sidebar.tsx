import {
  BookOpen,
  LayoutDashboard,
  Sprout,
  Network,
  ShieldCheck,
  PenTool,
  ListTree,
  Settings,
  ChevronLeft,
} from "lucide-react";
import type { ViewType } from "../types";

interface SidebarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  currentProjectName: string | null;
  onProjectClose: () => void;
}

// 全局菜单项（始终显示）
const globalItems: { view: ViewType; icon: typeof BookOpen; label: string }[] = [
  { view: "projects", icon: BookOpen, label: "作品库" },
];

// 项目级菜单项（打开项目后显示）
const projectItems: { view: ViewType; icon: typeof LayoutDashboard; label: string }[] = [
  { view: "dashboard", icon: LayoutDashboard, label: "仪表盘" },
  { view: "concept", icon: Sprout, label: "萌芽" },
  { view: "entity-graph", icon: Network, label: "图谱" },
  { view: "outline", icon: ListTree, label: "大纲" },
  { view: "writing", icon: PenTool, label: "笔耕" },
  { view: "constraint", icon: ShieldCheck, label: "约束" },
];

// 底部菜单项（始终显示）
const bottomItems: { view: ViewType; icon: typeof Settings; label: string }[] = [
  { view: "settings", icon: Settings, label: "设定" },
];

export default function Sidebar({
  currentView,
  onViewChange,
  currentProjectName,
  onProjectClose,
}: SidebarProps) {
  const handleGlobalClick = (view: ViewType) => {
    if (view === "projects") {
      onProjectClose();
    }
    onViewChange(view);
  };

  return (
    <aside className="sidebar">
      {/* 品牌区 */}
      <div className="sidebar-brand">
        <div className="sidebar-logo">墨</div>
        <span className="sidebar-title">PenSoul</span>
      </div>

      <nav className="sidebar-nav">
        {/* 全局菜单 */}
        {globalItems.map(({ view, icon: Icon, label }) => (
          <button
            key={view}
            className={`nav-item ${currentView === view && !currentProjectName ? "active" : ""}`}
            onClick={() => handleGlobalClick(view)}
          >
            <Icon size={16} />
            <span className="nav-label">{label}</span>
          </button>
        ))}

        {/* 项目级菜单（仅在打开项目后显示） */}
        {currentProjectName && (
          <>
            <div className="nav-divider" />

            {/* 返回按钮 */}
            <button
              className="nav-item nav-project-back"
              onClick={onProjectClose}
            >
              <ChevronLeft size={16} />
              <span className="nav-label">返回作品库</span>
            </button>

            {/* 项目名 */}
            <div className="nav-project-name">{currentProjectName}</div>

            {/* 项目功能菜单 */}
            {projectItems.map(({ view, icon: Icon, label }) => (
              <button
                key={view}
                className={`nav-item ${currentView === view ? "active" : ""}`}
                onClick={() => onViewChange(view)}
              >
                <Icon size={16} />
                <span className="nav-label">{label}</span>
              </button>
            ))}
          </>
        )}
      </nav>

      {/* 底部菜单 */}
      <div className="sidebar-bottom">
        <div className="nav-divider" />
        {bottomItems.map(({ view, icon: Icon, label }) => (
          <button
            key={view}
            className={`nav-item ${currentView === view ? "active" : ""}`}
            onClick={() => onViewChange(view)}
          >
            <Icon size={16} />
            <span className="nav-label">{label}</span>
          </button>
        ))}
      </div>
    </aside>
  );
}
