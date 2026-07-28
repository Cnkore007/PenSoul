import { useState, useEffect } from "react";
import { Trash2, Edit3, BookOpen, Calendar, FileText, Sparkles } from "lucide-react";
import type { ProjectMeta } from "../types";
import { loadProjects, saveProjects, deleteProjectData } from "../store";

interface ProjectManagerProps {
  onSelectProject: (project: ProjectMeta) => void;
  currentProjectId: string | null;
  onDeleteProject?: (projectId: string) => void;
}

export function ProjectManager({ onSelectProject, onDeleteProject }: ProjectManagerProps) {
  const [projects, setProjects] = useState<ProjectMeta[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingProject, setEditingProject] = useState<ProjectMeta | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  useEffect(() => { setProjects(loadProjects()); }, []);

  function persist(updated: ProjectMeta[]) {
    setProjects(updated);
    saveProjects(updated);
  }

  function handleCreate() {
    if (!newTitle.trim()) return;
    const p: ProjectMeta = {
      project_id: `proj-${Date.now()}`,
      title: newTitle.trim(),
      description: newDescription.trim(),
      created_at: new Date().toISOString().split("T")[0],
      updated_at: new Date().toISOString().split("T")[0],
      total_chapters: 0,
      total_words: 0,
    };
    persist([p, ...projects]);
    handleCancel();
  }

  function handleEdit() {
    if (!editingProject || !newTitle.trim()) return;
    persist(projects.map(p => p.project_id === editingProject.project_id
      ? { ...p, title: newTitle.trim(), description: newDescription.trim(), updated_at: new Date().toISOString().split("T")[0] }
      : p));
    handleCancel();
  }

  function handleDelete(id: string) {
    persist(projects.filter(p => p.project_id !== id));
    deleteProjectData(id);
    setConfirmDelete(null);
    onDeleteProject?.(id);
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
        <button className="pm-create-btn" onClick={() => setShowCreateModal(true)}>
          <Sparkles size={18} /><span>新建作品</span>
        </button>
      </div>

      {projects.length === 0 ? (
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
              <button className="pm-btn pm-btn-primary" onClick={editingProject ? handleEdit : handleCreate} disabled={!newTitle.trim()}>
                {editingProject ? "保存" : "创建"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
