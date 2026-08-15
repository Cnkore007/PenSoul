// ProjectsView — 作品库

import { useState, useEffect, useCallback } from "react";
import { listProjects, createProject, openProject, deleteProject } from "../ipc";
import type { ProjectSummary } from "../types";

export default function ProjectsView({ onProjectOpen }: { onProjectOpen: (p: ProjectSummary) => void }) {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [newName, setNewName] = useState("");
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  const refresh = useCallback(async () => {
    try { setProjects(await listProjects()); } catch { setMsg("后端未连接"); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleCreate = async () => {
    const title = newName.trim();
    if (!title) return;
    setLoading(true); setMsg("");
    try {
      const id = generateProjectId(title);
      await createProject(id, title);
      setNewName(""); setMsg(`项目「${title}」已创建`); refresh();
    } catch (e: any) { setMsg(`创建失败: ${e}`); }
    finally { setLoading(false); }
  };

  const handleOpen = async (project: ProjectSummary) => {
    try { await openProject(project.project_id); onProjectOpen(project); }
    catch (e: any) { setMsg(`打开失败: ${e}`); }
  };

  const handleDelete = async (projectId: string) => {
    const project = projects.find((p) => p.project_id === projectId);
    const label = project ? project.title : projectId;
    if (!confirm(`确定删除项目「${label}」吗？此操作不可恢复。`)) return;
    try {
      await deleteProject(projectId);
      refresh();
    } catch (e: any) { setMsg(`删除失败: ${e}`); }
  };

  return (
    <div className="view-card">
      <h2>作品库</h2>
      <div className="form-row">
        <input className="ps-input" placeholder="项目名称（支持中文）" value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleCreate()} />
        <button className="btn-primary" onClick={handleCreate} disabled={loading}>创建</button>
      </div>
      {msg && <p className="msg">{msg}</p>}
      {projects.length === 0 ? (
        <p className="empty">暂无项目，创建一个开始吧。</p>
      ) : (
        <ul className="project-list">
          {projects.map((p) => (
            <li key={p.project_id} className="project-item">
              <span className="project-name">{p.title || p.project_id}</span>
              <div className="btn-group">
                <button className="btn-sm" onClick={() => handleOpen(p)}>打开</button>
                <button className="btn-sm btn-danger" onClick={() => handleDelete(p.project_id)}>删除</button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/// 从项目名称自动生成合法的英文项目 ID
/// 优先取名称里的英文/数字片段；纯中文时回退为 project-<随机>
function generateProjectId(title: string): string {
  const ascii = title
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9_-]/g, "");
  if (ascii) {
    return ascii.length <= 64 ? ascii : ascii.slice(0, 64);
  }
  const suffix = `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
  return `project-${suffix}`;
}
