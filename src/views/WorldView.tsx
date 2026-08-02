import { useState, useMemo, useCallback } from "react";
import { MapPin, Clock, BookOpen, Plus, Trash2, Edit3 } from "lucide-react";
import type { ProjectData, WorldData } from "../types";
import { OptimizeControls } from "../components/OptimizeControls";
import { EntityAnnotations } from "../components/EntityAnnotations";

type TabType = "locations" | "timeline" | "rules";

interface WorldViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
}

const tabs: Array<{ id: TabType; label: string; icon: React.ReactNode; emptyIcon: string; emptyText: string }> = [
  { id: "locations", label: "地点", icon: <MapPin size={15} />, emptyIcon: "山", emptyText: "山河未绘" },
  { id: "timeline", label: "时间线", icon: <Clock size={15} />, emptyIcon: "时", emptyText: "岁月未书" },
  { id: "rules", label: "设定", icon: <BookOpen size={15} />, emptyIcon: "法", emptyText: "法则未立" },
];

export function WorldView({ projectData, persistProjectData }: WorldViewProps) {
  const [tab, setTab] = useState<TabType>("locations");
  const [showForm, setShowForm] = useState(false);
  const [formName, setFormName] = useState("");
  const [formDesc, setFormDesc] = useState("");
  // 行内编辑
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editDesc, setEditDesc] = useState("");

  const world = projectData.world;

  // 优化/撤回：写回项目数据（函数式更新，组件卸载后仍生效）
  const applyWorld = useCallback((parsed: WorldData) => {
    if (!Array.isArray(parsed.locations) || !Array.isArray(parsed.timeline_events) || !Array.isArray(parsed.setting_rules)) return;
    persistProjectData(prev => ({ ...prev, world: parsed }));
  }, [persistProjectData]);

  const worldJson = useMemo(() => JSON.stringify(world), [world]);

  function handleAdd() {
    if (!formName.trim()) return;
    if (tab === "locations") {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, locations: [...prev.world.locations, { id: `loc-${Date.now()}`, name: formName.trim(), description: formDesc.trim() }] },
      }));
    } else if (tab === "timeline") {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, timeline_events: [...prev.world.timeline_events, { event_id: `evt-${Date.now()}`, story_time: formName.trim(), description: formDesc.trim() }] },
      }));
    } else {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, setting_rules: [...prev.world.setting_rules, { rule_id: `rule-${Date.now()}`, title: formName.trim(), description: formDesc.trim() }] },
      }));
    }
    setFormName(""); setFormDesc(""); setShowForm(false);
  }

  function handleDeleteItem(id: string) {
    if (tab === "locations") {
      persistProjectData(prev => ({ ...prev, world: { ...prev.world, locations: prev.world.locations.filter(l => l.id !== id) } }));
    } else if (tab === "timeline") {
      persistProjectData(prev => ({ ...prev, world: { ...prev.world, timeline_events: prev.world.timeline_events.filter(e => e.event_id !== id) } }));
    } else {
      persistProjectData(prev => ({ ...prev, world: { ...prev.world, setting_rules: prev.world.setting_rules.filter(r => r.rule_id !== id) } }));
    }
  }

  function startEdit(id: string, name: string, desc: string) {
    setEditingId(id);
    setEditName(name);
    setEditDesc(desc);
  }

  function handleSaveEdit() {
    if (!editingId || !editName.trim()) return;
    const name = editName.trim();
    const desc = editDesc.trim();
    if (tab === "locations") {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, locations: prev.world.locations.map(l => l.id === editingId ? { ...l, name, description: desc } : l) },
      }));
    } else if (tab === "timeline") {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, timeline_events: prev.world.timeline_events.map(e => e.event_id === editingId ? { ...e, story_time: name, description: desc } : e) },
      }));
    } else {
      persistProjectData(prev => ({
        ...prev,
        world: { ...prev.world, setting_rules: prev.world.setting_rules.map(r => r.rule_id === editingId ? { ...r, title: name, description: desc } : r) },
      }));
    }
    setEditingId(null);
  }

  // 优化：由全局优化管理器执行（见 OptimizeControls），跨页面切换不中断

  const currentTab = tabs.find(t => t.id === tab)!;
  const items = tab === "locations" ? world.locations : tab === "timeline" ? world.timeline_events : world.setting_rules;

  const renderItem = (id: string, title: string, desc: string, target: string, titleTag?: string) => {
    if (editingId === id) {
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 8, flex: 1 }}>
          <input className="pm-input" style={{ marginBottom: 0 }} value={editName} onChange={e => setEditName(e.target.value)} autoFocus />
          <textarea className="pm-textarea" style={{ marginBottom: 0 }} rows={2} value={editDesc} onChange={e => setEditDesc(e.target.value)} />
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-primary" onClick={handleSaveEdit} disabled={!editName.trim()}>保存</button>
            <button className="btn btn-secondary" onClick={() => setEditingId(null)}>取消</button>
          </div>
        </div>
      );
    }
    return (
      <>
        <div>
          {titleTag ? <span className="timeline-tag">{titleTag}</span> : <h4 className="detail-title">{title}</h4>}
          <p className="detail-desc">{desc}</p>
        </div>
        <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
          <EntityAnnotations target={target} />
          <button className="pv-icon-btn" onClick={() => startEdit(id, title, desc)} title="编辑"><Edit3 size={14} /></button>
          <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDeleteItem(id)} title="删除"><Trash2 size={14} /></button>
        </div>
      </>
    );
  };

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>世界观</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <OptimizeControls
            type="world"
            contentJson={worldJson}
            apply={applyWorld}
            disabled={world.locations.length + world.timeline_events.length + world.setting_rules.length === 0}
          />
          <button className="btn btn-primary" onClick={() => setShowForm(true)}><Plus size={15} /> 新增</button>
        </div>
      </div>
      <div className="tab-bar">
        {tabs.map(t => (
          <button key={t.id} onClick={() => { setTab(t.id); setShowForm(false); setEditingId(null); }} className={"tab-item" + (tab === t.id ? " active" : "")}>{t.icon} {t.label}</button>
        ))}
      </div>

      {showForm && (
        <div className="card" style={{ marginBottom: 16 }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input className="pm-input" style={{ marginBottom: 0 }} placeholder={tab === "timeline" ? "时间（如：第一年 春）" : "名称"} value={formName} onChange={e => setFormName(e.target.value)} autoFocus />
            <textarea className="pm-textarea" style={{ marginBottom: 0 }} placeholder="描述" value={formDesc} onChange={e => setFormDesc(e.target.value)} rows={2} />
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-primary" onClick={handleAdd}>添加</button>
              <button className="btn btn-secondary" onClick={() => { setShowForm(false); setFormName(""); setFormDesc(""); }}>取消</button>
            </div>
          </div>
        </div>
      )}

      {items.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon">{currentTab.emptyIcon}</div>
          <div className="empty-state-text">{currentTab.emptyText}</div>
          <div className="empty-state-sub">点击「新增」开始构建</div>
        </div>
      ) : (
        <div>
          {tab === "locations" && world.locations.map(loc => (
            <div key={loc.id} className="detail-item" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              {renderItem(loc.id, loc.name, loc.description, `location:${loc.id}`)}
            </div>
          ))}
          {tab === "timeline" && world.timeline_events.map(evt => (
            <div key={evt.event_id} className="timeline-item" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              {renderItem(evt.event_id, evt.story_time, evt.description, `timeline:${evt.event_id}`, evt.story_time)}
            </div>
          ))}
          {tab === "rules" && world.setting_rules.map(rule => (
            <div key={rule.rule_id} className="detail-item" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              {renderItem(rule.rule_id, rule.title, rule.description, `rule:${rule.rule_id}`)}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
