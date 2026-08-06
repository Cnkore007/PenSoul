import { useState, useCallback, useEffect, useRef } from "react";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { WritingView } from "./views/WritingView";
import { OutlineView } from "./views/OutlineView";
import { CharacterView } from "./views/CharacterView";
import { WorldView } from "./views/WorldView";
import { HarnessConsole } from "./views/HarnessConsole";
import { StyleWorkshop } from "./views/StyleWorkshop";
import { ProjectManager } from "./views/ProjectManager";
import LlmSettingsView from "./views/LlmSettingsView";
import { WorkflowLibraryView } from "./views/WorkflowLibraryView";
import { ConceptView } from "./views/ConceptView";
import { ExpertLibraryView } from "./views/ExpertLibraryView";
import { ProjectDashboard } from "./views/ProjectDashboard";
import { AnnotationInbox } from "./views/AnnotationInbox";
import { BlueprintView } from "./views/BlueprintView";
import type { ViewType, ProjectMeta, ProjectData } from "./types";
import { loadProjectData, refreshProjectData, saveProjectData } from "./store";
import { getHarnessStatus } from "./ipc";
import { messageDialog } from "./dialogs";
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
  const [connected, setConnected] = useState(false);

  // 全局错误捕获 — 将错误显示在页面底部
  const [appError, setAppError] = useState<string | null>(null);
  React.useEffect(() => {
    const handler = (event: ErrorEvent) => {
      setAppError(event.message + "\n" + (event.error?.stack || ""));
    };
    window.addEventListener('error', handler);
    return () => window.removeEventListener('error', handler);
  }, []);

  // 检测后端连接状态
  useEffect(() => {
    let cancelled = false;
    async function checkConnection() {
      try {
        await getHarnessStatus();
        if (!cancelled) setConnected(true);
      } catch {
        if (!cancelled) setConnected(false);
      }
    }
    checkConnection();
    const interval = setInterval(checkConnection, 15000);
    return () => { cancelled = true; clearInterval(interval); };
  }, []);

  // 当选中项目后，异步加载项目数据
  useEffect(() => {
    if (!currentProject) {
      setProjectData(null);
      return;
    }
    let cancelled = false;
    loadProjectData(currentProject.project_id).then(data => {
      if (!cancelled) setProjectData(data);
    }).catch(err => {
      console.error("加载项目数据失败:", err);
      if (!cancelled) setProjectData(null);
    });
    return () => { cancelled = true; };
  }, [currentProject]);

  // 轻量刷新项目数据（不调 open_project，避免重建引擎打断管线）。
  // 后台任务（造化工坊写作落库、讨论结果持久化、细纲展开建章）会直接改后端本体，
  // 前端 projectData 是内存副本，需要主动刷新才能看到。
  const refreshNow = useCallback(async () => {
    if (!currentProject) return;
    try {
      // 等上一页面触发的保存在途完成，避免读到保存前的旧数据
      await pendingSaveRef.current?.catch(() => {});
      const data = await refreshProjectData(currentProject.project_id);
      setProjectData(data);
    } catch (e) {
      console.error("刷新项目数据失败:", e);
    }
  }, [currentProject]);

  // 项目内页面切换时自动刷新一次（全局页面无项目上下文，跳过）
  useEffect(() => {
    if (!currentProject) return;
    if (["projects", "llm-settings", "experts", "workflow-library"].includes(currentView)) return;
    refreshNow();
  }, [currentView, currentProject, refreshNow]);

  // 进入项目空间，默认跳转到 dashboard（概览）
  const handleSelectProject = useCallback((project: ProjectMeta) => {
    setCurrentProject(project);
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

  const handleWordCountChange = useCallback((count: number) => {
    setWordCount(count);
  }, []);

  // 保存失败提醒只弹一次，避免连续输入时反复打扰
  const saveErrorShownRef = useRef(false);
  // 记录在途保存：页面切换刷新前先等它完成，避免读到保存前的旧数据
  const pendingSaveRef = useRef<Promise<void> | null>(null);

  // 保存项目数据 — 先更新本地状态，再异步持久化到后端
  const persistProjectData = useCallback((updater: (prev: ProjectData) => ProjectData) => {
    setProjectData(prev => {
      if (!prev) return prev;
      const updated = updater(prev);
      // 异步保存到后端，不阻塞 UI；记录 Promise 供切换页面前 await
      const p = saveProjectData(updated).catch(err => {
        console.error("保存项目数据失败:", err);
        if (!saveErrorShownRef.current) {
          saveErrorShownRef.current = true;
          void messageDialog("部分数据保存失败，重启后可能丢失：\n" + (err?.message ?? err));
        }
      });
      pendingSaveRef.current = p;
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
    if (currentView === "experts") {
      return <ExpertLibraryView />;
    }
    if (currentView === "workflow-library") {
      return <WorkflowLibraryView />;
    }

    // 项目空间页面 — 需要项目上下文
    if (!currentProject || !projectData) {
      return <ProjectManager onSelectProject={handleSelectProject} currentProjectId={currentProject?.project_id ?? null} onDeleteProject={handleProjectDeleted} />;
    }

    switch (currentView) {
      case "concept":
        return <ConceptView projectData={projectData} persistProjectData={persistProjectData} />;
      case "dashboard":
        return <ProjectDashboard project={currentProject} projectData={projectData} onNavigate={setCurrentView} persistProjectData={persistProjectData} />;
      case "outline":
        return <OutlineView projectData={projectData} persistProjectData={persistProjectData} onRefresh={refreshNow} />;
      case "blueprint":
        return <BlueprintView projectData={projectData} onRefresh={refreshNow} />;
      case "writing":
        return <WritingView projectData={projectData} persistProjectData={persistProjectData} chapterId={currentChapterId} onWordCountChange={handleWordCountChange} />;
      case "character":
        return <CharacterView projectData={projectData} persistProjectData={persistProjectData} />;
      case "world":
        return <WorldView projectData={projectData} persistProjectData={persistProjectData} />;
      case "annotations":
        return <AnnotationInbox onNavigate={setCurrentView} />;
      case "consistency":
        return <StyleWorkshop projectData={projectData} />;
      case "harness":
        return <HarnessConsole projectData={projectData} persistProjectData={persistProjectData} onNavigate={setCurrentView} />;
      case "style":
        return <StyleWorkshop projectData={projectData} />;
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
