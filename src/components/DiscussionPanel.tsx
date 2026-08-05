import { useEffect, useMemo, useState } from "react";
import { Bot, CheckCircle2, Loader2, XCircle, MapPin, Clock, BookOpen, Users, Sparkles, ListOrdered, Scale, PenLine, RotateCcw, Send, Target, GitBranch } from "lucide-react";
import type { DiscussionTurn, DiscussionSynthesis, DiscussionEvent, AgentDiscussionConfig } from "../types";

interface DiscussionPanelProps {
  agents: AgentDiscussionConfig[];
  turns: DiscussionTurn[];
  liveEvents: Record<string, DiscussionEvent>; // key: `${agent_id}-${round}`
  synthesis: DiscussionSynthesis | null;
  discussing: boolean;
  onConfirmGenerate: (selected: SelectedResults, authorFeedback: string) => void;
  // 单独提交作者意见（不触发生成），供后续确认生成时一并记录
  onSubmitFeedback: (feedback: string) => void;
  // 清空当前讨论结果，重新发起讨论
  onRestartDiscussion: () => void;
  generated: boolean;
}

export interface SelectedResults {
  locations: Array<{
    name: string;
    description: string;
    level?: string;
    region?: string;
    faction?: string;
    unlocked_chapter?: string;
    sources?: string[];
  }>;
  timeline_events: Array<{ story_time: string; description: string; participants?: string[]; sources?: string[] }>;
  setting_rules: Array<{ name: string; description: string; constraints?: string[]; cost?: string; loophole?: string; sources?: string[] }>;
  characters: Array<{
    name: string;
    personality_traits: Array<[string, number]>;
    current_mood?: string;
    description?: string;
    relationships?: Array<{ from: string; to: string; relation_type: string; strength: number }>;
    wants?: string;
    fears?: string;
    secret?: string;
    speech_style?: string;
    arc?: Array<{ name: string; chapter_range?: string; trait_desc?: string; goal?: string }>;
    knows?: string[];
    does_not_know?: string[];
    sources?: string[];
  }>;
  outline_beats: Array<{
    title: string;
    description: string;
    chapter_hint?: string;
    volume?: string;
    beat_type?: string;
    hook?: string;
    payoff?: string;
    emotion_arc?: string;
    line_tags?: string[];
    foreshadowing?: Array<{ plant: string; payoff_hint?: string }>;
    sources?: string[];
  }>;
}

// 作者确认信息：同意标记 + 补充意见（至少其一才能生成）
export interface AuthorConfirmation {
  agreed: boolean;
  feedback: string;
}

const ROUND_LABELS: Record<number, string> = { 1: "第一轮 · 立论", 2: "第二轮 · 交锋", 3: "第三轮 · 成果提炼" };

