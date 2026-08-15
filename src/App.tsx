// App.tsx — 应用布局与视图路由

import { useState, useCallback } from "react";
import Sidebar from "./components/Sidebar";
import StatusBar from "./components/StatusBar";
import type { ProjectSummary, ViewType } from "./types";
import ProjectsView from "./views/ProjectsView";
import DashboardView from "./views/DashboardView";
import ConceptView from "./views/ConceptView";
import EntityGraphView from "./views/EntityGraphView";
import ConstraintView from "./views/ConstraintView";
import WritingView from "./views/WritingView";
import OutlineView from "./views/OutlineView";
import SettingsView from "./views/SettingsView";

const PROJECT_VIEWS: ViewType[] = [
  "dashboard",
  "concept",
  "entity-graph",
  "outline",
  "writing",
  "constraint",
];

function App() {
  const [currentView, setCurrentView] = useState<ViewType>("projects");
  const [currentProject, setCurrentProject] = useState<{ id: string; title: string } | null>(null);

  const handleProjectOpen = useCallback((project: ProjectSummary) => {
    setCurrentProject({ id: project.project_id, title: project.title || project.project_id });
    setCurrentView("dashboard");
  }, []);

  const handleProjectClose = useCallback(() => {
    setCurrentProject(null);
    setCurrentView("projects");
  }, []);

  return (
    <div className="app">
      <Sidebar
        currentView={currentView}
        onViewChange={setCurrentView}
        currentProjectName={currentProject?.title ?? null}
        onProjectClose={handleProjectClose}
      />
      <main className="main-area">
        <div className="content-area">
          <h1 className="view-title">
            <span className="diamond" />
            PenSoul 2.0
          </h1>
          {currentView === "projects" && !currentProject && (
            <ProjectsView onProjectOpen={handleProjectOpen} />
          )}
          {currentView === "settings" && <SettingsView />}
          {currentProject && PROJECT_VIEWS.includes(currentView) && (
            <>
              {currentView === "dashboard" && <DashboardView />}
              {currentView === "concept" && <ConceptView />}
              {currentView === "entity-graph" && <EntityGraphView />}
              {currentView === "constraint" && <ConstraintView />}
              {currentView === "outline" && <OutlineView />}
              {currentView === "writing" && <WritingView />}
            </>
          )}
          {currentProject && !PROJECT_VIEWS.includes(currentView) && currentView !== "settings" && (
            <div className="view-card">
              <h2>{getViewLabel(currentView)}</h2>
              <p>功能开发中。</p>
            </div>
          )}
          {!currentProject && currentView !== "projects" && currentView !== "settings" && (
            <div className="view-card">
              <p className="empty">请先在作品库中打开一个项目。</p>
            </div>
          )}
        </div>
        <StatusBar />
      </main>
    </div>
  );
}

function getViewLabel(view: ViewType): string {
  const labels: Record<ViewType, string> = {
    projects: "作品库", dashboard: "仪表盘", concept: "萌芽",
    "entity-graph": "图谱", outline: "大纲", writing: "笔耕",
    constraint: "约束", settings: "设定",
  };
  return labels[view] || view;
}

export default App;
