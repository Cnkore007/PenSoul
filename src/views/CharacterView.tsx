// CharacterView — 人物

import { useState, useEffect, useCallback } from "react";
import {
  listCharacters,
  addCharacter,
  updateCharacter,
  deleteCharacter,
} from "../ipc";
import type { Character } from "../types";
import { entityTypeLabels, label } from "../labels";

export default function CharacterView() {
  const [characters, setCharacters] = useState<Character[]>([]);
  const [newName, setNewName] = useState("");
  const [msg, setMsg] = useState("");
  const [selected, setSelected] = useState<Character | null>(null);
  const [editing, setEditing] = useState(false);
  const [editData, setEditData] = useState<Partial<Character>>({});

  const refresh = useCallback(async () => {
    try { setCharacters(await listCharacters()); } catch { setMsg("加载失败"); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const handleAdd = async () => {
    if (!newName.trim()) return;
    try { await addCharacter(newName.trim()); setNewName(""); refresh(); }
    catch (e: any) { setMsg(`${e}`); }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("确定删除此角色？")) return;
    try {
      await deleteCharacter(id);
      setSelected(null); setEditing(false);
      refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  const handleEdit = (c: Character) => {
    setSelected(c);
    setEditData({
      name: c.name, age: c.age, occupation: c.occupation,
      appearance: c.appearance, backstory: c.backstory,
      wants: c.wants, fears: c.fears, secret: c.secret,
    });
    setEditing(true);
  };

  const handleSave = async () => {
    if (!selected) return;
    try {
      await updateCharacter(selected.id, editData);
      setEditing(false);
      refresh();
    } catch (e: any) { setMsg(`${e}`); }
  };

  return (
    <div className="view-card">
      <h2>人物</h2>
      <div className="form-row">
        <input className="ps-input" placeholder="角色名" value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleAdd()} />
        <button className="btn-primary" onClick={handleAdd}>添加</button>
      </div>
      {msg && <p className="msg">{msg}</p>}

      <div className="entity-grid">
        {characters.map((c) => (
          <div key={c.id} className={`entity-card type-character ${selected?.id === c.id ? "selected" : ""}`}
            onClick={() => { if (!editing) setSelected(selected?.id === c.id ? null : c); }}>
            <span className="entity-type">{label(entityTypeLabels, "Character")}</span>
            <span className="entity-name">{c.name}</span>
            {c.occupation && <span className="entity-detail">{c.occupation}</span>}
          </div>
        ))}
      </div>

      {characters.length === 0 && <p className="empty">暂无角色。</p>}

      {selected && !editing && (
        <div className="detail-panel">
          <div className="detail-header">
            <h3>{selected.name}</h3>
            <div className="btn-group">
              <button className="btn-sm" onClick={() => handleEdit(selected)}>编辑</button>
              <button className="btn-sm btn-danger" onClick={() => handleDelete(selected.id)}>删除</button>
            </div>
          </div>
          <dl className="detail-list">
            {selected.age != null && <><dt>年龄</dt><dd>{selected.age}</dd></>}
            {selected.occupation && <><dt>职业</dt><dd>{selected.occupation}</dd></>}
            {selected.appearance && <><dt>外貌</dt><dd>{selected.appearance}</dd></>}
            {selected.backstory && <><dt>背景</dt><dd>{selected.backstory}</dd></>}
            {selected.wants && <><dt>渴望</dt><dd>{selected.wants}</dd></>}
            {selected.fears && <><dt>恐惧</dt><dd>{selected.fears}</dd></>}
            {selected.secret && <><dt>秘密</dt><dd>{selected.secret}</dd></>}
          </dl>
        </div>
      )}

      {selected && editing && (
        <div className="detail-panel">
          <div className="detail-header">
            <h3>编辑角色</h3>
            <div className="btn-group">
              <button className="btn-primary btn-sm" onClick={handleSave}>保存</button>
              <button className="btn-sm" onClick={() => setEditing(false)}>取消</button>
            </div>
          </div>
          <div className="edit-form">
            <label>名称<input className="ps-input" value={editData.name || ""} onChange={e => setEditData({...editData, name: e.target.value})} /></label>
            <label>年龄<input className="ps-input" type="number" value={editData.age ?? ""} onChange={e => setEditData({...editData, age: e.target.value ? parseInt(e.target.value) : null})} /></label>
            <label>职业<input className="ps-input" value={editData.occupation || ""} onChange={e => setEditData({...editData, occupation: e.target.value})} /></label>
            <label>外貌<input className="ps-input" value={editData.appearance || ""} onChange={e => setEditData({...editData, appearance: e.target.value})} /></label>
            <label>背景<textarea className="ps-input ps-textarea" value={editData.backstory || ""} onChange={e => setEditData({...editData, backstory: e.target.value})} /></label>
            <label>渴望<input className="ps-input" value={editData.wants || ""} onChange={e => setEditData({...editData, wants: e.target.value})} /></label>
            <label>恐惧<input className="ps-input" value={editData.fears || ""} onChange={e => setEditData({...editData, fears: e.target.value})} /></label>
            <label>秘密<input className="ps-input" value={editData.secret || ""} onChange={e => setEditData({...editData, secret: e.target.value})} /></label>
          </div>
        </div>
      )}
    </div>
  );
}
