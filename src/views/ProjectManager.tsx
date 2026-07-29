import { useState, useEffect, useCallback } from "react";
import { Trash2, Edit3, BookOpen, Calendar, FileText, Sparkles } from "lucide-react";
import type { ProjectMeta } from "../types";
import * as ipc from "../ipc";

interface ProjectManagerProps {
  onSelectProject: (project: ProjectMeta) => void;
  currentProjectId: string | null;
  onDeleteProject?: (projectId: string) => void;
}

export function ProjectManager({ onSelectProject, currentProjectId: _currentProjectId, onDeleteProject }: ProjectManagerProps) {
  const [projects, setProjects] = useState<ProjectMeta[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingProject, setEditingProject] = useState<ProjectMeta | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshProjects = useCallback(async () => {
    try {
      const list = await ipc.listProjects();
      setProjects(list as ProjectMeta[]);
    } catch (e) {
      console.error("加载项目列表失败:", e);
      setError("加载项目列表失败");
    }
  }, []);

  useEffect(() => { refreshProjects(); }, [refreshProjects]);

  async function handleCreate() {
    if (!newTitle.trim()) return;
    try {
      setLoading(true);
      setError(null);
      await ipc.createProject(newTitle.trim());
      await refreshProjects();
      handleCancel();
    } catch (e) {
      console.error("创建项目失败:", e);
      setError("创建项目失败");
    } finally {
      setLoading(false);
    }
  }

  async function handleEdit() {
    if (!editingProject || !newTitle.trim()) return;
    try {
      setLoading(true);
      setError(null);
      await ipc.updateProject(editingProject.project_id, newTitle.trim(), newDescription.trim());
      await refreshProjects();
      handleCancel();
    } catch (e) {
      console.error("编辑项目失败:", e);
      setError("编辑项目失败");
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(id: string) {
    try {
      setLoading(true);
      setError(null);
      await ipc.deleteProject(id);
      await refreshProjects();
      setConfirmDelete(null);
      onDeleteProject?.(id);
    } catch (e) {
      console.error("删除项目失败:", e);
      setError("删除项目失败");
    } finally {
      setLoading(false);
    }
  }

  function handleCancel() {
    setShowCreateModal(false);
    setEditingProject(null);
    setNewTitle("");
    setNewDescription("");
  }

  const isModal = showCreateModal || editingProject;

  return (
    <div className="project-manager">
      <div className="pm-hero">
        <div className="pm-hero-content">
          <div className="pm-hero-seal">笔</div>
          <div>
            <h1 className="pm-hero-title">作品库</h1>
            <p className="pm-hero-subtitle">笔墨落处，便是江湖</p>
          </div>
        </div>
        <button className="pm-create-btn" onClick={() => setShowCreateModal(true)} disabled={loading}>
          <Sparkles size={18} /><span>新建作品</span>
        </button>
      </div>

      {error && (
        <div style={{ padding: "8px 16px", margin: "0 16px", background: "#fef2f2", color: "#991b1b", borderRadius: 6, fontSize: "var(--text-sm)" }}>
          {error}
        </div>
      )}

      {projects.length === 0 && !loading ? (
        <div className="pm-empty">
          <div className="pm-empty-icon">卷</div>
          <div className="pm-empty-title">尚无作品</div>
          <div className="pm-empty-sub">点击「新建作品」，开启创作之旅</div>
        </div>
      ) : (
        <div className="pm-grid">
          {projects.map(project => (
            <div key={project.project_id} className="pm-card">
              <div className="pm-card-top" onClick={() => onSelectProject(project)}>
                <div className="pm-card-initial">{project.title.charAt(0)}</div>
                <div className="pm-card-info">
                  <h3 className="pm-card-title">{project.title}</h3>
                  <p className="pm-card-desc">{project.description || "暂无简介"}</p>
                </div>
              </div>
              <div className="pm-card-meta">
                <div className="pm-meta-item"><FileText size={13} /><span>{project.total_chapters} 章 · {project.total_words.toLocaleString()} 字</span></div>
                <div className="pm-meta-item"><Calendar size={13} /><span>{project.updated_at}</span></div>
              </div>
              <div className="pm-card-actions">
                <button className="pm-action-btn pm-action-open" onClick={() => onSelectProject(project)}>
                  <BookOpen size={14} /> 打开
                </button>
                <button className="pm-action-btn" onClick={() => { setEditingProject(project); setNewTitle(project.title); setNewDescription(project.description); }}>
                  <Edit3 size={14} /> 编辑
                </button>
                {confirmDelete === project.project_id ? (
                  <div className="pm-confirm-delete">
                    <span>确定？</span>
                    <button className="pm-confirm-yes" onClick={() => handleDelete(project.project_id)}>是</button>
                    <button className="pm-confirm-no" onClick={() => setConfirmDelete(null)}>否</button>
                  </div>
                ) : (
                  <button className="pm-action-btn pm-action-delete" onClick={() => setConfirmDelete(project.project_id)}>
                    <Trash2 size={14} /> 删除
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {isModal && (
        <div className="pm-modal-overlay" onClick={handleCancel}>
          <div className="pm-modal" onClick={e => e.stopPropagation()}>
            <div className="pm-modal-header"><h3>{editingProject ? "编辑作品" : "新建作品"}</h3></div>
            <div className="pm-modal-body">
              <label className="pm-label">作品标题</label>
              <input className="pm-input" type="text" value={newTitle} onChange={e => setNewTitle(e.target.value)} placeholder="请输入作品标题" autoFocus />
              <label className="pm-label">作品简介</label>
              <textarea className="pm-textarea" value={newDescription} onChange={e => setNewDescription(e.target.value)} rows={3} placeholder="请输入作品简介（可选）" />
            </div>
            <div className="pm-modal-footer">
              <button className="pm-btn pm-btn-cancel" onClick={handleCancel}>取消</button>
              <button className="pm-btn pm-btn-primary" onClick={editingProject ? handleEdit : handleCreate} disabled={!newTitle.trim() || loading}>
                {loading ? "处理中..." : (editingProject ? "保存" : "创建")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
