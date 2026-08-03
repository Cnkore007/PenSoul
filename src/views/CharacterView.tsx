import { useState, useMemo, useCallback } from "react";
import { UserPlus, Trash2, Edit3, ChevronDown, ChevronUp, Search, Eraser } from "lucide-react";
import type { ProjectData, CharacterData } from "../types";
import { SaveControls } from "../components/SaveControls";
import { EntityAnnotations } from "../components/EntityAnnotations";
import { clearCharacters } from "../ipc";
import { confirmDialog, messageDialog } from "../dialogs";

interface CharacterViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
}

function parseTraits(text: string): Array<[string, number]> {
  return text.split(/[,，、\s]+/).filter(Boolean).map(t => [t.trim(), 0.5]);
}

export function CharacterView({ projectData, persistProjectData }: CharacterViewProps) {
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState("");
  const [traits, setTraits] = useState("");
  const [mood, setMood] = useState("");
  // 行内编辑
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editTraits, setEditTraits] = useState("");
  const [editMood, setEditMood] = useState("");
  // 渲染保护：分页 + 搜索 + 关系折叠（角色/关系量大时不卡）
  const [visibleCount, setVisibleCount] = useState(24);
  const [query, setQuery] = useState("");
  const [expandedRels, setExpandedRels] = useState<Set<string>>(new Set());

  const characters = projectData.characters;
  const filtered = useMemo(() => {
    const q = query.trim();
    if (!q) return characters;
    return characters.filter(c =>
      c.name.includes(q) || c.personality_traits.some(([t]) => t.includes(q))
    );
  }, [characters, query]);

  function toggleRels(id: string) {
    setExpandedRels(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }

  // 优化/撤回：写回项目数据（函数式更新，组件卸载后仍生效）
  const applyCharacters = useCallback((parsed: CharacterData[]) => {
    if (!Array.isArray(parsed)) return;
    persistProjectData(prev => ({ ...prev, characters: parsed }));
  }, [persistProjectData]);

  const charactersJson = useMemo(() => JSON.stringify(characters), [characters]);

  function handleAdd() {
    if (!name.trim()) return;
    const char: CharacterData = {
      id: `char-${Date.now()}`,
      name: name.trim(),
      personality_traits: parseTraits(traits),
      current_mood: mood.trim() || undefined,
      relationships: [],
    };
    persistProjectData(prev => ({ ...prev, characters: [...prev.characters, char] }));
    setName(""); setTraits(""); setMood(""); setShowForm(false);
  }

  function handleDelete(id: string) {
    persistProjectData(prev => ({ ...prev, characters: prev.characters.filter(c => c.id !== id) }));
  }

  // 一键清空人物志（前后端同步）
  async function handleClearAll() {
    if (characters.length === 0) return;
    if (!(await confirmDialog(`一键删除全部 ${characters.length} 个角色？\n角色及其关系将全部删除，不可恢复。`))) return;
    try {
      await clearCharacters();
      persistProjectData(prev => ({ ...prev, characters: [] }));
      await messageDialog(`已删除全部 ${characters.length} 个角色`);
    } catch (e: any) {
      await messageDialog("清空失败：\n" + (typeof e === "string" ? e : e?.message || String(e)));
    }
  }

  function startEdit(char: CharacterData) {
    setEditingId(char.id);
    setEditName(char.name);
    setEditTraits(char.personality_traits.map(t => t[0]).join("、"));
    setEditMood(char.current_mood || "");
  }

  function handleSaveEdit() {
    if (!editingId || !editName.trim()) return;
    persistProjectData(prev => ({
      ...prev,
      characters: prev.characters.map(c => c.id === editingId ? {
        ...c,
        name: editName.trim(),
        personality_traits: parseTraits(editTraits),
        current_mood: editMood.trim() || undefined,
      } : c),
    }));
    setEditingId(null);
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>人物志</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-secondary" onClick={handleClearAll}
            title="一键删除全部角色与人物关系（不可恢复）"
            disabled={characters.length === 0}>
            <Eraser size={15} /> 全部删除
          </button>
          <SaveControls
            type="character"
            contentJson={charactersJson}
            apply={applyCharacters}
            disabled={characters.length === 0}
          />
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
        <div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, flex: 1, maxWidth: 320 }}>
              <Search size={14} style={{ color: "var(--color-ink-3)" }} />
              <input
                className="pm-input"
                style={{ marginBottom: 0 }}
                placeholder="搜索角色名 / 性格特征"
                value={query}
                onChange={e => { setQuery(e.target.value); setVisibleCount(24); }}
              />
            </div>
            <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
              {filtered.length > visibleCount
                ? `显示 ${visibleCount} / ${filtered.length} 位角色`
                : `${filtered.length} 位角色`}
            </span>
          </div>
          <div className="grid-auto">
          {filtered.slice(0, visibleCount).map(char => (
            <div key={char.id} className="char-card">
              {editingId === char.id ? (
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <input className="pm-input" style={{ marginBottom: 0 }} value={editName} onChange={e => setEditName(e.target.value)} autoFocus />
                  <input className="pm-input" style={{ marginBottom: 0 }} placeholder="性格特征（逗号分隔）" value={editTraits} onChange={e => setEditTraits(e.target.value)} />
                  <input className="pm-input" style={{ marginBottom: 0 }} placeholder="当前心境（可选）" value={editMood} onChange={e => setEditMood(e.target.value)} />
                  <div style={{ display: "flex", gap: 8 }}>
                    <button className="btn btn-primary" onClick={handleSaveEdit} disabled={!editName.trim()}>保存</button>
                    <button className="btn btn-secondary" onClick={() => setEditingId(null)}>取消</button>
                  </div>
                </div>
              ) : (
                <>
                  <div className="char-header">
                    <div className="char-avatar"><span className="char-avatar-letter">{char.name.charAt(0)}</span></div>
                    <div className="char-info">
                      <h3 className="char-name">{char.name}</h3>
                      {char.current_mood && <span className="char-mood">{char.current_mood}</span>}
                    </div>
                    <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
                      <EntityAnnotations target={`character:${char.id}`} />
                      <button className="pv-icon-btn" onClick={() => startEdit(char)} title="编辑"><Edit3 size={14} /></button>
                      <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDelete(char.id)} title="删除"><Trash2 size={14} /></button>
                    </div>
                  </div>
                  <div className="char-section">
                    <div className="char-section-label">性情</div>
                    <div className="char-traits">
                      {char.personality_traits.map(([trait]) => (
                        <span key={trait} className="tag tag-accent">{trait}</span>
                      ))}
                    </div>
                  </div>
                  {char.relationships.length > 0 && (
                    <div className="char-section">
                      <div className="char-section-label" style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        关系（{char.relationships.length}）
                        {char.relationships.length > 6 && (
                          <button className="pv-icon-btn" title={expandedRels.has(char.id) ? "收起" : "展开全部"} onClick={() => toggleRels(char.id)}>
                            {expandedRels.has(char.id) ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                          </button>
                        )}
                      </div>
                      <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", lineHeight: 1.6 }}>
                        {(expandedRels.has(char.id) ? char.relationships : char.relationships.slice(0, 6)).map((r, i) => (
                          <div key={i}>{r.from} → {r.to}：{r.relation_type}</div>
                        ))}
                        {!expandedRels.has(char.id) && char.relationships.length > 6 && (
                          <div style={{ color: "var(--color-ink-3)", opacity: 0.7 }}>… 其余 {char.relationships.length - 6} 条已折叠</div>
                        )}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          ))}
          </div>
          {filtered.length > visibleCount && (
            <div style={{ textAlign: "center", marginTop: 16 }}>
              <button className="btn btn-secondary" onClick={() => setVisibleCount(n => n + 24)}>
                加载更多（{filtered.length - visibleCount} 位）
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
