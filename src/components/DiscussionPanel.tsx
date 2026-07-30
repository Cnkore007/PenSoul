import { useMemo, useState } from "react";
import { Bot, CheckCircle2, Loader2, XCircle, MapPin, Clock, BookOpen, Users, Sparkles, ListOrdered } from "lucide-react";
import type { DiscussionTurn, DiscussionSynthesis, DiscussionEvent, AgentDiscussionConfig } from "../types";

interface DiscussionPanelProps {
  agents: AgentDiscussionConfig[];
  turns: DiscussionTurn[];
  liveEvents: Record<string, DiscussionEvent>; // key: `${agent_id}-${round}`
  synthesis: DiscussionSynthesis | null;
  discussing: boolean;
  onConfirmGenerate: (selected: SelectedResults) => void;
  generated: boolean;
}

export interface SelectedResults {
  locations: Array<{ name: string; description: string }>;
  timeline_events: Array<{ story_time: string; description: string }>;
  setting_rules: Array<{ name: string; description: string }>;
  characters: Array<{
    name: string;
    personality_traits: Array<[string, number]>;
    current_mood?: string;
    description?: string;
    relationships?: Array<{ from: string; to: string; relation_type: string; strength: number }>;
  }>;
  outline_beats: Array<{ title: string; description: string; chapter_hint?: string }>;
}

const ROUND_LABELS: Record<number, string> = { 1: "第一轮 · 立论", 2: "第二轮 · 交锋" };

function TurnCard({ agent, turn, live }: {
  agent?: AgentDiscussionConfig;
  turn?: DiscussionTurn;
  live?: DiscussionEvent;
}) {
  const running = live?.status === "running";
  const isError = live?.status === "error";
  const name = agent?.name || turn?.agent_name || live?.agent_name || "Agent";
  const meta = agent ? `${agent.model} · ${agent.perspective}` : turn?.perspective || "";
  const content = turn?.content || (isError ? `❌ ${live?.content}` : "");

  return (
    <div style={{ border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-md)", padding: "var(--space-md) var(--space-lg)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-sm)" }}>
        {running
          ? <Loader2 size={15} className="spinning" style={{ color: "var(--color-accent)" }} />
          : isError
            ? <XCircle size={15} style={{ color: "var(--color-error)" }} />
            : <Bot size={15} style={{ color: turn ? "var(--color-jade)" : "var(--color-ink-faint)" }} />}
        <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-sm)", letterSpacing: "1px", color: "var(--color-ink)" }}>{name}</span>
        <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>{meta}</span>
        <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
          {running ? "正在思考..." : turn ? "已发言" : isError ? "出错" : "等待中"}
        </span>
      </div>
      {content && (
        <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-2)", lineHeight: 1.8, whiteSpace: "pre-wrap", padding: "var(--space-sm)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
          {content}
        </div>
      )}
    </div>
  );
}

