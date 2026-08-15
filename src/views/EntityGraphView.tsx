// EntityGraphView — 图谱 = 整本书知识库（P0 档案化）
// 档案类型筛选（人物/组织/设定/事件/伏笔/规则） + 搜索 + 详情编辑
// 人物档案：外貌/衣着/功法/境界/法宝 等；组织档案：结构/目标/规则/成员；
// 设定/事件/伏笔/规则均支持详情编辑（P1-1 修复：全部档案类型可维护）

import { useState, useEffect, useCallback } from "react";
import {
  listEntities,
  listCharacters,
  listOrganizations,
  listLocations,
  listTimeline,
  listForeshadows,
  listRules,
  getGraphStats,
  addCharacter,
  addOrganization,
  addEvent,
  addSetting,
  addForeshadow,
  addRule,
  updateCharacter,
  updateOrganization,
  updateSetting,
  updateEvent,
  updateForeshadow,
  updateRule,
  deleteCharacter,
  deleteOrganization,
  deleteEvent,
  deleteSetting,
  deleteForeshadow,
  deleteRule,
} from "../ipc";
import type {
  EntitySummary,
  GraphStats,
  Character,
  Organization,
  Location,
  TimelineEvent,
  Foreshadow,
  Chapter,
} from "../types";
import { listChapters } from "../ipc";
import { entityTypeLabels, label } from "../labels";

type Tab = "all" | "Character" | "Organization" | "Setting" | "Event" | "Foreshadow" | "Rule";

const TABS: [Tab, string][] = [
  ["all", "全部"],
  ["Character", "人物"],
  ["Organization", "组织"],
  ["Setting", "设定"],
  ["Event", "事件"],
  ["Foreshadow", "伏笔"],
  ["Rule", "规则"],
];

const FORESHADOW_STATUSES = ["Planned", "Planted", "Progressing", "Resolved", "Abandoned", "Overdue"];

