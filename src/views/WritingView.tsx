import { useState, useEffect, useMemo } from "react";
import { Save, BookOpen, ChevronRight, ChevronDown, FileText } from "lucide-react";
import { TipTapEditor } from "../components/TipTapEditor";
import type { ProjectData, Chapter } from "../types";

interface WritingViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  chapterId: string | null;
  onWordCountChange: (count: number) => void;
}

export function WritingView({ projectData, persistProjectData, chapterId, onWordCountChange }: WritingViewProps) {
  const [chapter, setChapter] = useState<Chapter | null>(null);
  const [content, setContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(chapterId);
  const [expandedVolumes, setExpandedVolumes] = useState<Record<string, boolean>>({});
  const [showNav, setShowNav] = useState(true);

  // 笔耕只显示已开始细写（有正文）的章节；只有梗概的章节留在大纲页，
  // 待工作流细写出正文后才会出现在这里
  const writingVolumes = useMemo(
    () => projectData.volumes
      .map(v => ({ ...v, chapters: v.chapters.filter(c => c.word_count > 0) }))
      .filter(v => v.chapters.length > 0),
    [projectData.volumes]
  );

  // 展开所有卷
  useEffect(() => {
    const expanded: Record<string, boolean> = {};
    writingVolumes.forEach(v => { expanded[v.volume_id] = true; });
    setExpandedVolumes(expanded);
  }, [writingVolumes]);

  // 从 prop 同步外部选中
  useEffect(() => {
    if (chapterId) setSelectedId(chapterId);
  }, [chapterId]);

  // 选中的章节变化时加载内容
  useEffect(() => {
    if (!selectedId) { setChapter(null); setContent(""); return; }
    for (const vol of projectData.volumes) {
      const ch = vol.chapters.find(c => c.chapter_id === selectedId);
      if (ch) { setChapter(ch); setContent(ch.content); onWordCountChange(ch.word_count); return; }
    }
    setChapter(null); setContent("");
  }, [selectedId, projectData, onWordCountChange]);

  // 字数统计
  useEffect(() => {
    const plainText = content.replace(/<[^>]*>/g, "");
    onWordCountChange(plainText.length);
  }, [content, onWordCountChange]);

  function handleSave() {
    if (!chapter) return;
    setSaving(true);
    const plainText = content.replace(/<[^>]*>/g, "");
    persistProjectData(prev => ({
      ...prev,
      volumes: prev.volumes.map(v => ({
        ...v,
        chapters: v.chapters.map(c => c.chapter_id === chapter.chapter_id
          ? { ...c, content, word_count: plainText.length, version: c.version + 1 }
          : c),
      })),
    }));
    setChapter({ ...chapter, content, version: chapter.version + 1, word_count: plainText.length });
    setSaveMsg("已保存");
    setSaving(false);
    setTimeout(() => setSaveMsg(null), 2000);
  }

  function toggleVolume(volId: string) {
    setExpandedVolumes(prev => ({ ...prev, [volId]: !prev[volId] }));
  }

  // 统计（只统计写作中的章节）
  const totalChapters = useMemo(
    () => writingVolumes.reduce((s, v) => s + v.chapters.length, 0),
    [writingVolumes]
  );

  return (
    <div className="writing-layout">
      {/* 章节导航侧边栏 */}
      {showNav && totalChapters > 0 && (
        <div className="writing-nav">
          <div className="writing-nav-header">
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-sm)", letterSpacing: "1px" }}>章节导航</span>
          </div>
          <div className="writing-nav-list">
            {writingVolumes.map(volume => (
              <div key={volume.volume_id} className="writing-nav-volume">
                <div
                  className="writing-nav-vol-header"
                  onClick={() => toggleVolume(volume.volume_id)}
                >
                  {expandedVolumes[volume.volume_id] ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                  <span className="writing-nav-vol-title">{volume.volume_id === "_default" ? "未分卷" : volume.title}</span>
                  <span className="writing-nav-vol-count">{volume.chapters.length} 章</span>
                </div>
                {expandedVolumes[volume.volume_id] && volume.chapters.map(ch => (
                  <div
                    key={ch.chapter_id}
                    className={`writing-nav-chapter ${selectedId === ch.chapter_id ? "active" : ""}`}
                    onClick={() => setSelectedId(ch.chapter_id)}
                  >
                    <FileText size={12} />
                    <span className="writing-nav-ch-title">{ch.title}</span>
                    <span className="writing-nav-ch-words">{ch.word_count.toLocaleString()}</span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}

      {totalChapters === 0 && (
        <div className="writing-nav" style={{ justifyContent: "center", alignItems: "center", display: "flex" }}>
          <div style={{ textAlign: "center", color: "var(--color-ink-faint)", fontSize: "var(--text-sm)" }}>
            <FileText size={24} strokeWidth={1} style={{ marginBottom: 8, opacity: 0.4 }} />
            <div>暂无写作中的章节</div>
            <div style={{ fontSize: "var(--text-xs)", marginTop: 4 }}>章节经工作流细写后会出现在这里</div>
          </div>
        </div>
      )}

      {/* 主编辑区 */}
      <div className="writing-main">
        <div className="view-header" style={{ paddingLeft: showNav && totalChapters > 0 ? "var(--space-sm)" : undefined }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
            {totalChapters > 0 && (
              <button
                className="btn btn-ghost"
                style={{ padding: "4px", flexShrink: 0 }}
                onClick={() => setShowNav(!showNav)}
                title={showNav ? "隐藏导航" : "显示导航"}
              >
                {showNav ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
              </button>
            )}
            <h2 style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {chapter?.title || (totalChapters > 0 ? "选择章节" : "笔耕")}
            </h2>
            {chapter && <span className={`badge badge-${chapter.status.toLowerCase()}`}>{chapter.status}</span>}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <button className={"btn btn-primary" + (saving || !chapter ? " btn-disabled" : "")} onClick={handleSave} disabled={saving || !chapter}>
              <Save size={15} /> {saving ? "保存中..." : "保存"}
            </button>
          </div>
        </div>
        {saveMsg && <div className="save-message success">{saveMsg}</div>}
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
          {chapter ? (
            <TipTapEditor key={chapter.chapter_id} content={content} onChange={setContent} placeholder="落笔之处，便是江湖..." />
          ) : (
            <div className="empty-state" style={{ flex: 1 }}>
              <div className="empty-state-icon">笔</div>
              <div className="empty-state-text">
                {totalChapters > 0 ? "选择章节开始创作" : "笔墨未落，尚待挥毫"}
              </div>
              <div className="empty-state-sub">
                {totalChapters > 0
                  ? "从左侧导航选择一个章节"
                  : "章节由工作流细写后，在此打磨正文"}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* 信息侧边栏 */}
      {chapter && (
        <div className="card writing-info">
          <div className="card-header"><BookOpen size={15} color="var(--color-ink-3)" /><h3>章节信息</h3></div>
          <div className="writing-info-section">
            <div className="writing-info-label">字数</div>
            <div className="writing-info-value-lg">{chapter.word_count.toLocaleString()}</div>
            <div className="writing-info-unit">字</div>
          </div>
          <div className="writing-info-section">
            <div className="writing-info-label">版本</div>
            <div className="writing-info-value">第 {chapter.version} 版</div>
          </div>
          <div className="writing-info-section">
            <div className="writing-info-label">进度</div>
            <span className={`badge badge-${chapter.status.toLowerCase()}`}>{chapter.status}</span>
          </div>
        </div>
      )}
    </div>
  );
}
