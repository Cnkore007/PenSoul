import { useState, useMemo } from "react";
import { MapPin, Clock, BookOpen, Plus, Trash2, Sparkles } from "lucide-react";
import type { ProjectData } from "../types";
import { InspirationPanel } from "../components/InspirationPanel";

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
  const [inspirationOpen, setInspirationOpen] = useState(false);

  const world = projectData.world;

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

  const currentTab = tabs.find(t => t.id === tab)!;
  const items = tab === "locations" ? world.locations : tab === "timeline" ? world.timeline_events : world.setting_rules;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>世界观</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            className="btn btn-ghost"
            onClick={() => setInspirationOpen(!inspirationOpen)}
            title="AI 灵感"
            style={{ color: inspirationOpen ? "var(--color-accent)" : undefined }}
          >
            <Sparkles size={15} /> 灵感
          </button>
          <button className="btn btn-primary" onClick={() => setShowForm(true)}><Plus size={15} /> 新增</button>
        </div>
      </div>
      <div className="tab-bar">
        {tabs.map(t => (
          <button key={t.id} onClick={() => { setTab(t.id); setShowForm(false); }} className={"tab-item" + (tab === t.id ? " active" : "")}>{t.icon} {t.label}</button>
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
              <div><h4 className="detail-title">{loc.name}</h4><p className="detail-desc">{loc.description}</p></div>
              <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDeleteItem(loc.id)}><Trash2 size={14} /></button>
            </div>
          ))}
          {tab === "timeline" && world.timeline_events.map(evt => (
            <div key={evt.event_id} className="timeline-item" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              <div><span className="timeline-tag">{evt.story_time}</span><p className="detail-desc">{evt.description}</p></div>
              <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDeleteItem(evt.event_id)}><Trash2 size={14} /></button>
            </div>
          ))}
          {tab === "rules" && world.setting_rules.map(rule => (
            <div key={rule.rule_id} className="detail-item" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              <div><h4 className="detail-title">{rule.title}</h4><p className="detail-desc">{rule.description}</p></div>
              <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => handleDeleteItem(rule.rule_id)}><Trash2 size={14} /></button>
            </div>
          ))}
        </div>
      )}
      <InspirationPanel
        contextType="world"
        contextData={useMemo(() => JSON.stringify({
          locations: projectData.world.locations,
          timeline_events: projectData.world.timeline_events,
          setting_rules: projectData.world.setting_rules,
        }), [projectData.world])}
        externalExpanded={inspirationOpen}
        onToggle={() => setInspirationOpen(!inspirationOpen)}
        hideTrigger={true}
      />
    </div>
  );
}