export default function EntityGraphView() {
  const [entities, setEntities] = useState<EntitySummary[]>([]);
  const [stats, setStats] = useState<GraphStats | null>(null);
  const [tab, setTab] = useState<Tab>("all");
  const [keyword, setKeyword] = useState("");
  const [msg, setMsg] = useState("");

  // 详情缓存
  const [characters, setCharacters] = useState<Character[]>([]);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [settings, setSettings] = useState<Location[]>([]);
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [foreshadows, setForeshadows] = useState<Foreshadow[]>([]);
  const [rules, setRules] = useState<string[]>([]);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [selected, setSelected] = useState<EntitySummary | null>(null);
  // 规则无 ID，用索引选中
  const [selectedRuleIdx, setSelectedRuleIdx] = useState<number | null>(null);

  // 编辑表单
  const [form, setForm] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  // 添加表单
  const [newName, setNewName] = useState("");
  const [newCategory, setNewCategory] = useState("");
  // 事件/伏笔需要指定章号（P1-7：禁止硬编码挂第 1 章）
  const [newChapter, setNewChapter] = useState("");
  // 新增规则
  const [newRule, setNewRule] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [e, s] = await Promise.all([listEntities(), getGraphStats()]);
      setEntities(e);
      setStats(s);
    } catch {
      setMsg("加载失败");
    }
    listCharacters().then(setCharacters).catch((err) => setMsg(`人物档案加载失败: ${err}`));
    listOrganizations().then(setOrganizations).catch((err) => setMsg(`组织档案加载失败: ${err}`));
    listLocations().then(setSettings).catch((err) => setMsg(`设定档案加载失败: ${err}`));
    listTimeline().then(setEvents).catch((err) => setMsg(`事件档案加载失败: ${err}`));
    listForeshadows().then(setForeshadows).catch((err) => setMsg(`伏笔档案加载失败: ${err}`));
    listRules().then(setRules).catch((err) => setMsg(`规则加载失败: ${err}`));
    listChapters()
      .then((cs) => {
        setChapters(cs);
        // 默认章号 = 当前最大章号
        const max = cs.reduce((m, c) => Math.max(m, c.chapter_no), 0);
        setNewChapter(max > 0 ? String(max) : "");
      })
      .catch((err) => setMsg(`章节列表加载失败: ${err}`));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // 筛选 + 搜索（规则单独渲染，不走实体列表）
  const visible = entities.filter((e) => {
    if (tab !== "all" && e.type !== tab) return false;
    if (keyword && !e.name.includes(keyword)) return false;
    return true;
  });

  const handleSelect = (e: EntitySummary) => {
    setSelected(e);
    setSelectedRuleIdx(null);
    setForm({});
    if (e.type === "Character") {
      const c = characters.find((x) => x.id === e.id);
      if (c) {
        setForm({
          name: c.name,
          age: c.age != null ? String(c.age) : "",
          occupation: c.occupation || "",
          appearance: c.appearance || "",
          attire: c.attire || "",
          techniques: (c.techniques || []).join(","),
          realm: c.realm || "",
          items: (c.items || []).join(","),
          wants: c.wants || "",
          fears: c.fears || "",
          secret: c.secret || "",
          backstory: c.backstory || "",
        });
      }
    } else if (e.type === "Organization") {
      const o = organizations.find((x) => x.id === e.id);
      if (o) {
        setForm({
          name: o.name,
          category: o.category,
          structure: o.structure,
          goals: o.goals,
          rules: (o.rules || []).join(","),
          description: o.description,
        });
      }
    } else if (e.type === "Setting") {
      const s = settings.find((x) => x.id === e.id);
      if (s) {
        setForm({
          name: s.name,
          category: s.category,
          description: s.description,
        });
      }
    } else if (e.type === "Event") {
      const ev = events.find((x) => x.id === e.id);
      if (ev) {
        setForm({
          name: ev.name,
          chapter_id: String(ev.chapter_id),
          description: ev.description,
        });
      }
    } else if (e.type === "Foreshadow") {
      const f = foreshadows.find((x) => x.id === e.id);
      if (f) {
        setForm({
          name: f.name,
          description: f.description,
          status: f.status,
          planted_chapter: String(f.planted_chapter),
          expected_payoff: f.expected_payoff != null ? String(f.expected_payoff) : "",
          actual_payoff: f.actual_payoff != null ? String(f.actual_payoff) : "",
        });
      }
    }
  };

  const handleSave = async () => {
    if (!selected) return;
    setSaving(true);
    try {
      if (selected.type === "Character") {
        // 空串传给后端 = 清空字段（P2-10 修复）
        await updateCharacter(selected.id, {
          name: form.name || undefined,
          age: form.age ? Number(form.age) : null,
          occupation: form.occupation || "",
          appearance: form.appearance || "",
          attire: form.attire || "",
          techniques: form.techniques ? form.techniques.split(",").map((s) => s.trim()).filter(Boolean) : [],
          realm: form.realm || "",
          items: form.items ? form.items.split(",").map((s) => s.trim()).filter(Boolean) : [],
          wants: form.wants || "",
          fears: form.fears || "",
          secret: form.secret || "",
          backstory: form.backstory || "",
        });
      } else if (selected.type === "Organization") {
        await updateOrganization(selected.id, {
          name: form.name || undefined,
          category: form.category || "",
          structure: form.structure || "",
          goals: form.goals || "",
          rules: form.rules ? form.rules.split(",").map((s) => s.trim()).filter(Boolean) : [],
          description: form.description || "",
        });
      } else if (selected.type === "Setting") {
        await updateSetting(selected.id, {
          name: form.name || undefined,
          category: form.category || "",
          description: form.description || "",
        });
      } else if (selected.type === "Event") {
        await updateEvent(selected.id, {
          name: form.name || undefined,
          chapter_id: form.chapter_id ? Number(form.chapter_id) : undefined,
          description: form.description || "",
        });
      } else if (selected.type === "Foreshadow") {
        await updateForeshadow(selected.id, {
          name: form.name || undefined,
          description: form.description || "",
          status: form.status || undefined,
          planted_chapter: form.planted_chapter ? Number(form.planted_chapter) : undefined,
          expected_payoff: form.expected_payoff ? Number(form.expected_payoff) : null,
          actual_payoff: form.actual_payoff ? Number(form.actual_payoff) : null,
        });
      }
      setMsg("档案已保存到正典");
      refresh();
    } catch (e: any) {
      setMsg(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!selected) return;
    if (!confirm(`确认删除档案「${selected.name}」？此操作不可撤销。`)) return;
    try {
      const fn: Record<string, (id: string) => Promise<void>> = {
        Character: deleteCharacter,
        Organization: deleteOrganization,
        Event: deleteEvent,
        Setting: deleteSetting,
        Foreshadow: deleteForeshadow,
      };
      await fn[selected.type](selected.id);
      setSelected(null);
      setMsg("档案已删除");
      refresh();
    } catch (e: any) {
      setMsg(`删除失败: ${e}`);
    }
  };

  const handleAdd = async () => {
    if (tab === "Rule") {
      if (!newRule.trim()) {
        setMsg("请输入规则内容");
        return;
      }
      try {
        await addRule(newRule.trim());
        setNewRule("");
        setMsg("已添加规则");
        refresh();
      } catch (e: any) {
        setMsg(`添加规则失败: ${e}`);
      }
      return;
    }
    if (!newName.trim()) {
      setMsg("请输入名称");
      return;
    }
    try {
      if (tab === "Character") await addCharacter(newName.trim());
      else if (tab === "Organization") await addOrganization(newName.trim(), newCategory.trim());
      else if (tab === "Event") {
        // 章号必填且必须存在（后端也校验）
        const no = Number(newChapter);
        if (!no || no < 1) {
          setMsg("请填写有效的章节号（事件发生的章节）");
          return;
        }
        await addEvent(newName.trim(), no);
      } else if (tab === "Setting") await addSetting(newName.trim(), newCategory.trim() || "地点");
      else if (tab === "Foreshadow") {
        const no = Number(newChapter);
        if (!no || no < 1) {
          setMsg("请填写有效的埋设章节号");
          return;
        }
        await addForeshadow(newName.trim(), no);
      }
      setNewName("");
      setNewCategory("");
      setNewChapter("");
      setMsg("已添加档案");
      refresh();
    } catch (e: any) {
      setMsg(`添加失败: ${e}`);
    }
  };

  const handleRuleSelect = (idx: number) => {
    setSelectedRuleIdx(idx);
    setSelected(null);
    setForm({ rule: rules[idx] || "" });
  };

  const handleRuleSave = async () => {
    if (selectedRuleIdx == null) return;
    const content = form.rule || "";
    if (!content.trim()) {
      setMsg("规则内容不能为空");
      return;
    }
    try {
      await updateRule(selectedRuleIdx, content.trim());
      setMsg("规则已更新");
      refresh();
    } catch (e: any) {
      setMsg(`规则更新失败: ${e}`);
    }
  };

  const handleRuleDelete = async () => {
    if (selectedRuleIdx == null) return;
    if (!confirm(`确认删除规则「${rules[selectedRuleIdx]?.slice(0, 20)}…」？`)) return;
    try {
      await deleteRule(selectedRuleIdx);
      setSelectedRuleIdx(null);
      setMsg("规则已删除");
      refresh();
    } catch (e: any) {
      setMsg(`规则删除失败: ${e}`);
    }
  };

  const input = (key: string) => (
    <input
      className="ps-input"
      value={form[key] || ""}
      onChange={(e) => setForm({ ...form, [key]: e.target.value })}
    />
  );

  const textarea = (key: string, rows = 2) => (
    <textarea
      className="ps-input ps-textarea"
      rows={rows}
      value={form[key] || ""}
      onChange={(e) => setForm({ ...form, [key]: e.target.value })}
    />
  );

  const chapterOptions = (
    <select
      className="ps-input"
      value={form.chapter_id || form.planted_chapter || ""}
      onChange={(e) => setForm({ ...form, [selected?.type === "Event" ? "chapter_id" : "planted_chapter"]: e.target.value })}
    >
      <option value="">选择章节…</option>
      {chapters.map((c) => (
        <option key={c.chapter_id} value={String(c.chapter_no)}>
          第 {c.chapter_no} 章 · {c.title}
        </option>
      ))}
    </select>
  );

  return (
    <div className="view-card">
      <h2>图谱 · 知识库</h2>
      <p className="empty" style={{ marginTop: "-0.5rem", textAlign: "left" }}>
        整本书的档案库：人物 / 组织 / 设定 / 事件 / 伏笔 / 规则，随小说推进自动更新。
      </p>
      {msg && <p className="msg">{msg}</p>}
      {stats && (
        <div className="stats-row">
          <span>实体总数: {stats.total_entities}</span>
          <span>关系: {stats.total_relations}</span>
        </div>
      )}

      <div className="graph-toolbar">
        <div className="tab-bar">
          {TABS.map(([key, l]) => (
            <button
              key={key}
              className={`tab-btn ${tab === key ? "active" : ""}`}
              onClick={() => {
                setTab(key);
                setSelected(null);
                setSelectedRuleIdx(null);
              }}
            >
              {l}
            </button>
          ))}
        </div>
        <input
          className="ps-input graph-search"
          placeholder="搜索档案..."
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
        />
      </div>

      {tab !== "all" && tab !== "Rule" && (
        <div className="form-row graph-add">
          <input
            className="ps-input"
            placeholder={tab === "Character" ? "人物名" : tab === "Organization" ? "组织名" : tab === "Setting" ? "设定名" : tab === "Event" ? "事件名" : "伏笔名"}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          {(tab === "Organization" || tab === "Setting") && (
            <input
              className="ps-input"
              placeholder={tab === "Organization" ? "势力类型（宗门/家族/帝国…）" : "类别（地点/体系/功法…）"}
              value={newCategory}
              onChange={(e) => setNewCategory(e.target.value)}
            />
          )}
          {(tab === "Event" || tab === "Foreshadow") && (
            <select
              className="ps-input"
              value={newChapter}
              onChange={(e) => setNewChapter(e.target.value)}
            >
              <option value="">选择章节…</option>
              {chapters.map((c) => (
                <option key={c.chapter_id} value={String(c.chapter_no)}>
                  第 {c.chapter_no} 章 · {c.title}
                </option>
              ))}
            </select>
          )}
          <button className="btn-primary btn-sm" onClick={handleAdd}>
            + 添加{label(entityTypeLabels, tab as any)}
          </button>
        </div>
      )}

      {tab === "Rule" && (
        <div className="form-row graph-add">
          <input
            className="ps-input"
            placeholder="新世界观规则（如：灵气浓度随境界提升而下降）"
            value={newRule}
            onChange={(e) => setNewRule(e.target.value)}
          />
          <button className="btn-primary btn-sm" onClick={handleAdd}>
            + 添加规则
          </button>
        </div>
      )}

      <div className="graph-layout">
        <div className="graph-list">
          {tab === "Rule" ? (
            rules.length > 0 ? (
              <ul className="entity-nav">
                {rules.map((r, idx) => (
                  <li
                    key={idx}
                    className={`entity-nav-item ${selectedRuleIdx === idx ? "active" : ""}`}
                    onClick={() => handleRuleSelect(idx)}
                  >
                    <span className="entity-nav-name">{r.length > 24 ? `${r.slice(0, 24)}…` : r}</span>
                    <span className="entity-nav-type">规则</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="empty">暂无规则，使用上方表单添加。</p>
            )
          ) : visible.length > 0 ? (
            <ul className="entity-nav">
              {visible.map((e) => (
                <li
                  key={`${e.type}-${e.id}`}
                  className={`entity-nav-item ${selected?.id === e.id && selected?.type === e.type ? "active" : ""}`}
                  onClick={() => handleSelect(e)}
                >
                  <span className="entity-nav-name">{e.name}</span>
                  <span className="entity-nav-type">{label(entityTypeLabels, e.type)}</span>
                </li>
              ))}
            </ul>
          ) : (
            <p className="empty">暂无档案，使用上方表单添加。</p>
          )}
        </div>

        <div className="graph-detail">
          {!selected && selectedRuleIdx == null && <p className="empty">选择一个档案查看与编辑详情。</p>}

          {selected?.type === "Character" && (
            <div className="section">
              <h3>人物档案</h3>
              <div className="form-grid">
                <label className="llm-field">姓名{input("name")}</label>
                <label className="llm-field">年龄{input("age")}</label>
                <label className="llm-field">职业{input("occupation")}</label>
                <label className="llm-field">外貌{textarea("appearance")}</label>
                <label className="llm-field">衣着{textarea("attire")}</label>
                <label className="llm-field">功法（逗号分隔）{input("techniques")}</label>
                <label className="llm-field">境界{input("realm")}</label>
                <label className="llm-field">法宝（逗号分隔）{input("items")}</label>
                <label className="llm-field">欲望{input("wants")}</label>
                <label className="llm-field">恐惧{input("fears")}</label>
                <label className="llm-field">秘密{input("secret")}</label>
                <label className="llm-field">背景{textarea("backstory", 3)}</label>
              </div>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleSave} disabled={saving}>
                  {saving ? "保存中..." : "保存档案"}
                </button>
                <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
              </div>
              <p className="llm-hint">留空字段并保存 = 清空该字段。</p>
            </div>
          )}

          {selected?.type === "Organization" && (
            <div className="section">
              <h3>组织档案</h3>
              <div className="form-grid">
                <label className="llm-field">名称{input("name")}</label>
                <label className="llm-field">势力类型{input("category")}</label>
                <label className="llm-field">等级结构{textarea("structure")}</label>
                <label className="llm-field">目标{textarea("goals")}</label>
                <label className="llm-field">规则（逗号分隔）{input("rules")}</label>
                <label className="llm-field">描述{textarea("description", 3)}</label>
              </div>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleSave} disabled={saving}>
                  {saving ? "保存中..." : "保存档案"}
                </button>
                <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
              </div>
            </div>
          )}

          {selected?.type === "Setting" && (
            <div className="section">
              <h3>设定档案</h3>
              <div className="form-grid">
                <label className="llm-field">名称{input("name")}</label>
                <label className="llm-field">类别（地点/体系/功法/法宝/境界）{input("category")}</label>
                <label className="llm-field">描述/规则{textarea("description", 4)}</label>
              </div>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleSave} disabled={saving}>
                  {saving ? "保存中..." : "保存档案"}
                </button>
                <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
              </div>
            </div>
          )}

          {selected?.type === "Event" && (
            <div className="section">
              <h3>事件档案</h3>
              <div className="form-grid">
                <label className="llm-field">名称{input("name")}</label>
                <label className="llm-field">发生章节{chapterOptions}</label>
                <label className="llm-field">描述{textarea("description", 4)}</label>
              </div>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleSave} disabled={saving}>
                  {saving ? "保存中..." : "保存档案"}
                </button>
                <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
              </div>
            </div>
          )}

          {selected?.type === "Foreshadow" && (
            <div className="section">
              <h3>伏笔档案</h3>
              <div className="form-grid">
                <label className="llm-field">名称{input("name")}</label>
                <label className="llm-field">
                  状态
                  <select
                    className="ps-input"
                    value={form.status || ""}
                    onChange={(e) => setForm({ ...form, status: e.target.value })}
                  >
                    <option value="">选择状态…</option>
                    {FORESHADOW_STATUSES.map((s) => (
                      <option key={s} value={s}>{s}</option>
                    ))}
                  </select>
                </label>
                <label className="llm-field">埋设章节{chapterOptions}</label>
                <label className="llm-field">预期回收章节{input("expected_payoff")}</label>
                <label className="llm-field">实际回收章节{input("actual_payoff")}</label>
                <label className="llm-field">描述{textarea("description", 4)}</label>
              </div>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleSave} disabled={saving}>
                  {saving ? "保存中..." : "保存档案"}
                </button>
                <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
              </div>
            </div>
          )}

          {selected && selected.type !== "Character" && selected.type !== "Organization" &&
            selected.type !== "Setting" && selected.type !== "Event" && selected.type !== "Foreshadow" && (
            <div className="section">
              <h3>{label(entityTypeLabels, selected.type)}档案</h3>
              <p><strong>{selected.name}</strong></p>
              <button className="btn-sm btn-danger" onClick={handleDelete}>删除</button>
            </div>
          )}

          {selectedRuleIdx != null && (
            <div className="section">
              <h3>世界观规则</h3>
              <label className="llm-field">
                规则内容
                <textarea
                  className="ps-input ps-textarea"
                  rows={4}
                  value={form.rule || ""}
                  onChange={(e) => setForm({ ...form, rule: e.target.value })}
                />
              </label>
              <div className="btn-group">
                <button className="btn-primary" onClick={handleRuleSave}>保存规则</button>
                <button className="btn-sm btn-danger" onClick={handleRuleDelete}>删除</button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