function TurnCard({ agent, turn, live }: {
  agent?: AgentDiscussionConfig;
  turn?: DiscussionTurn;
  live?: DiscussionEvent;
}) {
  const running = live?.status === "running";
  const isError = live?.status === "error";
  const name = agent?.name || turn?.agent_name || live?.agent_name || "评审员";
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

export function DiscussionPanel({ agents, turns, liveEvents, synthesis, discussing, onConfirmGenerate, onSubmitFeedback, onRestartDiscussion, generated }: DiscussionPanelProps) {
  const enabledAgents = agents.filter(a => a.enabled);
  const rounds = [1, 2];

  // 成果勾选状态：key = `${类别}-${序号}`
  const [checked, setChecked] = useState<Record<string, boolean>>({});
  // 作者确认：是否同意讨论结果 / 补充意见（两者至少其一）
  const [agreed, setAgreed] = useState(false);
  const [feedback, setFeedback] = useState("");
  const [feedbackSubmitted, setFeedbackSubmitted] = useState(false);

  // 重新讨论/结果被清空时，重置勾选与意见状态
  useEffect(() => {
    if (!synthesis) {
      setChecked({});
      setAgreed(false);
      setFeedback("");
      setFeedbackSubmitted(false);
    }
  }, [synthesis]);

  const groups = useMemo(() => {
    if (!synthesis) return [];
    // 合成一行附加信息（空段自动省略），把新增字段轻量展示出来
    const meta = (parts: Array<string | undefined>) => parts.filter(Boolean).join(" · ");
    return [
      {
        key: "locations",
        label: "地点",
        icon: <MapPin size={14} />,
        items: synthesis.locations.map(i => ({
          title: i.name,
          desc: i.description,
          meta: meta([i.level, i.region, i.faction].concat(i.unlocked_chapter ? [`解锁：${i.unlocked_chapter}`] : [])),
        })),
        target: "世界观",
      },
      {
        key: "timeline_events",
        label: "时间线",
        icon: <Clock size={14} />,
        items: synthesis.timeline_events.map(i => ({
          title: i.story_time,
          desc: i.description,
          meta: meta([i.participants?.join("、")]),
        })),
        target: "世界观",
      },
      {
        key: "setting_rules",
        label: "设定规则",
        icon: <BookOpen size={14} />,
        items: synthesis.setting_rules.map(i => ({
          title: i.name,
          desc: i.description,
          meta: meta([
            i.constraints?.length ? `约束：${i.constraints.join("；")}` : undefined,
            i.cost ? `代价：${i.cost}` : undefined,
            i.loophole ? `漏洞：${i.loophole}` : undefined,
          ]),
        })),
        target: "世界观",
      },
      {
        key: "characters",
        label: "人物",
        icon: <Users size={14} />,
        items: synthesis.characters.map(i => ({
          title: i.name,
          desc: i.description || i.personality_traits.map(t => t[0]).join("、"),
          meta: meta([
            i.wants ? `欲望：${i.wants}` : undefined,
            i.fears ? `恐惧：${i.fears}` : undefined,
            i.speech_style ? `说话方式：${i.speech_style}` : undefined,
            i.arc?.length ? `弧线：${i.arc.map(a => a.name).join("→")}` : undefined,
          ]),
        })),
        target: "人物志",
      },
      {
        key: "outline_beats",
        label: "情节脉络",
        icon: <ListOrdered size={14} />,
        items: (synthesis.outline_beats ?? []).map(i => ({
          title: i.chapter_hint ? `${i.title}（${i.chapter_hint}）` : i.title,
          desc: i.description,
          meta: meta([
            i.volume ? `卷：${i.volume}` : undefined,
            i.beat_type ? `类型：${i.beat_type}` : undefined,
            i.hook ? `钩子：${i.hook}` : undefined,
            i.payoff ? `爽点：${i.payoff}` : undefined,
            i.line_tags?.length ? `线路：${i.line_tags.join("/")}` : undefined,
          ]),
        })),
        target: "大纲",
      },
    ].filter(g => g.items.length > 0);
  }, [synthesis]);

  const totalItems = groups.reduce((n, g) => n + g.items.length, 0);
  const uncheckedCount = Object.values(checked).filter(v => v === false).length;
  const selectedCount = totalItems - uncheckedCount;

  const canConfirm = agreed || feedback.trim().length > 0;

  const handleConfirm = () => {
    if (!synthesis || !canConfirm) return;
    const pick = <T,>(key: string, items: T[]): T[] => items.filter((_, i) => checked[`${key}-${i}`] !== false);
    onConfirmGenerate({
      locations: pick("locations", synthesis.locations),
      timeline_events: pick("timeline_events", synthesis.timeline_events),
      setting_rules: pick("setting_rules", synthesis.setting_rules),
      characters: pick("characters", synthesis.characters),
      outline_beats: pick("outline_beats", synthesis.outline_beats ?? []),
    }, agreed ? (feedback.trim() || "同意讨论结果") : feedback.trim());
  };

  // 提交意见：保存到萌芽数据（不生成），提示后仍可继续确认生成
  const handleSubmitFeedback = () => {
    const text = feedback.trim();
    if (!text) return;
    onSubmitFeedback(text);
    setFeedbackSubmitted(true);
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
          {[...rounds, 3].map(round => {
            const roundTurns = turns.filter(t => t.round === round);
            const roundLive = Object.values(liveEvents).filter(e => e.round === round);
            // 该轮是否已有任何内容（事件或发言）
            const hasContent = roundTurns.length > 0 || roundLive.length > 0;
            if (!hasContent && round === 2) return null;
            // 第三轮：成果提炼进度（分维度提炼 / 冲突检查 / 裁决），不是评审 Agent 发言
            if (round === 3) {
              if (roundLive.length === 0) return null;
              return (
                <div key={round} style={{ marginBottom: "var(--space-lg)" }}>
                  <div style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-accent)", letterSpacing: "1px", marginBottom: "var(--space-sm)" }}>
                    {ROUND_LABELS[round]}
                  </div>
                  <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-xs)" }}>
                    {roundLive.map(ev => {
                      const running = ev.status === "running";
                      const isError = ev.status === "error";
                      return (
                        <div key={ev.agent_id} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--text-xs)", color: "var(--color-ink-2)" }}>
                          {running
                            ? <Loader2 size={13} className="spinning" style={{ color: "var(--color-accent)" }} />
                            : isError
                              ? <XCircle size={13} style={{ color: "var(--color-error)" }} />
                              : <CheckCircle2 size={13} style={{ color: "var(--color-jade)" }} />}
                          <span style={{ color: "var(--color-ink)" }}>{ev.agent_name}</span>
                          <span style={{ color: "var(--color-ink-3)" }}>
                            {running ? "提炼中..." : isError ? `失败：${ev.content}` : ev.content || "完成"}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            }
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
            <button className="btn btn-secondary" style={{ padding: "4px 10px", fontSize: "var(--text-xs)" }}
              onClick={onRestartDiscussion} disabled={discussing}
              title="清空本次讨论结果，重新发起讨论">
              <RotateCcw size={12} style={{ marginRight: 4, verticalAlign: -1 }} /> 重新讨论
            </button>
          </div>

          {synthesis.summary && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-2)", lineHeight: 1.8, padding: "var(--space-sm) var(--space-md)", background: "var(--color-jade-wash)", borderRadius: "var(--radius-sm)", marginBottom: "var(--space-md)", whiteSpace: "pre-wrap" }}>
              {synthesis.summary}
            </div>
          )}

          {(synthesis.commitments?.length ?? 0) > 0 && (
            <div style={{ border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-sm)", padding: "var(--space-sm) var(--space-md)", marginBottom: "var(--space-md)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)", marginBottom: 6 }}>
                <Target size={13} style={{ color: "var(--color-jade)" }} /> 承诺与卖点
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-2)", lineHeight: 1.7 }}>
                {(synthesis.commitments ?? []).map((c, i) => (
                  <div key={i}>· <b>{c.statement}</b> <span style={{ color: "var(--color-ink-3)" }}>（{c.kind ?? "rule"} · {c.scope ?? "book"}）</span></div>
                ))}
              </div>
            </div>
          )}

          {(synthesis.quality_notes?.length ?? 0) > 0 && (
            <div style={{ border: "1px dashed var(--color-accent)", borderRadius: "var(--radius-sm)", padding: "var(--space-sm) var(--space-md)", marginBottom: "var(--space-md)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-accent)", marginBottom: 4 }}>
                <PenLine size={13} /> 共识复核与质量提示
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--text-2xs)", color: "var(--color-ink-2)", lineHeight: 1.7 }}>
                {(synthesis.quality_notes ?? []).map((n, i) => <div key={i}>· {n}</div>)}
              </div>
            </div>
          )}

          {(synthesis.disagreements?.length ?? 0) > 0 && (
            <div style={{ border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-sm)", padding: "var(--space-sm) var(--space-md)", marginBottom: "var(--space-md)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)", marginBottom: 6 }}>
                <Scale size={14} style={{ color: "var(--color-accent)" }} /> 分歧与裁决
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {synthesis.disagreements!.map((d, i) => (
                  <div key={i} style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-2)", lineHeight: 1.7, padding: "var(--space-xs) var(--space-sm)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                    <div style={{ fontWeight: 600, color: "var(--color-ink)" }}>
                      {d.topic}
                      {d.dimension && <span style={{ fontWeight: 400, color: "var(--color-ink-3)" }}> · {d.dimension}</span>}
                      <span style={{ marginLeft: 6, color: d.status === "resolved" ? "var(--color-jade)" : "var(--color-error)", fontWeight: 500 }}>
                        {d.status === "resolved" ? "已收敛" : d.adjudicated ? "已裁决" : "未收敛"}
                      </span>
                    </div>
                    {d.sides && d.sides.length > 0 && (
                      <div>
                        {d.sides.map((s, j) => (
                          <div key={j}>
                            {s.agent}：{s.position}
                            {s.rationale && <span style={{ color: "var(--color-ink-3)" }}>（{s.rationale}）</span>}
                          </div>
                        ))}
                      </div>
                    )}
                    {d.resolution && (
                      <div style={{ color: "var(--color-accent)", marginTop: 2 }}>
                        {d.adjudicated ? "裁决建议：" : "收敛结果："}{d.resolution}
                      </div>
                    )}
                    {d.alternatives && d.alternatives.length > 0 && (
                      <div style={{ color: "var(--color-ink-3)", marginTop: 2 }}>
                        备选路径：{d.alternatives.join(" ｜ ")}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {(synthesis.subplots?.length ?? 0) > 0 && (
            <div style={{ border: "1px solid var(--color-rule-light)", borderRadius: "var(--radius-sm)", padding: "var(--space-sm) var(--space-md)", marginBottom: "var(--space-md)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)", marginBottom: 6 }}>
                <GitBranch size={13} style={{ color: "var(--color-accent)" }} /> 副线
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {(synthesis.subplots ?? []).map((s, i) => (
                  <div key={i} style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-2)", lineHeight: 1.7, padding: "var(--space-xs) var(--space-sm)", background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)" }}>
                    <div style={{ fontWeight: 600, color: "var(--color-ink)" }}>
                      {s.name}
                      {s.chapter_range && <span style={{ fontWeight: 400, color: "var(--color-ink-3)" }}> · {s.chapter_range}</span>}
                    </div>
                    {s.description && <div>{s.description}</div>}
                    {s.mainline_relation && <div style={{ color: "var(--color-accent)" }}>与主线：{s.mainline_relation}</div>}
                    {(s.open_threads?.length ?? 0) > 0 && (
                      <div style={{ color: "var(--color-ink-3)" }}>未解问题：{s.open_threads!.join(" ｜ ")}</div>
                    )}
                  </div>
                ))}
              </div>
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
                              {item.meta && (
                                <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-accent)", lineHeight: 1.6, marginTop: 2 }}>
                                  {item.meta}
                                </div>
                              )}
                            </div>
                          </label>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-md)" }}>
                <button className="btn btn-primary" onClick={handleConfirm} disabled={generated || selectedCount === 0 || !canConfirm}
                  title={!canConfirm ? "请先勾选同意讨论结果，或填写补充意见" : ""}>
                  <CheckCircle2 size={15} /> {generated ? "已生成" : `确认生成（${selectedCount} 项）`}
                </button>
                {generated && (
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--color-jade)" }}>
                    已写入世界观、人物志与大纲，可前往对应页面查看
                  </span>
                )}
              </div>
              {!generated && (
                <div style={{ marginTop: "var(--space-md)", borderTop: "1px solid var(--color-rule-light)", paddingTop: "var(--space-md)" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)", marginBottom: 6 }}>
                    <PenLine size={14} style={{ color: "var(--color-accent)" }} /> 作者确认
                  </div>
                  <label style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)", fontSize: "var(--text-xs)", color: "var(--color-ink-2)", marginBottom: "var(--space-sm)", cursor: "pointer" }}>
                    <input type="checkbox" checked={agreed} onChange={e => setAgreed(e.target.checked)} style={{ accentColor: "var(--color-accent)" }} />
                    我同意本次讨论结果
                  </label>
                  <textarea
                    className="pm-textarea"
                    rows={2}
                    placeholder="补充意见（可选）：对讨论结果不满意的地方、希望调整的方向，将随成果一并记录"
                    value={feedback}
                    onChange={e => setFeedback(e.target.value)}
                    style={{ minHeight: 56, fontSize: "var(--text-xs)", lineHeight: 1.7 }}
                  />
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 6 }}>
                    <button
                      className="btn btn-secondary"
                      style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}
                      onClick={handleSubmitFeedback}
                      disabled={!feedback.trim() || feedbackSubmitted}
                      title="先保存补充意见；后续点「确认生成」时意见会随成果一并写入"
                    >
                      <Send size={11} style={{ marginRight: 4, verticalAlign: -1 }} />
                      {feedbackSubmitted ? "意见已提交" : "提交意见"}
                    </button>
                    {feedbackSubmitted && (
                      <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-jade)" }}>
                        意见已保存，可继续勾选条目后确认生成
                      </span>
                    )}
                  </div>
                  {!canConfirm && (
                    <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginTop: 4 }}>
                      至少勾选「我同意」或填写补充意见后才能生成
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
