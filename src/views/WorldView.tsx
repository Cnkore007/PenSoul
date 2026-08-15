// WorldView — 世界（地点/时间线/伏笔/规则）

import { useState, useEffect, useCallback } from "react";
import {
  listLocations,
  listTimeline,
  listForeshadows,
  listRules,
  addSetting,
  addEvent,
  addForeshadow,
  addRule,
  updateSetting,
  updateEvent,
  updateForeshadow,
  updateRule,
  deleteSetting,
  deleteEvent,
  deleteForeshadow,
  deleteRule,
} from "../ipc";
import type { Location, TimelineEvent, Foreshadow } from "../types";
import { foreshadowStatusLabels, label } from "../labels";

type Tab = "locations" | "timeline" | "foreshadows" | "rules";

export default function WorldView() {
  const [locations, setLocations] = useState<Location[]>([]);
  const [timeline, setTimeline] = useState<TimelineEvent[]>([]);
  const [foreshadows, setForeshadows] = useState<Foreshadow[]>([]);
  const [rules, setRules] = useState<string[]>([]);
  const [msg, setMsg] = useState("");
  const [tab, setTab] = useState<Tab>("locations");

  const [newLocName, setNewLocName] = useState("");
  const [newLocCat, setNewLocCat] = useState("");
  const [newEvtName, setNewEvtName] = useState("");
  const [newEvtCh, setNewEvtCh] = useState("1");
  const [newFsName, setNewFsName] = useState("");
  const [newFsCh, setNewFsCh] = useState("1");
  const [newRule, setNewRule] = useState("");

  const [editId, setEditId] = useState<string | null>(null);
  const [editData, setEditData] = useState<any>({});
  const [editRuleIdx, setEditRuleIdx] = useState<number | null>(null);
  const [editRuleVal, setEditRuleVal] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [l, t, f, r] = await Promise.all([listLocations(), listTimeline(), listForeshadows(), listRules()]);
      setLocations(l); setTimeline(t); setForeshadows(f); setRules(r);
    } catch { setMsg("加载失败"); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleAddLocation = async () => {
    if (!newLocName.trim()) return;
    try {
      await addSetting(newLocName.trim(), newLocCat.trim() || "地点");
      setNewLocName(""); setNewLocCat(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleAddEvent = async () => {
    if (!newEvtName.trim()) return;
    try {
      await addEvent(newEvtName.trim(), parseInt(newEvtCh) || 1);
      setNewEvtName(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleAddForeshadow = async () => {
    if (!newFsName.trim()) return;
    try {
      await addForeshadow(newFsName.trim(), parseInt(newFsCh) || 1);
      setNewFsName(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleAddRule = async () => {
    if (!newRule.trim()) return;
    try {
      await addRule(newRule.trim());
      setNewRule(""); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteLocation = async (id: string) => {
    if (!confirm("确定删除此地点？")) return;
    try { await deleteSetting(id); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteEvent = async (id: string) => {
    if (!confirm("确定删除此事件？")) return;
    try { await deleteEvent(id); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteForeshadow = async (id: string) => {
    if (!confirm("确定删除此伏笔？")) return;
    try { await deleteForeshadow(id); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleDeleteRule = async (idx: number) => {
    if (!confirm("确定删除此规则？")) return;
    try { await deleteRule(idx); refresh(); } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveLocation = async () => {
    if (!editId) return;
    try {
      await updateSetting(editId, editData);
      setEditId(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveEvent = async () => {
    if (!editId) return;
    try {
      await updateEvent(editId, editData);
      setEditId(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveForeshadow = async () => {
    if (!editId) return;
    try {
      await updateForeshadow(editId, editData);
      setEditId(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleSaveRule = async () => {
    if (editRuleIdx === null) return;
    try {
      await updateRule(editRuleIdx, editRuleVal);
      setEditRuleIdx(null); refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  return (
    <div className="view-card">
      <h2>世界</h2>
      {msg && <p className="msg">{msg}</p>}

      <div className="tab-bar">
        {(["locations", "timeline", "foreshadows", "rules"] as const).map((t) => (
          <button key={t} className={`tab-btn ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {{ locations: "地点", timeline: "时间线", foreshadows: "伏笔", rules: "规则" }[t]}
          </button>
        ))}
      </div>

      {tab === "locations" && (
        <div>
          <div className="form-row">
            <input className="ps-input" placeholder="地点名" value={newLocName}
              onChange={(e) => setNewLocName(e.target.value)} />
            <input className="ps-input ps-input-sm" placeholder="类别" value={newLocCat}
              onChange={(e) => setNewLocCat(e.target.value)} />
            <button className="btn-primary" onClick={handleAddLocation}>添加</button>
          </div>
          <div className="entity-grid">
            {locations.map((l) => (
              <div key={l.id} className={`entity-card type-setting ${editId === l.id ? "editing" : ""}`}>
                {editId === l.id ? (
                  <div className="edit-form-inline">
                    <input className="ps-input" value={editData.name || ""} onChange={e => setEditData({...editData, name: e.target.value})} placeholder="名称" />
                    <input className="ps-input" value={editData.category || ""} onChange={e => setEditData({...editData, category: e.target.value})} placeholder="类别" />
                    <input className="ps-input" value={editData.description || ""} onChange={e => setEditData({...editData, description: e.target.value})} placeholder="描述" />
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleSaveLocation}>保存</button>
                      <button className="btn-sm" onClick={() => setEditId(null)}>取消</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <span className="entity-type">{l.category || "地点"}</span>
                    <span className="entity-name">{l.name}</span>
                    {l.description && <span className="entity-detail">{l.description}</span>}
                    <div className="btn-group">
                      <button className="btn-sm" onClick={() => { setEditId(l.id); setEditData({name: l.name, category: l.category, description: l.description}); }}>编辑</button>
                      <button className="btn-sm btn-danger" onClick={() => handleDeleteLocation(l.id)}>删除</button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
          {locations.length === 0 && <p className="empty">暂无地点。</p>}
        </div>
      )}

      {tab === "timeline" && (
        <div>
          <div className="form-row">
            <input className="ps-input" placeholder="事件名" value={newEvtName}
              onChange={(e) => setNewEvtName(e.target.value)} />
            <input className="ps-input ps-input-sm" placeholder="章节" value={newEvtCh}
              onChange={(e) => setNewEvtCh(e.target.value)} />
            <button className="btn-primary" onClick={handleAddEvent}>添加</button>
          </div>
          <div className="timeline-list">
            {timeline.map((e) => (
              <div key={e.id} className={`timeline-item ${editId === e.id ? "editing" : ""}`}>
                {editId === e.id ? (
                  <div className="edit-form-inline">
                    <input className="ps-input" value={editData.name || ""} onChange={ev => setEditData({...editData, name: ev.target.value})} placeholder="名称" />
                    <input className="ps-input ps-input-sm" value={editData.chapter_id || ""} onChange={ev => setEditData({...editData, chapter_id: parseInt(ev.target.value) || 1})} placeholder="章节" />
                    <input className="ps-input" value={editData.description || ""} onChange={ev => setEditData({...editData, description: ev.target.value})} placeholder="描述" />
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleSaveEvent}>保存</button>
                      <button className="btn-sm" onClick={() => setEditId(null)}>取消</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <span className="timeline-chapter">第{e.chapter_id}章</span>
                    <span className="timeline-name">{e.name}</span>
                    {e.description && <span className="timeline-desc">{e.description}</span>}
                    <div className="btn-group">
                      <button className="btn-sm" onClick={() => { setEditId(e.id); setEditData({name: e.name, chapter_id: e.chapter_id, description: e.description}); }}>编辑</button>
                      <button className="btn-sm btn-danger" onClick={() => handleDeleteEvent(e.id)}>删除</button>
                    </div>
                  </>
                )}
              </div>
            ))}
            {timeline.length === 0 && <p className="empty">暂无事件。</p>}
          </div>
        </div>
      )}

      {tab === "foreshadows" && (
        <div>
          <div className="form-row">
            <input className="ps-input" placeholder="伏笔名" value={newFsName}
              onChange={(e) => setNewFsName(e.target.value)} />
            <input className="ps-input ps-input-sm" placeholder="埋伏章" value={newFsCh}
              onChange={(e) => setNewFsCh(e.target.value)} />
            <button className="btn-primary" onClick={handleAddForeshadow}>添加</button>
          </div>
          <div className="entity-grid">
            {foreshadows.map((f) => (
              <div key={f.id} className={`entity-card type-foreshadow ${editId === f.id ? "editing" : ""}`}>
                {editId === f.id ? (
                  <div className="edit-form-inline">
                    <input className="ps-input" value={editData.name || ""} onChange={e => setEditData({...editData, name: e.target.value})} placeholder="名称" />
                    <select className="ps-input" value={editData.status || ""} onChange={e => setEditData({...editData, status: e.target.value})}>
                      <option value="Planned">待埋（Planned）</option>
                      <option value="Planted">已埋（Planted）</option>
                      <option value="Progressing">推进中（Progressing）</option>
                      <option value="Resolved">已回收（Resolved）</option>
                      <option value="Abandoned">已废弃（Abandoned）</option>
                      <option value="Overdue">已逾期（Overdue）</option>
                    </select>
                    <input className="ps-input" value={editData.planted_chapter || ""} onChange={e => setEditData({...editData, planted_chapter: parseInt(e.target.value) || 1})} placeholder="埋伏章" />
                    <input className="ps-input" value={editData.expected_payoff || ""} onChange={e => setEditData({...editData, expected_payoff: e.target.value ? parseInt(e.target.value) : null})} placeholder="计划回收章（可空）" />
                    <input className="ps-input" value={editData.actual_payoff || ""} onChange={e => setEditData({...editData, actual_payoff: e.target.value ? parseInt(e.target.value) : null})} placeholder="实际回收章（可空）" />
                    <input className="ps-input" value={editData.description || ""} onChange={e => setEditData({...editData, description: e.target.value})} placeholder="描述" />
                    <div className="btn-group">
                      <button className="btn-sm" onClick={handleSaveForeshadow}>保存</button>
                      <button className="btn-sm" onClick={() => setEditId(null)}>取消</button>
                    </div>
                  </div>
                ) : (
                  <>
                    <span className="entity-type">{label(foreshadowStatusLabels, f.status)}</span>
                    <span className="entity-name">{f.name}</span>
                    <span className="entity-detail">埋于第{f.planted_chapter}章</span>
                    <div className="btn-group">
                      <button className="btn-sm" onClick={() => { setEditId(f.id); setEditData({name: f.name, status: f.status, description: f.description, planted_chapter: f.planted_chapter, expected_payoff: f.expected_payoff, actual_payoff: f.actual_payoff}); }}>编辑</button>
                      <button className="btn-sm btn-danger" onClick={() => handleDeleteForeshadow(f.id)}>删除</button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
          {foreshadows.length === 0 && <p className="empty">暂无伏笔。</p>}
        </div>
      )}

      {tab === "rules" && (
        <div>
          <div className="form-row">
            <input className="ps-input" placeholder="新规则" value={newRule}
              onChange={(e) => setNewRule(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddRule()} />
            <button className="btn-primary" onClick={handleAddRule}>添加</button>
          </div>
          {rules.length > 0 ? (
            <ul className="rules-list">
              {rules.map((r, i) => (
                <li key={i}>
                  {editRuleIdx === i ? (
                    <div className="edit-form-inline">
                      <input className="ps-input" value={editRuleVal} onChange={e => setEditRuleVal(e.target.value)} />
                      <button className="btn-sm" onClick={handleSaveRule}>保存</button>
                      <button className="btn-sm" onClick={() => setEditRuleIdx(null)}>取消</button>
                    </div>
                  ) : (
                    <>
                      <span className="rule-content">{r}</span>
                      <div className="btn-group">
                        <button className="btn-sm" onClick={() => { setEditRuleIdx(i); setEditRuleVal(r); }}>编辑</button>
                        <button className="btn-sm btn-danger" onClick={() => handleDeleteRule(i)}>删除</button>
                      </div>
                    </>
                  )}
                </li>
              ))}
            </ul>
          ) : (
            <p className="empty">暂无世界观规则。</p>
          )}
        </div>
      )}
    </div>
  );
}
