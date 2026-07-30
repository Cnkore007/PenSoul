import { useState } from "react";
import { ChevronRight, ChevronDown, FileText, Plus } from "lucide-react";
import type { ProjectData, VolumeWithChapters, Chapter } from "../types";

interface OutlineViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  onSelectChapter: (chapterId: string) => void;
  currentChapterId: string | null;
}

export function OutlineView({ projectData, persistProjectData, onSelectChapter, currentChapterId }: OutlineViewProps) {
  const [newVolumeTitle, setNewVolumeTitle] = useState("");
  const [showNewVolume, setShowNewVolume] = useState(false);
  const [newChapterTitle, setNewChapterTitle] = useState("");
  const [showNewChapterFor, setShowNewChapterFor] = useState<string | null>(null);

  const volumes = projectData.volumes;
  const totalChapters = volumes.reduce((s, v) => s + v.chapters.length, 0);
  const totalWords = volumes.reduce((s, v) => s + v.chapters.reduce((s2, c) => s2 + c.word_count, 0), 0);

  function toggleVolume(volId: string) {
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId ? { ...v, expanded: !v.expanded } : v),
    }));
  }

  function addVolume() {
    if (!newVolumeTitle.trim()) return;
    const vol: VolumeWithChapters = {
      volume_id: `vol-${Date.now()}`,
      title: newVolumeTitle.trim(),
      chapter_count: 0,
      expanded: true,
      chapters: [],
    };
    persistProjectData(prev => ({ ...prev, volumes: [...prev.volumes, vol] }));
    setNewVolumeTitle("");
    setShowNewVolume(false);
  }

  function addChapter(volId: string) {
    if (!newChapterTitle.trim()) return;
    const ch: Chapter = {
      chapter_id: `ch-${Date.now()}`,
      volume_id: volId,
      title: newChapterTitle.trim(),
      content: "",
      word_count: 0,
      version: 1,
      status: "Draft",
    };
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => v.volume_id === volId
        ? { ...v, chapters: [...v.chapters, ch], chapter_count: v.chapters.length + 1 }
        : v),
    }));
    setNewChapterTitle("");
    setShowNewChapterFor(null);
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>大纲</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-primary" onClick={() => setShowNewVolume(true)}>
            <Plus size={15} /> 新建卷
          </button>
        </div>
      </div>

      {showNewVolume && (
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <input className="pm-input" style={{ marginBottom: 0, flex: 1 }} placeholder="卷名" value={newVolumeTitle} onChange={e => setNewVolumeTitle(e.target.value)} autoFocus onKeyDown={e => e.key === "Enter" && addVolume()} />
          <button className="btn btn-primary" onClick={addVolume}>确定</button>
          <button className="btn btn-secondary" onClick={() => { setShowNewVolume(false); setNewVolumeTitle(""); }}>取消</button>
        </div>
      )}

      <div className="card outline-card">
        {volumes.length === 0 && (
          <div className="empty-state" style={{ padding: "40px 20px" }}>
            <div className="empty-state-icon">纲</div>
            <div className="empty-state-text">尚无大纲</div>
            <div className="empty-state-sub">点击「新建卷」开始构建你的故事骨架</div>
          </div>
        )}
        {volumes.map(volume => (
          <div key={volume.volume_id} className="outline-volume-group">
            <div onClick={() => toggleVolume(volume.volume_id)} className="volume-header">
              <span className="volume-expand-icon">
                {volume.expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
              </span>
              <span className="volume-title">{volume.title}</span>
              <span className="volume-count">{volume.chapters.length} 章</span>
            </div>
            {volume.expanded && (
              <div className="chapter-list">
                {volume.chapters.map(ch => (
                  <div key={ch.chapter_id}
                    onClick={() => onSelectChapter(ch.chapter_id)}
                    className={"chapter-item" + (currentChapterId === ch.chapter_id ? " active" : "")}>
                    <FileText size={13} className="chapter-icon" />
                    <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
                      <span className={"chapter-title" + (currentChapterId === ch.chapter_id ? " active" : "")}>{ch.title}</span>
                      {ch.summary && (
                        <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                          {ch.summary}
                        </span>
                      )}
                    </div>
                    <span className="chapter-words">{ch.word_count.toLocaleString()} 字</span>
                    <div className={"status-dot status-dot-" + ch.status.toLowerCase()} />
                  </div>
                ))}
                {showNewChapterFor === volume.volume_id ? (
                  <div style={{ display: "flex", gap: 8, padding: "8px 32px" }}>
                    <input className="pm-input" style={{ marginBottom: 0, flex: 1 }} placeholder="章节标题" value={newChapterTitle} onChange={e => setNewChapterTitle(e.target.value)} autoFocus onKeyDown={e => e.key === "Enter" && addChapter(volume.volume_id)} />
                    <button className="btn btn-primary" style={{ padding: "4px 10px" }} onClick={() => addChapter(volume.volume_id)}>添加</button>
                    <button className="btn btn-secondary" style={{ padding: "4px 10px" }} onClick={() => { setShowNewChapterFor(null); setNewChapterTitle(""); }}>取消</button>
                  </div>
                ) : (
                  <div style={{ padding: "4px 32px" }}>
                    <button className="btn btn-secondary" style={{ fontSize: "var(--text-xs)", padding: "2px 8px" }} onClick={() => setShowNewChapterFor(volume.volume_id)}>
                      <Plus size={12} /> 添加章节
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="outline-stats">
        <span>共 {totalChapters} 章</span>
        <span className="outline-stat-sep">·</span>
        <span>{totalWords.toLocaleString()} 字</span>
      </div>
    </div>
  );
}
