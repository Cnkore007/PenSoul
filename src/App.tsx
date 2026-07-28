import { useState, useCallback } from "react";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { WritingView } from "./views/WritingView";
import { OutlineView } from "./views/OutlineView";
import { CharacterView } from "./views/CharacterView";
import { WorldView } from "./views/WorldView";
import { ConsistencyView } from "./views/ConsistencyView";
import { HarnessConsole } from "./views/HarnessConsole";
import { StyleWorkshop } from "./views/StyleWorkshop";
import { ProjectManager } from "./views/ProjectManager";
import LlmSettingsView from "./views/LlmSettingsView";
import { PluginView } from "./views/PluginView";
import { WorkflowView } from "./views/WorkflowView";
import { ProjectDashboard } from "./views/ProjectDashboard";
import type { ViewType, ProjectMeta, ProjectData } from "./types";
import { loadProjectData, saveProjectData } from "./store";
import "./tokens.css";
import "./App.css";

// 诊断辅助：将错误渲染到页面可见位置
function captureError(err: any) {
  const el = document.getElementById('pensoul-error');
  if (el) {
    el.style.display = 'block';
    el.textContent = typeof err === 'string' ? err : (err?.message || String(err)) + '\n' + (err?.stack || '');
  }
}
if (typeof window !== 'undefined') {
  window.addEventListener('error', (e) => {
    captureError(e.error || e.message);
  });
  window.addEventListener('unhandledrejection', (e) => {
    captureError(e.reason);
  });
}

// 错误边界 — 捕获子组件渲染异常，避免白屏
import React from "react";

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }
  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          padding: "40px",
          background: "#fef2f2",
          color: "#991b1b",
          fontFamily: "monospace",
          fontSize: "14px",
          lineHeight: 1.6,
        }}>
          <h2 style={{ margin: "0 0 12px" }}>渲染错误</h2>
          <pre style={{ whiteSpace: "pre-wrap", margin: 0 }}>
            {this.state.error?.message}
            {"\n"}
            {this.state.error?.stack}
          </pre>
        </div>
      );
    }
    return this.props.children;
  }
}

function App() {
  const [currentView, setCurrentView] = useState<ViewType>("projects");
  const [currentProject, setCurrentProject] = useState<ProjectMeta | null>(null);
  const [projectData, setProjectData] = useState<ProjectData | null>(null);
  const [currentChapterId, setCurrentChapterId] = useState<string | null>(null);
  const [wordCount, setWordCount] = useState(0);
  const [connected] = useState(true);
  
  // 全局错误捕获 — 将错误显示在页面底部
  const [appError, setAppError] = useState<string | null>(null);
  React.useEffect(() => {
    const handler = (event: ErrorEvent) => {
      setAppError(event.message + "\n" + (event.error?.stack || ""));
    };
    window.addEventListener('error', handler);
    return () => window.removeEventListener('error', handler);
  }, []);

  // 进入项目空间，默认跳转到 dashboard（项目概览）
  const handleSelectProject = useCallback((project: ProjectMeta) => {
    setCurrentProject(project);
    const data = loadProjectData(project.project_id);
    setProjectData(data);
    setCurrentView("dashboard");
  }, []);

  const handleExitProject = useCallback(() => {
    setCurrentProject(null);
    setProjectData(null);
    setCurrentChapterId(null);
    setCurrentView("projects");
  }, []);

  // 当在项目空间内删除了当前项目时，自动退回作品库
  const handleProjectDeleted = useCallback((deletedProjectId: string) => {
    if (currentProject && currentProject.project_id === deletedProjectId) {
      setCurrentProject(null);
      setProjectData(null);
      setCurrentChapterId(null);
      setCurrentView("projects");
    }
  }, [currentProject]);

  const handleSelectChapter = useCallback((chapterId: string) => {
    setCurrentChapterId(chapterId);
    setCurrentView("writing");
  }, []);

  const handleWordCountChange = useCallback((count: number) => {
    setWordCount(count);
  }, []);

  // 保存项目数据到 localStorage
  const persistProjectData = useCallback((updater: (prev: ProjectData) => ProjectData) => {
    setProjectData(prev => {
      if (!prev) return prev;
      const updated = updater(prev);
      saveProjectData(updated);
      return updated;
    });
  }, []);

  const renderView = () => {
    // 全局页面 — 无项目上下文
    if (currentView === "projects") {
      return <ProjectManager onSelectProject={handleSelectProject} currentProjectId={currentProject?.project_id ?? null} onDeleteProject={handleProjectDeleted} />;
    }
    if (currentView === "llm-settings") {
      return <LlmSettingsView />;
    }
    if (currentView === "plugins") {
      return <PluginView />;
    }

    // 项目空间页面 — 需要项目上下文
    if (!currentProject || !projectData) {
      return <ProjectManager onSelectProject={handleSelectProject} currentProjectId={currentProject?.project_id ?? null} onDeleteProject={handleProjectDeleted} />;
    }

    switch (currentView) {
      case "dashboard":
        return <ProjectDashboard project={currentProject} projectData={projectData} onNavigate={setCurrentView} persistProjectData={persistProjectData} />;
      case "outline":
        return <OutlineView projectData={projectData} persistProjectData={persistProjectData} onSelectChapter={handleSelectChapter} currentChapterId={currentChapterId} />;
      case "writing":
        return <WritingView projectData={projectData} persistProjectData={persistProjectData} chapterId={currentChapterId} onWordCountChange={handleWordCountChange} />;
      case "character":
        return <CharacterView projectData={projectData} persistProjectData={persistProjectData} />;
      case "world":
        return <WorldView projectData={projectData} persistProjectData={persistProjectData} />;
      case "workflow":
        return <WorkflowView projectData={projectData} persistProjectData={persistProjectData} onNavigate={setCurrentView} />;
      case "consistency":
        return <ConsistencyView />;
      case "harness":
        return <HarnessConsole projectData={projectData} onNavigate={setCurrentView} />;
      case "style":
        return <StyleWorkshop />;
      default:
        return <ProjectDashboard project={currentProject} projectData={projectData} onNavigate={setCurrentView} />;
    }
  };

  return (
    <div className="app-container">
      <Sidebar
        currentView={currentView}
        onViewChange={setCurrentView}
        currentProject={currentProject}
        onExitProject={handleExitProject}
      />
      <div className="main-area">
        <div className="content-area">
          <ErrorBoundary key={currentView}>{renderView()}</ErrorBoundary>
        </div>
        {appError && (
          <div style={{
            padding: "8px 16px",
            background: "#fef2f2",
            color: "#991b1b",
            fontFamily: "monospace",
            fontSize: "12px",
            whiteSpace: "pre-wrap",
            borderTop: "1px solid #fecaca",
            maxHeight: "120px",
            overflow: "auto",
          }}>
            <strong>错误:</strong> {appError}
          </div>
        )}
        <StatusBar currentView={currentView} wordCount={wordCount} connected={connected} />
      </div>
    </div>
  );
}

export default App;
