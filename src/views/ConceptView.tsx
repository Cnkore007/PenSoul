import { useState, useMemo, useCallback, useEffect, useRef } from "react";
import type { ProjectData, SproutData, AgentDiscussionConfig, LlmModel, Expert, DiscussionTurn, DiscussionSynthesis, DiscussionEvent, Chapter } from "../types";
import { DEFAULT_DISCUSSION_AGENTS } from "../types";
import {
  Lightbulb, Target, Bot, MessageSquare,
  Plus, Trash2, GripVertical, FileText,
  PenLine, Layers, BarChart3, BookOpen,
  Save, Upload, UserCheck,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { saveSprout, loadSprout, saveSettings, loadExperts, listModels, discussConcept } from "../ipc";
import { DiscussionPanel, type SelectedResults } from "../components/DiscussionPanel";

interface ConceptViewProps {
  projectData: ProjectData;
  persistProjectData?: (updater: (prev: ProjectData) => ProjectData) => void;
}

export function ConceptView({ projectData, persistProjectData }: ConceptViewProps) {
  const { settings, sprout } = projectData;

  // 可用模型列表（异步加载）
  const [availableModels, setAvailableModels] = useState<LlmModel[]>([]);
  // 可用专家列表（异步加载）
  const [availableExpertsList, setAvailableExpertsList] = useState<Expert[]>([]);

  // 加载模型和专家数据
  useEffect(() => {
    listModels().then(models => {
      setAvailableModels(models);
      // 模型加载后，将 Agent 中不存在的模型自动替换为第一个可用模型
      if (models.length > 0 && persistProjectData) {
        const validIds = new Set(models.map((m: LlmModel) => m.model_id));
        const firstModelId = models[0].model_id;
        persistProjectData(prev => ({
          ...prev,
          sprout: {
            ...prev.sprout,
            agents: prev.sprout.agents.map(a => ({
              ...a,
              model: validIds.has(a.model) ? a.model : firstModelId,
            })),
          },
        }));
      }
    }).catch(() => {});
    loadExperts().then(raw => {
      const mapped: Expert[] = raw.map((e: any) => ({
        id: e.id,
        name: e.name,
        description: e.description,
        sourcePersona: e.source_persona,
        modelId: e.model_id,
        perspective: e.perspective,
        defaultPrompt: e.default_prompt,
        createdAt: e.created_at,
        skillPath: e.skill_path,
        skillSummary: e.skill_summary,
      }));
      setAvailableExpertsList(mapped);
    }).catch(() => {});
  }, []);

  // 自动计算总字数
  const autoTargetWords = useMemo(() => {
    if (settings.targetChapters > 0 && settings.chapterTargetWords > 0) {
      return settings.targetChapters * settings.chapterTargetWords;
    }
    return 0;
  }, [settings.targetChapters, settings.chapterTargetWords]);

  // 后端同步状态
  const [backendSynced, setBackendSynced] = useState<boolean | null>(null);

  // 同步到后端
  const syncToEngine = useCallback(async () => {
    try {
      await saveSprout({
        idea_description: sprout.ideaDescription,
        agents: sprout.agents.map(a => ({
          id: a.id,
          name: a.name,
          model: a.model,
          prompt: a.prompt,
          perspective: a.perspective,
          enabled: a.enabled,
        })),
      });
      await saveSettings({
        target_chapters: settings.targetChapters,
        target_words: autoTargetWords,
        chapter_target_words: settings.chapterTargetWords,
        target_volumes: settings.targetVolumes,
        genre: settings.genre || "",
      });
      setBackendSynced(true);
    } catch {
      setBackendSynced(false);
    }
    setTimeout(() => setBackendSynced(null), 3000);
  }, [sprout, settings, autoTargetWords]);

  // 从后端加载
  const loadFromEngine = useCallback(async () => {
    const loaded = await loadSprout();
    if (loaded && persistProjectData) {
      persistProjectData(prev => ({
        ...prev,
        sprout: {
          ideaDescription: loaded.idea_description,
          agents: loaded.agents.map((a: any) => ({
            id: a.id,
            name: a.name,
            model: a.model,
            prompt: a.prompt,
            perspective: a.perspective,
            enabled: a.enabled,
          })),
        },
      }));
    }
  }, [persistProjectData]);

  // 专家浏览弹窗
  const [showExpertBrowser, setShowExpertBrowser] = useState(false);

  // 更新 sprout 数据
  const updateSprout = useCallback((updater: (prev: SproutData) => SproutData) => {
    persistProjectData?.(prev => ({
      ...prev,
      sprout: updater(prev.sprout),
    }));
  }, [persistProjectData]);

  // 从专家库添加 Agent（预置模式下：预置自动消失，替换为专家 Agent）
  const addExpertAsAgent = useCallback((expert: Expert) => {
    const newAgent: AgentDiscussionConfig = {
      id: "agent-" + Date.now() + "-" + Math.random().toString(36).slice(2, 6),
      name: expert.name,
      model: expert.modelId,
      perspective: expert.perspective,
      prompt: expert.defaultPrompt,
      enabled: true,
      expertId: expert.id,
      skillPath: expert.skillPath,
    };
    updateSprout(prev => {
      const usingPresets = prev.agents.length === 0 && !prev.presetsDismissed;
      // 防止重复添加同一专家
      const rest = usingPresets ? [] : prev.agents.filter(a => a.expertId !== expert.id);
      return { ...prev, agents: [...rest, newAgent], presetsDismissed: true };
    });
  }, [updateSprout]);


  // 更新 settings
  const updateSetting = useCallback((key: string, value: number | string) => {
    persistProjectData?.(prev => ({
      ...prev,
      settings: { ...prev.settings, [key]: value },
    }));
  }, [persistProjectData]);

  // 添加 Agent（预置模式下：预置自动消失）
  const addAgent = useCallback(() => {
    const defaultModel = availableModels.length > 0 ? availableModels[0].model_id : "gpt-4o";
    const newAgent: AgentDiscussionConfig = {
      id: "agent-" + Date.now(),
      name: "新评审员",
      model: defaultModel,
      perspective: "综合评审",
      prompt: "请从综合角度分析这个构思的可行性。",
      enabled: true,
    };
    updateSprout(prev => {
      const usingPresets = prev.agents.length === 0 && !prev.presetsDismissed;
      return { ...prev, agents: [...(usingPresets ? [] : prev.agents), newAgent], presetsDismissed: true };
    });
  }, [updateSprout, availableModels]);

  // 删除 Agent（预置模式下：删除某个预置后，其余预置落地为普通 Agent）
  const removeAgent = useCallback((id: string) => {
    updateSprout(prev => {
      const usingPresets = prev.agents.length === 0 && !prev.presetsDismissed;
      if (usingPresets) {
        return {
          ...prev,
          agents: DEFAULT_DISCUSSION_AGENTS.filter(a => a.id !== id),
          presetsDismissed: true,
        };
      }
      return { ...prev, agents: prev.agents.filter(a => a.id !== id) };
    });
  }, [updateSprout]);

  // 更新单个 Agent
  const updateAgent = useCallback((id: string, updater: Partial<AgentDiscussionConfig>) => {
    updateSprout(prev => ({
      ...prev,
      agents: prev.agents.map(a => a.id === id ? { ...a, ...updater } : a),
    }));
  }, [updateSprout]);

  // 从专家库导入（回填到当前 Agent）
  const importFromExpert = useCallback((agentId: string) => {
    if (availableExpertsList.length === 0) {
      alert("专家库为空，请先前往「专家库」页面创建或导入专家");
      return;
    }
    const chosen = window.prompt("输入要导入的专家序号：\n" + availableExpertsList.map(function(e, i) { return (i+1) + ". " + e.name + " [" + e.perspective + "]"; }).join("\n"));
    if (!chosen) return;
    const idx = parseInt(chosen) - 1;
    if (!isNaN(idx) && idx >= 0 && idx < availableExpertsList.length) {
      const expert = availableExpertsList[idx];
      updateAgent(agentId, { name: expert.name, model: expert.modelId, perspective: expert.perspective, prompt: expert.defaultPrompt, expertId: expert.id });
    }
  }, [updateAgent, availableExpertsList]);

  // 重置为预置 Agent（清空自定义，回到预置回退模式）
  const resetAgents = useCallback(() => {
    updateSprout(prev => ({ ...prev, agents: [], presetsDismissed: false }));
  }, [updateSprout]);

  // 启动讨论（真实 LLM 调用，两轮交锋 + 结构化成果）
  // 预置回退模式：agents 为空且用户未移除预置时，显示预置 Agent
  const usingPresets = sprout.agents.length === 0 && !sprout.presetsDismissed;
  const agents = usingPresets ? DEFAULT_DISCUSSION_AGENTS : sprout.agents;
  const [discussionError, setDiscussionError] = useState<string | null>(null);
  // 讨论结果初始化为上次的讨论（切换页面后保留）
  const [turns, setTurns] = useState<DiscussionTurn[]>(() => sprout.lastDiscussion?.turns ?? []);
  const [synthesis, setSynthesis] = useState<DiscussionSynthesis | null>(() => sprout.lastDiscussion?.synthesis ?? null);
  const [liveEvents, setLiveEvents] = useState<Record<string, DiscussionEvent>>({});
  const [discussing, setDiscussing] = useState(false);
  const [generated, setGenerated] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => { if (unlistenRef.current) unlistenRef.current(); };
  }, []);

  // 创作设定上下文（随讨论传给每个 Agent：章节、字数、卷数、类型）
  const settingsContext = useMemo(() => {
    const parts: string[] = [];
    if (settings.genre) parts.push(`故事类型：${settings.genre}`);
    if (settings.targetChapters > 0) parts.push(`目标章数：${settings.targetChapters} 章`);
    if (settings.chapterTargetWords > 0) parts.push(`每章目标字数：${settings.chapterTargetWords} 字`);
    if (autoTargetWords > 0) parts.push(`预计总字数：${autoTargetWords.toLocaleString()} 字`);
    if (settings.targetVolumes > 0) parts.push(`预计卷数：${settings.targetVolumes} 卷`);
    return parts.length > 0 ? parts.join("；") : "（作者尚未填写创作设定，请提醒作者补充）";
  }, [settings, autoTargetWords]);

  const startDiscussion = useCallback(async () => {
    try {
      setDiscussionError(null);
      setDiscussing(true);
      setTurns([]);
      setSynthesis(null);
      setLiveEvents({});
      setGenerated(false);
      const enabledAgents = agents.filter(a => a.enabled);
      if (enabledAgents.length === 0) {
        setDiscussionError("没有启用的 Agent，请先添加至少一个 Agent");
        setDiscussing(false);
        return;
      }

      // 订阅实时讨论进度事件
      const unlisten = await listen<DiscussionEvent>("discussion-event", (evt) => {
        const e = evt.payload;
        setLiveEvents(prev => ({ ...prev, [`${e.agent_id}-${e.round}`]: e }));
        if (e.status === "done" && (e.round === 1 || e.round === 2)) {
          setTurns(prev => {
            // 事件可能重复，按 agent+round 去重
            if (prev.some(t => t.agent_id === e.agent_id && t.round === e.round)) return prev;
            return [...prev, {
              agent_id: e.agent_id,
              agent_name: e.agent_name,
              perspective: "",
              round: e.round,
              content: e.content,
            }];
          });
        }
      });
      unlistenRef.current = unlisten;

      try {
        const output = await discussConcept(
          sprout.ideaDescription,
          settingsContext,
          enabledAgents.map(a => ({
            id: a.id,
            name: a.name,
            model: a.model,
            prompt: a.prompt,
            perspective: a.perspective,
            enabled: a.enabled,
            skill_path: a.skillPath || null,
          })),
        );
        setTurns(output.turns);
        setSynthesis(output.synthesis);
        // 持久化讨论结果，切换页面后仍可查看
        updateSprout(prev => ({
          ...prev,
          lastDiscussion: { turns: output.turns, synthesis: output.synthesis },
        }));
      } finally {
        if (unlistenRef.current) { unlistenRef.current(); unlistenRef.current = null; }
        setDiscussing(false);
      }
    } catch (e: any) {
      setDiscussionError("讨论出错: " + (e?.message || String(e)));
      setDiscussing(false);
    }
  }, [agents, sprout.ideaDescription, settingsContext, updateSprout]);

  // 确认生成：把勾选的成果写入世界观与人物志（按名称去重）
  const handleConfirmGenerate = useCallback((selected: SelectedResults) => {
    persistProjectData?.(prev => {
      const existingLoc = new Set(prev.world.locations.map(l => l.name));
      const existingEvt = new Set(prev.world.timeline_events.map(e => `${e.story_time}-${e.description}`));
      const existingRule = new Set(prev.world.setting_rules.map(r => r.title));
      const existingChar = new Set(prev.characters.map(c => c.name));

      const now = Date.now();
      const selectedCharNames = new Set(selected.characters.map(c => c.name));
      const newLocations = selected.locations
        .filter(l => !existingLoc.has(l.name))
        .map((l, i) => ({ id: `loc-${now}-${i}`, name: l.name, description: l.description }));
      const newEvents = selected.timeline_events
        .filter(e => !existingEvt.has(`${e.story_time}-${e.description}`))
        .map((e, i) => ({ event_id: `evt-${now}-${i}`, story_time: e.story_time, description: e.description }));
      const newRules = selected.setting_rules
        .filter(r => !existingRule.has(r.name))
        .map((r, i) => ({ rule_id: `rule-${now}-${i}`, title: r.name, description: r.description }));
      const newCharacters = selected.characters
        .filter(c => !existingChar.has(c.name))
        .map((c, i) => ({
          id: `char-${now}-${i}`,
          name: c.name,
          personality_traits: c.personality_traits,
          current_mood: c.current_mood || "",
          // 人物关系：只保留指向本次生成或已有人物的关系
          relationships: (c.relationships || [])
            .filter(r => selectedCharNames.has(r.to) || existingChar.has(r.to))
            .map(r => ({ from: r.from, to: r.to, relation_type: r.relation_type, strength: r.strength })),
        }));

      // 情节脉络 → 大纲：作为章节追加到第一卷（按章节标题去重），没有卷则自动建「第一卷」
      const existingChapterTitles = new Set(prev.volumes.flatMap(v => v.chapters.map(c => c.title)));
      const newBeats = (selected.outline_beats ?? []).filter(b => b.title.trim() && !existingChapterTitles.has(b.title));
      let volumes = prev.volumes;
      if (newBeats.length > 0) {
        // 讨论成果是大纲层的章节梗概，正文留空待作者在笔耕中创作
        const toChapter = (b: (typeof newBeats)[number], i: number, volId: string): Chapter => {
          const hint = (b.chapter_hint ?? "").trim();
          return {
            chapter_id: `ch-${now}-${i}`,
            volume_id: volId,
            title: b.title.trim(),
            summary: hint ? `【${hint}】\n${b.description}` : b.description,
            content: "",
            word_count: 0,
            version: 1,
            status: "Draft",
          };
        };
        if (volumes.length === 0) {
          const volId = `vol-${now}`;
          volumes = [{
            volume_id: volId,
            title: "第一卷",
            chapter_count: newBeats.length,
            chapters: newBeats.map((b, i) => toChapter(b, i, volId)),
            expanded: true,
          }];
        } else {
          const firstId = volumes[0].volume_id;
          const added = newBeats.map((b, i) => toChapter(b, i, firstId));
          volumes = volumes.map((v, vi) => vi === 0
            ? { ...v, chapters: [...v.chapters, ...added], chapter_count: v.chapters.length + added.length }
            : v);
        }
      }

      return {
        ...prev,
        world: {
          locations: [...prev.world.locations, ...newLocations],
          timeline_events: [...prev.world.timeline_events, ...newEvents],
          setting_rules: [...prev.world.setting_rules, ...newRules],
        },
        characters: [...prev.characters, ...newCharacters],
        volumes,
      };
    });
    setGenerated(true);
  }, [persistProjectData]);

  const hasIdea = sprout.ideaDescription.trim().length > 0;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>灵魂萄芽</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px" }}>
          想法描述 · 创作设定 · 多维度讨论
        </span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
          <button className="btn btn-accent" onClick={syncToEngine} style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}>
            <Save size={13} />
            同步到引擎
          </button>
          <button className="btn btn-secondary" onClick={loadFromEngine} style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}>
            <Upload size={13} /> 从引擎加载
          </button>
          {backendSynced !== null && (
            <span style={{
              fontSize: "var(--text-xs)", padding: "2px 8px", borderRadius: "var(--radius-sm)",
              background: backendSynced ? "var(--color-jade-wash)" : "var(--color-error-wash)",
              color: backendSynced ? "var(--color-jade)" : "var(--color-error)",
            }}>
              {backendSynced ? "已同步" : "失败"}
            </span>
          )}
        </div>
      </div>

      {/* ── 第一步：想法描述 ── */}
      <div style={{
        background: "var(--color-paper)", border: "1px solid var(--color-rule-light)",
        borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)",
        marginBottom: "var(--space-xl)", boxShadow: "var(--shadow-subtle)",
      }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)",
          borderBottom: "1px solid var(--color-rule-light)",
        }}>
          <div style={{
            width: 28, height: 28, borderRadius: "50%",
            background: "var(--color-accent)", display: "flex",
            alignItems: "center", justifyContent: "center", flexShrink: 0,
            color: "#fff", fontSize: "var(--text-xs)", fontWeight: 600,
          }}>1</div>
          <Lightbulb size={18} style={{ color: "var(--color-accent)" }} />
          <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
            想法描述
          </span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px", marginLeft: "auto" }}>
            用你的话描述这个故事的种子
          </span>
        </div>
        <textarea
          className="pm-textarea"
          style={{ minHeight: 120, fontSize: "var(--text-sm)", lineHeight: 1.8 }}
          placeholder={"写下让你兴奋的核心想法。可以是一段话、一个场景、一个设定——任何形式。"}
          value={sprout.ideaDescription}
          onChange={(e) => updateSprout(prev => ({ ...prev, ideaDescription: e.target.value }))}
        />
        <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginTop: 4, letterSpacing: "0.3px", textAlign: "right" }}>
          {sprout.ideaDescription.length} 字
        </div>
      </div>

      {/* ── 第二步：创作设定 ── */}
      <div style={{
        background: "var(--color-paper)", border: "1px solid var(--color-rule-light)",
        borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)",
        marginBottom: "var(--space-xl)", boxShadow: "var(--shadow-subtle)",
      }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)",
          borderBottom: "1px solid var(--color-rule-light)",
        }}>
          <div style={{
            width: 28, height: 28, borderRadius: "50%",
            background: "var(--color-indigo)", display: "flex",
            alignItems: "center", justifyContent: "center", flexShrink: 0,
            color: "#fff", fontSize: "var(--text-xs)", fontWeight: 600,
          }}>2</div>
          <Target size={18} style={{ color: "var(--color-indigo)" }} />
          <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
            创作设定
          </span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px", marginLeft: "auto" }}>
            设定目标，为 Agent 提供讨论上下文
          </span>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: "var(--space-md)" }}>
          {/* 目标章数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <FileText size={13} /> 目标总章数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder={"例：100"}
              value={settings.targetChapters || ""}
              onChange={(e) => updateSetting("targetChapters", parseInt(e.target.value) || 0)} />
          </div>
          {/* 每章字数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <BarChart3 size={13} /> 每章目标字数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder={"例：3000"}
              value={settings.chapterTargetWords || ""}
              onChange={(e) => updateSetting("chapterTargetWords", parseInt(e.target.value) || 0)} />
          </div>
          {/* 自动计算字数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <PenLine size={13} /> 预计总字数（自动计算）
            </label>
            <div style={{ padding: "8px 12px", fontSize: "var(--text-sm)", background: "var(--color-paper-warm)", color: "var(--color-ink-2)", border: "1px solid var(--color-rule)", borderRadius: "var(--radius-sm)", minHeight: 38, display: "flex", alignItems: "center" }}>
              {autoTargetWords > 0 ? (
                <span style={{ fontWeight: 600 }}>
                  {autoTargetWords.toLocaleString()} 字
                  <span style={{ fontWeight: 400, color: "var(--color-ink-3)", marginLeft: 6, fontSize: "var(--text-xs)" }}>
                    ({settings.targetChapters} 章 × {settings.chapterTargetWords.toLocaleString()} 字)
                  </span>
                </span>
              ) : (
                <span style={{ color: "var(--color-ink-faint)", fontStyle: "italic" }}>
                  输入目标章数和每章字数后自动计算
                </span>
              )}
            </div>
          </div>
          {/* 卷数 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <Layers size={13} /> 预计卷数
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="number" min="0" placeholder={"例：5"}
              value={settings.targetVolumes || ""}
              onChange={(e) => updateSetting("targetVolumes", parseInt(e.target.value) || 0)} />
          </div>
          {/* 类型 */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", display: "flex", alignItems: "center", gap: 4, letterSpacing: "0.5px" }}>
              <BookOpen size={13} /> 故事类型
            </label>
            <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-sm)", padding: "8px 12px" }}
              type="text" placeholder={"例：玄幻、言情、科幻"}
              value={settings.genre || ""}
              onChange={(e) => updateSetting("genre", e.target.value)} />
          </div>
        </div>
      </div>

      {/* ── 第三步：Agent 讨论配置 ── */}
      <div style={{
        background: "var(--color-paper)", border: "1px solid var(--color-rule-light)",
        borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)",
        marginBottom: "var(--space-xl)", boxShadow: "var(--shadow-subtle)",
      }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)",
          borderBottom: "1px solid var(--color-rule-light)",
        }}>
          <div style={{ width: 28, height: 28, borderRadius: "50%", background: "var(--color-jade)", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, color: "#fff", fontSize: "var(--text-xs)", fontWeight: 600 }}>3</div>
          <Bot size={18} style={{ color: "var(--color-jade)" }} />
          <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
            Agent 讨论配置
          </span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px", marginLeft: "auto" }}>
            配置多个 Agent 从不同维度讨论构思
          </span>
          {usingPresets && (
            <span style={{ fontSize: "var(--text-2xs)", padding: "2px 8px", borderRadius: "var(--radius-xs)", background: "var(--color-indigo-wash)", color: "var(--color-indigo)" }}>
              预置 · 添加专家后自动替换
            </span>
          )}
          <button className="btn btn-secondary" style={{ padding: "4px 10px", fontSize: "var(--text-xs)" }} onClick={resetAgents}>
            重置预置
          </button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
          {agents.map((agent) => (
            <div key={agent.id} style={{
              border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-md)",
              padding: "var(--space-md)", background: agent.enabled ? "var(--color-paper)" : "var(--color-paper-warm)",
              opacity: agent.enabled ? 1 : 0.5,
            }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", marginBottom: "var(--space-sm)" }}>
                <GripVertical size={14} style={{ color: "var(--color-ink-faint)", cursor: "grab" }} />
                <input style={{ flex: 1, border: "none", background: "transparent", fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "1px", color: "var(--color-ink)", outline: "none" }}
                  value={agent.name}
                  onChange={(e) => updateAgent(agent.id, { name: e.target.value })}
                  placeholder="Agent 名称" />
                <span style={{ fontSize: "var(--text-2xs)", padding: "2px 8px", borderRadius: "var(--radius-xs)", background: "var(--color-jade-wash)", color: "var(--color-jade)" }}>
                  {agent.perspective}
                </span>
                <button className="pv-icon-btn" onClick={() => importFromExpert(agent.id)} title={"从专家库选择"}>
                  <UserCheck size={14} />
                </button>
                <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => removeAgent(agent.id)} title={"删除此 Agent"}>
                  <Trash2 size={14} />
                </button>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-sm)", marginBottom: "var(--space-sm)" }}>
                <div>
                  <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "block", marginBottom: 2, letterSpacing: "0.5px" }}>评审维度</label>
                  <input className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-xs)", padding: "4px 8px" }}
                    placeholder={"例：商业与市场"}
                    value={agent.perspective}
                    onChange={(e) => updateAgent(agent.id, { perspective: e.target.value })} />
                </div>
                <div>
                  <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "block", marginBottom: 2, letterSpacing: "0.5px" }}>模型</label>
                  <select className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-xs)", padding: "4px 8px" }}
                    value={agent.model}
                    onChange={(e) => updateAgent(agent.id, { model: e.target.value })}>
                    {availableModels.map(m => (
                      <option key={m.model_id} value={m.model_id}>{m.display_name} ({m.model_id})</option>
                    ))}
                    {availableModels.length === 0 && (
                      <>
                        <option value="gpt-4o">GPT-4o (gpt-4o)</option>
                        <option value="gpt-4o-mini">GPT-4o Mini (gpt-4o-mini)</option>
                        <option value="claude-sonnet-4-20250514">Claude Sonnet 4</option>
                        <option value="deepseek-chat">DeepSeek V3</option>
                        <option value="qwen-2.5-72b">Qwen 2.5 72B</option>
                      </>
                    )}
                  </select>
                </div>
              </div>

              <div>
                <label style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", display: "block", marginBottom: 2, letterSpacing: "0.5px" }}>评审提示词</label>
                <textarea className="pm-input" style={{ marginBottom: 0, fontSize: "var(--text-xs)", padding: "4px 8px", minHeight: 50, resize: "vertical" }}
                  placeholder={"Agent 的评审方向和评价标准..."}
                  value={agent.prompt}
                  onChange={(e) => updateAgent(agent.id, { prompt: e.target.value })} />
              </div>
            </div>
          ))}
        </div>

        <div style={{ display: "flex", gap: "var(--space-sm)", marginTop: "var(--space-md)" }}>
          <button className="btn btn-secondary" onClick={addAgent}>
            <Plus size={14} /> 添加 Agent
          </button>
          <button className="btn btn-accent" onClick={() => setShowExpertBrowser(true)}>
            <UserCheck size={14} /> 从专家库添加
          </button>
        </div>
      </div>

      {/* ── 专家浏览器弹窗 ── */}
      {showExpertBrowser && (
        <div style={{ position: "fixed", inset: 0, zIndex: 1000, background: "rgba(0,0,0,0.4)", display: "flex", alignItems: "center", justifyContent: "center" }}
          onClick={() => setShowExpertBrowser(false)}>
          <div style={{ background: "var(--color-paper)", borderRadius: "var(--radius-md)", padding: "var(--space-xl)", maxWidth: 520, width: "90%", maxHeight: "70vh", overflow: "auto", boxShadow: "var(--shadow-lg)" }}
            onClick={e => e.stopPropagation()}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)" }}>
              <UserCheck size={18} style={{ color: "var(--color-accent)" }} />
              <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>从专家库添加</span>
              <button className="pv-icon-btn" style={{ marginLeft: "auto" }} onClick={() => setShowExpertBrowser(false)}>&times;</button>
            </div>
            {availableExpertsList.length === 0 ? (
              <div style={{ padding: 20, textAlign: "center", color: "var(--color-ink-3)" }}>专家库为空，请先前往「专家库」页面创建或导入</div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
                {availableExpertsList.map(ex => (
                  <div key={ex.id} style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", padding: "var(--space-sm) var(--space-md)", border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-sm)", cursor: "pointer", transition: "all var(--dur-short) var(--ease-out)" }}
                    onClick={() => { addExpertAsAgent(ex); setShowExpertBrowser(false); }}
                    onMouseOver={e => (e.currentTarget.style.borderColor = "var(--color-accent)")}
                    onMouseOut={e => (e.currentTarget.style.borderColor = "")}>
                    <Bot size={18} style={{ color: "var(--color-accent)", flexShrink: 0 }} />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: "var(--text-sm)", fontWeight: 500, color: "var(--color-ink)" }}>{ex.name}</div>
                      <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{ex.perspective} &middot; {ex.modelId}</div>
                    </div>
                    <Plus size={16} style={{ color: "var(--color-accent)", flexShrink: 0 }} />
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── 启动讨论 ── */}
      <div style={{ textAlign: "center", marginBottom: "var(--space-xl)" }}>
        <button className="btn btn-primary" style={{ padding: "12px 32px", fontSize: "var(--text-md)", letterSpacing: "2px" }}
          onClick={startDiscussion} disabled={discussing || !hasIdea}>
          {discussing ? (<><MessageSquare size={18} /> 讨论中（共 {agents.filter(a => a.enabled).length} 位 Agent）</>) : (<><MessageSquare size={18} /> 启动多维度讨论</>)}
        </button>
        {!hasIdea && (
          <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 8, fontStyle: "italic" }}>
            请先在「想法描述」中写下你的故事构思
          </div>
        )}
      </div>

      {/* ── 错误提示 ── */}
      {discussionError && (
        <div style={{ marginBottom: "var(--space-md)", padding: "var(--space-sm) var(--space-md)", background: "var(--color-error-wash)", border: "1px solid var(--color-error)", borderRadius: "var(--radius-sm)", fontSize: "var(--text-xs)", color: "var(--color-error)" }}>
          {discussionError}
        </div>
      )}

      {/* ── 讨论过程 + 讨论成果 ── */}
      {(turns.length > 0 || Object.keys(liveEvents).length > 0 || synthesis) && (
        <DiscussionPanel
          agents={agents}
          turns={turns}
          liveEvents={liveEvents}
          synthesis={synthesis}
          discussing={discussing}
          onConfirmGenerate={handleConfirmGenerate}
          generated={generated}
        />
      )}
    </div>
  );
}
