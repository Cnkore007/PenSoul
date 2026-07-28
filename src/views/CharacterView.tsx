import { useState, useMemo } from "react";
import { UserPlus, Trash2, Sparkles } from "lucide-react";
import type { ProjectData, CharacterData } from "../types";
import { InspirationPanel } from "../components/InspirationPanel";

interface CharacterViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
}

export function CharacterView({ projectData, persistProjectData }: CharacterViewProps) {
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState("");
  const [traits, setTraits] = useState("");
  const [mood, setMood] = useState("");
  const [inspirationOpen, setInspirationOpen] = useState(false);

  const characters = projectData.characters;

  function handleAdd() {
    if (!name.trim()) return;
    const traitPairs: [string, number][] = traits.split(/[,，、\s]+/).filter(Boolean).map(t => [t.trim(), 0.5]);
    const char: CharacterData = {
      id: `char-${Date.now()}`,
      name: name.trim(),
      personality_traits: traitPairs,
      current_mood: mood.trim() || undefined,
      relationships: [],
    };
    persistProjectData(prev => ({ ...prev, characters: [...prev.characters, char] }));
    setName(""); setTraits(""); setMood(""); setShowForm(false);
  }

  function handleDelete(id: string) {
    persistProjectData(prev => ({ ...prev, characters: prev.characters.filter(c => c.id !== id) }));
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>人物志</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            className="btn btn-ghost"
            onClick={() => setInspirationOpen(!inspirationOpen)}
            title="AI 灵感"
            style={{ color: inspirationOpen ? "var(--color-accent)" : undefined }}
          >
            <Sparkles size={15} /> 灵感
          </button>
          <button className="btn btn-primary" onClick={() => setShowForm(true)}><UserPlus size={15} /> 新建角色</button>
        </div>
      </div>

      {showForm && (
        <div className="card" style={{ marginBottom: 16 }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input className="pm-input" style={{ marginBottom: 0 }} placeholder="角色名" value={name} onChange={e => setName(e.target.value)} autoFocus />
            <input className="pm-input" style={{ marginBottom: 0 }} placeholder="性格特征（逗号分隔，如：冷静、聪慧、固执）" value={traits} onChange={e => setTraits(e.target.value)} />
            <input className="pm-input" style={{ marginBottom: 0 }} placeholder="当前心境（可选）" value={mood} onChange={e => setMood(e.target.value)} />
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-primary" onClick={handleAdd}>添加</button>
              <button className="btn btn-secondary" onClick={() => { setShowForm(false); setName(""); setTraits(""); setMood(""); }}>取消</button>
            </div>
          </div>
        </div>
      )}

      {characters.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">人</div>
          <div className="empty-state-text">人物未定，故事难成</div>
          <div className="empty-state-sub">每一位角色都是故事的灵魂</div>
        </div>
      ) : (
        <div className="grid-auto">
          {characters.map(char => (
            <div key={char.id} className="char-card">
              <div className="char-header">
                <div className="char-avatar"><span className="char-avatar-letter">{char.name.charAt(0)}</span></div>
                <div className="char-info">
                  <h3 className="char-name">{char.name}</h3>
                  {char.current_mood && <span className="char-mood">{char.current_mood}</span>}
                </div>
                <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDelete(char.id)} title="删除"><Trash2 size={14} /></button>
              </div>
              <div className="char-section">
                <div className="char-section-label">性情</div>
                <div className="char-traits">
                  {char.personality_traits.map(([trait]) => (
                    <span key={trait} className="tag tag-accent">{trait}</span>
                  ))}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
      {/* 灵感面板 */}
      <InspirationPanel
        contextType="character"
        contextData={useMemo(() => JSON.stringify({
          characters: projectData.characters.map(c => ({
            name: c.name,
            traits: c.personality_traits.map(t => t[0]),
            mood: c.current_mood,
          })),
        }), [projectData.characters])}
        externalExpanded={inspirationOpen}
        onToggle={() => setInspirationOpen(!inspirationOpen)}
        hideTrigger={true}
      />
    </div>
  );
}
