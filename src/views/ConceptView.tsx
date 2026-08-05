import { useState, useMemo, useCallback, useEffect, useRef } from "react";
import type { ProjectData, SproutData, AgentDiscussionConfig, LlmModel, Expert, DiscussionTurn, DiscussionSynthesis, DiscussionEvent } from "../types";
import { DEFAULT_DISCUSSION_AGENTS } from "../types";
import {
  Lightbulb, Target, Bot, MessageSquare,
  Plus, Trash2, GripVertical, FileText,
  PenLine, Layers, BarChart3, BookOpen,
  UserCheck, RefreshCw,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { loadSprout, loadExperts, listModels, discussConcept, getDiscussionState, saveOutlineArcs, clearDiscussionResult } from "../ipc";
import { pickDefaultModel } from "../store";
import { DiscussionPanel, type SelectedResults } from "../components/DiscussionPanel";

interface ConceptViewProps {
  projectData: ProjectData;
  persistProjectData?: (updater: (prev: ProjectData) => ProjectData) => void;
}

// 解析 chapter_hint 中的章节范围（「第1-200章」「第1~3章」「第1至5章」等），
// 返回 [起始章, 结束章]；无法解析或范围无效时返回 null
function parseChapterRange(hint: string): [number, number] | null {
  const m = hint.match(/第?\s*(\d+)\s*[-–~至到]\s*(\d+)\s*章?/);
  if (!m) return null;
  const a = parseInt(m[1], 10);
  const b = parseInt(m[2], 10);
  if (!Number.isFinite(a) || !Number.isFinite(b) || b <= a) return null;
  return [a, b];
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
        const firstModelId = pickDefaultModel(models)?.model_id ?? models[0].model_id;
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

  // 专家浏览弹窗
  const [showExpertBrowser, setShowExpertBrowser] = useState(false);
  // 从专家库导入到指定 Agent（null = 添加新 Agent）
  const [importTargetAgentId, setImportTargetAgentId] = useState<string | null>(null);

  // 更新 sprout 数据
  const updateSprout = useCallback((updater: (prev: SproutData) => SproutData) => {
    persistProjectData?.(prev => ({
      ...prev,
      sprout: updater(prev.sprout),
    }));
  }, [persistProjectData]);

  // 自愈历史数据：已添加的专家 Agent 若丢失 skillPath（旧版同步丢字段），
  // 按名字从专家库回填，保证讨论能加载蒸馏技能文件
  useEffect(() => {
    if (availableExpertsList.length === 0 || sprout.agents.length === 0) return;
    const byName = new Map(availableExpertsList.map(e => [e.name, e]));
    const healable = sprout.agents.filter(
      a => !a.skillPath && byName.get(a.name)?.skillPath
    );
    if (healable.length === 0) return;
    updateSprout(prev => ({
      ...prev,
      agents: prev.agents.map(a => {
        if (a.skillPath) return a;
        const ex = byName.get(a.name);
        return ex?.skillPath
          ? { ...a, expertId: a.expertId || ex.id, skillPath: ex.skillPath }
          : a;
      }),
    }));
  }, [availableExpertsList, sprout.agents, updateSprout]);

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
    const defaultModel = pickDefaultModel(availableModels)?.model_id ?? "gpt-4o";
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

  // 讨论事件统一处理（实时与重放共用）：进度展示 + 已完成发言落入 turns
  // "__discussion__" 是后端在结果持久化后发射的终态标记，不作为发言展示
  const applyDiscussionEvent = useCallback((e: DiscussionEvent) => {
    if (e.agent_id === "__discussion__") return;
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
  }, []);

  // 拉取后端持久化的讨论结果（后台讨论完成后调用）
  const fetchPersistedDiscussion = useCallback(async () => {
    try {
      const loaded = await loadSprout();
      const rec = loaded?.last_discussion;
      if (!rec) return;
      setTurns(rec.turns ?? []);
      setSynthesis(rec.synthesis ?? null);
      setDiscussing(false);
      updateSprout(prev => ({
        ...prev,
        lastDiscussion: {
          turns: rec.turns ?? [],
          synthesis: rec.synthesis,
          authorFeedback: rec.author_feedback ?? "",
        },
      }));
    } catch { /* 拉取失败时下次进入页面再恢复 */ }
  }, [updateSprout]);

  // 重连后台讨论：进入页面时若讨论仍在进行（中途切走过），
  // 重放缓冲事件恢复进度，并订阅终态事件接管结果
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const st = await getDiscussionState().catch(() => null);
      if (!st || cancelled || !st.running) return;
      setDiscussing(true);
      (st.events ?? []).forEach(applyDiscussionEvent);
      const unlisten = await listen<DiscussionEvent>("discussion-event", (evt) => {
        const e = evt.payload;
        if (e.agent_id === "__discussion__" && e.status === "finished") {
          fetchPersistedDiscussion();
          return;
        }
        applyDiscussionEvent(e);
      });
      unlistenRef.current = unlisten;
      if (cancelled) { unlisten(); return; }
      // 订阅间隙讨论可能已结束：复查一次，避免错过终态
      const st2 = await getDiscussionState().catch(() => null);
      if (st2 && !st2.running) fetchPersistedDiscussion();
    })();
    return () => { cancelled = true; };
  }, [applyDiscussionEvent, fetchPersistedDiscussion]);

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
        setDiscussionError("没有启用的评审员，请先添加至少一位评审员");
        setDiscussing(false);
        return;
      }

      // 订阅实时讨论进度事件（终态由 invoke 返回接管，这里只跟进度）
      const unlisten = await listen<DiscussionEvent>("discussion-event", (evt) => {
        applyDiscussionEvent(evt.payload);
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
  }, [agents, sprout.ideaDescription, settingsContext, updateSprout, applyDiscussionEvent]);

  // 确认生成：把勾选的成果写入世界观与人物志（按名称去重）
  const handleConfirmGenerate = useCallback((selected: SelectedResults, authorFeedback: string) => {
    persistProjectData?.(prev => {
      const existingLoc = new Set(prev.world.locations.map(l => l.name));
      const existingEvt = new Set(prev.world.timeline_events.map(e => `${e.story_time}-${e.description}`));
      const existingRule = new Set(prev.world.setting_rules.map(r => r.title));
      const existingChar = new Set(prev.characters.map(c => c.name));

      const now = Date.now();
      const selectedCharNames = new Set(selected.characters.map(c => c.name));
      const newLocations = selected.locations
        .filter(l => !existingLoc.has(l.name))
        .map((l, i) => ({
          id: `loc-${now}-${i}`,
          name: l.name,
          description: l.description,
          level: l.level ?? "",
          region: l.region ?? "",
          faction: l.faction ?? "",
          unlocked_chapter: l.unlocked_chapter ?? "",
          sources: l.sources ?? [],
        }));
      const newEvents = selected.timeline_events
        .filter(e => !existingEvt.has(`${e.story_time}-${e.description}`))
        .map((e, i) => ({
          event_id: `evt-${now}-${i}`,
          story_time: e.story_time,
          description: e.description,
          participants: e.participants ?? [],
          sources: e.sources ?? [],
        }));
      const newRules = selected.setting_rules
        .filter(r => !existingRule.has(r.name))
        .map((r, i) => ({
          rule_id: `rule-${now}-${i}`,
          title: r.name,
          description: r.description,
          constraints: r.constraints ?? [],
          cost: r.cost ?? "",
          loophole: r.loophole ?? "",
          sources: r.sources ?? [],
        }));
      const newCharacters = selected.characters
        .filter(c => !existingChar.has(c.name))
        .map((c, i) => ({
          id: `char-${now}-${i}`,
          name: c.name,
          personality_traits: c.personality_traits,
          current_mood: c.current_mood || "",
          wants: c.wants ?? "",
          fears: c.fears ?? "",
          secret: c.secret ?? "",
          speech_style: c.speech_style ?? "",
          // 提炼层的 arc（CharacterArcStage）落库为人物志的 arc_stages
          arc_stages: (c.arc ?? []).map(a => ({
            name: a.name,
            chapter_range: a.chapter_range ?? "",
            trait_desc: a.trait_desc ?? "",
            goal: a.goal ?? "",
          })),
          knows: c.knows ?? [],
          does_not_know: c.does_not_know ?? [],
          sources: c.sources ?? [],
          // 人物关系：只保留指向本次生成或已有人物的关系
          relationships: (c.relationships || [])
            .filter(r => selectedCharNames.has(r.to) || existingChar.has(r.to))
            .map(r => ({ from: r.from, to: r.to, relation_type: r.relation_type, strength: r.strength })),
        }));

      // 情节脉络 → 大纲规划层：生成脉络节点（不是章节！）。
      // 节点带章节范围，后续在大纲页「展开细纲」才生成逐章可写章节。
      // 若提炼结果带卷标签，则按卷分组建卷（第一卷/第二卷…），脉络节点挂到对应卷下。
      const existingArcTitles = new Set(prev.outlineArcs.map(a => a.title));
      const newBeats = (selected.outline_beats ?? []).filter(b => b.title.trim() && !existingArcTitles.has(b.title.trim()));
      // 卷分组：按 beat.volume 标签（保持首次出现顺序；未分卷节点不建卷）
      const volLabels: string[] = [];
      const volLabelSet = new Set<string>();
      for (const b of newBeats) {
        const label = (b.volume ?? "").trim();
        if (label && !volLabelSet.has(label)) {
          volLabelSet.add(label);
          volLabels.push(label);
        }
      }
      const volIdByLabel = new Map<string, string>();
      const newVolumes: Array<import("../types").VolumeWithChapters> = [];
      volLabels.forEach((label, i) => {
        // 与现有卷按标题去重，避免重复导入时建出同名卷
        const match = prev.volumes.find(v => v.title.trim() === label);
        if (match) {
          volIdByLabel.set(label, match.volume_id);
          return;
        }
        const vid = `vol-${now}-${i}`;
        volIdByLabel.set(label, vid);
        newVolumes.push({ volume_id: vid, title: label, chapter_count: 0, expanded: true, chapters: [] });
      });
      let newArcs: import("../types").OutlineArc[] = [];
      if (newBeats.length > 0) {
        // 范围分配：优先解析 chapter_hint（第N-M章）；没有范围的节点把剩余章数均分
        let cursor = prev.outlineArcs.reduce((m, a) => Math.max(m, a.chapter_end), 0);
        const totalTarget = prev.settings.targetChapters;
        const remaining = totalTarget > cursor ? totalTarget - cursor : 0;
        const evenSpan = Math.max(1, Math.ceil((remaining > 0 ? remaining : newBeats.length * 10) / newBeats.length));
        newArcs = newBeats.map((b, i) => {
          const range = parseChapterRange(b.chapter_hint ?? "");
          const start = range?.[0] ?? cursor + 1;
          const end = range?.[1] ?? Math.max(start, cursor + evenSpan);
          cursor = Math.max(cursor, end);
          const label = (b.volume ?? "").trim();
          return {
            arc_id: `arc-${now}-${i}`,
            title: b.title.trim(),
            description: b.description ?? "",
            chapter_start: start,
            chapter_end: end,
            volume_id: label ? volIdByLabel.get(label) ?? "" : "",
            expanded_until: 0,
          };
        });
        // 脉络节点独立持久化（不走 saveProjectData 全量保存）
        saveOutlineArcs([...prev.outlineArcs, ...newArcs])
          .catch(e => console.error("脉络节点保存失败:", e));
      }

      return {
        ...prev,
        world: {
          locations: [...prev.world.locations, ...newLocations],
          timeline_events: [...prev.world.timeline_events, ...newEvents],
          setting_rules: [...prev.world.setting_rules, ...newRules],
        },
        characters: [...prev.characters, ...newCharacters],
        outlineArcs: [...prev.outlineArcs, ...newArcs],
        volumes: [...prev.volumes, ...newVolumes],
          sprout: {
            ...prev.sprout,
            lastDiscussion: {
              ...(prev.sprout.lastDiscussion ?? {
                turns: [],
                synthesis: {
                  summary: "",
                  locations: [],
                  timeline_events: [],
                  setting_rules: [],
                  characters: [],
                  outline_beats: [],
                },
              }),
              authorFeedback,
            },
          },
      };
    });
    setGenerated(true);
  }, [persistProjectData]);

  // 单独提交作者意见（不触发生成）：保存到 sprout.lastDiscussion.authorFeedback
  const handleSubmitFeedback = useCallback((feedback: string) => {
    updateSprout(prev => ({
      ...prev,
      lastDiscussion: prev.lastDiscussion
        ? { ...prev.lastDiscussion, authorFeedback: feedback }
        : { turns: [], synthesis: { summary: "", locations: [], timeline_events: [], setting_rules: [], characters: [], outline_beats: [] }, authorFeedback: feedback },
    }));
  }, [updateSprout]);

  // 重新讨论：清空当前讨论结果与生成状态，回到初始
  const handleRestartDiscussion = useCallback(() => {
    setTurns([]);
    setSynthesis(null);
    setLiveEvents({});
    setGenerated(false);
    setDiscussionError(null);
    updateSprout(prev => ({ ...prev, lastDiscussion: undefined }));
    // 后端同步清空，避免 save_sprout 的「None 保留旧结果」保护把旧成果带回来
    clearDiscussionResult().catch(e => console.error("清空讨论结果失败:", e));
  }, [updateSprout]);

  const hasIdea = sprout.ideaDescription.trim().length > 0;

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>灵魂萌芽</h2>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px" }}>
          想法描述 · 创作设定 · 多维度讨论
        </span>
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
            设定目标，为评审员提供讨论上下文
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

      {/* ── 第三步：评审员讨论配置 ── */}
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
            评审员讨论配置
          </span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", letterSpacing: "0.5px", marginLeft: "auto" }}>
            配置多位评审员从不同维度讨论构思
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
                  placeholder="评审员名称" />
                <span style={{ fontSize: "var(--text-2xs)", padding: "2px 8px", borderRadius: "var(--radius-xs)", background: "var(--color-jade-wash)", color: "var(--color-jade)" }}>
                  {agent.perspective}
                </span>
                <button className="pv-icon-btn" onClick={() => { setImportTargetAgentId(agent.id); setShowExpertBrowser(true); }} title={"从专家库选择"}>
                  <UserCheck size={14} />
                </button>
                <button className="pv-icon-btn pv-icon-btn-danger" onClick={() => removeAgent(agent.id)} title={"删除此评审员"}>
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
                  placeholder={"评审员的评审方向和评价标准..."}
                  value={agent.prompt}
                  onChange={(e) => updateAgent(agent.id, { prompt: e.target.value })} />
              </div>
            </div>
          ))}
        </div>

        <div style={{ display: "flex", gap: "var(--space-sm)", marginTop: "var(--space-md)" }}>
          <button className="btn btn-secondary" onClick={addAgent}>
            <Plus size={14} /> 添加评审员
          </button>
          <button className="btn btn-accent" onClick={() => { setImportTargetAgentId(null); setShowExpertBrowser(true); }}>
            <UserCheck size={14} /> 从专家库添加
          </button>
        </div>
      </div>

      {/* ── 专家浏览器弹窗 ── */}
      {showExpertBrowser && (
        <div style={{ position: "fixed", inset: 0, zIndex: 1000, background: "rgba(0,0,0,0.4)", display: "flex", alignItems: "center", justifyContent: "center" }}
          onClick={() => { setImportTargetAgentId(null); setShowExpertBrowser(false); }}>
          <div style={{ background: "var(--color-paper)", borderRadius: "var(--radius-md)", padding: "var(--space-xl)", maxWidth: 520, width: "90%", maxHeight: "70vh", overflow: "auto", boxShadow: "var(--shadow-lg)" }}
            onClick={e => e.stopPropagation()}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)" }}>
              <UserCheck size={18} style={{ color: "var(--color-accent)" }} />
              <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>
                {importTargetAgentId ? "从专家库导入" : "从专家库添加"}
              </span>
              <button className="pv-icon-btn" style={{ marginLeft: "auto" }} onClick={() => { setImportTargetAgentId(null); setShowExpertBrowser(false); }}>&times;</button>
            </div>
            {availableExpertsList.length === 0 ? (
              <div style={{ padding: 20, textAlign: "center", color: "var(--color-ink-3)" }}>专家库为空，请先前往「专家库」页面创建或导入</div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
                {availableExpertsList.map(ex => (
                  <div key={ex.id} style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", padding: "var(--space-sm) var(--space-md)", border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-sm)", cursor: "pointer", transition: "all var(--dur-short) var(--ease-out)" }}
                    onClick={() => {
                      if (importTargetAgentId) {
                        updateAgent(importTargetAgentId, {
                          name: ex.name,
                          model: ex.modelId,
                          perspective: ex.perspective,
                          prompt: ex.defaultPrompt,
                          expertId: ex.id,
                        });
                      } else {
                        addExpertAsAgent(ex);
                      }
                      setImportTargetAgentId(null);
                      setShowExpertBrowser(false);
                    }}
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
          {discussing ? (<><MessageSquare size={18} /> 讨论中（共 {agents.filter(a => a.enabled).length} 位评审员）</>) : (<><MessageSquare size={18} /> 启动多维度讨论</>)}
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

      {/* ── 系统提示（模型自动切换/不可用等） ── */}
      {Object.values(liveEvents).filter(e => e.agent_id === "__system__").map(e => (
        <div key={e.agent_id} style={{
          marginBottom: "var(--space-md)", padding: "var(--space-sm) var(--space-md)",
          background: "var(--color-ochre-wash, rgba(191,144,0,0.08))",
          border: "1px solid var(--color-ochre)", borderRadius: "var(--radius-sm)",
          fontSize: "var(--text-xs)", color: "var(--color-ochre)",
          display: "flex", alignItems: "center", gap: 8,
        }}>
          <RefreshCw size={13} />
          {e.content}
        </div>
      ))}

      {/* ── 讨论过程 + 讨论成果 ── */}
      {(turns.length > 0 || Object.keys(liveEvents).length > 0 || synthesis) && (
        <DiscussionPanel
          agents={agents}
          turns={turns}
          liveEvents={liveEvents}
          synthesis={synthesis}
          discussing={discussing}
          onConfirmGenerate={handleConfirmGenerate}
          onSubmitFeedback={handleSubmitFeedback}
          onRestartDiscussion={handleRestartDiscussion}
          generated={generated}
        />
      )}
    </div>
  );
}