export function DiscussionPanel({ agents, turns, liveEvents, synthesis, discussing, onConfirmGenerate, generated }: DiscussionPanelProps) {
  const enabledAgents = agents.filter(a => a.enabled);
  const rounds = [1, 2];

  // 成果勾选状态：key = `${类别}-${序号}`
  const [checked, setChecked] = useState<Record<string, boolean>>({});

  const groups = useMemo(() => {
    if (!synthesis) return [];
    return [
      { key: "locations", label: "地点", icon: <MapPin size={14} />, items: synthesis.locations.map(i => ({ title: i.name, desc: i.description })), target: "世界观" },
      { key: "timeline_events", label: "时间线", icon: <Clock size={14} />, items: synthesis.timeline_events.map(i => ({ title: i.story_time, desc: i.description })), target: "世界观" },
      { key: "setting_rules", label: "设定规则", icon: <BookOpen size={14} />, items: synthesis.setting_rules.map(i => ({ title: i.name, desc: i.description })), target: "世界观" },
      { key: "characters", label: "人物", icon: <Users size={14} />, items: synthesis.characters.map(i => ({ title: i.name, desc: i.description || i.personality_traits.map(t => t[0]).join("、") })), target: "人物志" },
      { key: "outline_beats", label: "情节脉络", icon: <ListOrdered size={14} />, items: (synthesis.outline_beats ?? []).map(i => ({ title: i.chapter_hint ? `${i.title}（${i.chapter_hint}）` : i.title, desc: i.description })), target: "大纲" },
    ].filter(g => g.items.length > 0);
  }, [synthesis]);

  const totalItems = groups.reduce((n, g) => n + g.items.length, 0);
  const uncheckedCount = Object.values(checked).filter(v => v === false).length;
  const selectedCount = totalItems - uncheckedCount;

  const handleConfirm = () => {
    if (!synthesis) return;
    const pick = <T,>(key: string, items: T[]): T[] => items.filter((_, i) => checked[`${key}-${i}`] !== false);
    onConfirmGenerate({
      locations: pick("locations", synthesis.locations),
      timeline_events: pick("timeline_events", synthesis.timeline_events),
      setting_rules: pick("setting_rules", synthesis.setting_rules),
      characters: pick("characters", synthesis.characters),
      outline_beats: pick("outline_beats", synthesis.outline_beats ?? []),
    });
  };

  const hasAnyTurn = turns.length > 0 || Object.keys(liveEvents).length > 0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-xl)" }}>
      {/* ── 讨论过程（按轮次分组，实时更新）── */}
      {hasAnyTurn && (
        <div style={{ background: "var(--color-paper)", border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)", boxShadow: "var(--shadow-subtle)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)", borderBottom: "1px solid var(--color-rule-light)" }}>
            {discussing ? <Loader2 size={18} className="spinning" style={{ color: "var(--color-accent)" }} /> : <CheckCircle2 size={18} style={{ color: "var(--color-jade)" }} />}
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>
              {discussing ? "讨论进行中" : "讨论过程"}
            </span>
          </div>
          {rounds.map(round => {
            const roundTurns = turns.filter(t => t.round === round);
            const roundLive = Object.values(liveEvents).filter(e => e.round === round);
            // 该轮是否已有任何内容（事件或发言）
            const hasContent = roundTurns.length > 0 || roundLive.length > 0;
            if (!hasContent && round === 2) return null;
            return (
              <div key={round} style={{ marginBottom: "var(--space-lg)" }}>
                <div style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-accent)", letterSpacing: "1px", marginBottom: "var(--space-sm)" }}>
                  {ROUND_LABELS[round]}
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
                  {enabledAgents.map(agent => {
                    const turn = roundTurns.find(t => t.agent_id === agent.id);
                    const live = liveEvents[`${agent.id}-${round}`];
                    if (!turn && !live) return null;
                    return <TurnCard key={agent.id} agent={agent} turn={turn} live={live} />;
                  })}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* ── 讨论成果（结构化，确认后写入）── */}
      {synthesis && (
        <div style={{ background: "var(--color-paper)", border: "1px solid var(--color-jade)", borderRadius: "var(--radius-md)", padding: "var(--space-lg) var(--space-xl)", boxShadow: "var(--shadow-subtle)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-md)", paddingBottom: "var(--space-sm)", borderBottom: "1px solid var(--color-rule-light)" }}>
            <Sparkles size={18} style={{ color: "var(--color-jade)" }} />
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px", color: "var(--color-ink)" }}>讨论成果</span>
            <span style={{ marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-ink-3)" }}>
              勾选要采纳的条目，确认后写入世界观、人物志与大纲
            </span>
          </div>

          {synthesis.summary && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-2)", lineHeight: 1.8, padding: "var(--space-sm) var(--space-md)", background: "var(--color-jade-wash)", borderRadius: "var(--radius-sm)", marginBottom: "var(--space-md)", whiteSpace: "pre-wrap" }}>
              {synthesis.summary}
            </div>
          )}

          {groups.length === 0 ? (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", fontStyle: "italic" }}>
              本次讨论未提炼出可结构化的条目
            </div>
          ) : (
            <>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)", marginBottom: "var(--space-md)" }}>
                {groups.map(g => (
                  <div key={g.key}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)", marginBottom: 6 }}>
                      {g.icon} {g.label}
                      <span style={{ fontWeight: 400, color: "var(--color-ink-3)" }}>→ 写入{g.target}</span>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      {g.items.map((item, i) => {
                        const key = `${g.key}-${i}`;
                        const isChecked = checked[key] !== false; // 默认勾选
                        return (
                          <label key={key} style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-sm)", padding: "var(--space-sm) var(--space-md)", background: isChecked ? "var(--color-accent-wash)" : "transparent", borderRadius: "var(--radius-sm)", cursor: "pointer" }}>
                            <input type="checkbox" checked={isChecked} style={{ marginTop: 3 }}
                              onChange={() => setChecked(prev => ({ ...prev, [key]: !isChecked }))} />
                            <div style={{ flex: 1, minWidth: 0 }}>
                              <div style={{ fontSize: "var(--text-sm)", fontWeight: 500, color: "var(--color-ink)" }}>{item.title}</div>
                              <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", lineHeight: 1.6 }}>{item.desc}</div>
                            </div>
                          </label>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-md)" }}>
                <button className="btn btn-primary" onClick={handleConfirm} disabled={generated || selectedCount === 0}>
                  <CheckCircle2 size={15} /> {generated ? "已生成" : `确认生成（${selectedCount} 项）`}
                </button>
                {generated && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-jade)" }}>
                    已写入世界观、人物志与大纲，可前往对应页面查看
                  </span>
                )}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
