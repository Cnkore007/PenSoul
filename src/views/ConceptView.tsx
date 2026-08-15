// ConceptView — 萌芽（对话式创作工作台）
// 用户输入想法与设定，LLM 以可选视角参与讨论；
// 对话成熟后生成结构化提案，确认后才写入正典

import { useState, useEffect, useCallback, useRef } from "react";
import {
  getSproutSession,
  sproutStart,
  sproutChat,
  sproutGenerate,
  sproutApply,
  sproutDiscard,
  sproutClear,
  getConcept,
  listRules,
  listOutlineArcs,
} from "../ipc";
import type {
  CoreConcept,
  OutlineArc,
  SproutProposal,
  SproutSession,
} from "../types";

const PERSPECTIVES = ["综合", "结构", "人物", "世界观"] as const;
type Perspective = (typeof PERSPECTIVES)[number];

const CONCEPT_FIELDS: { key: keyof CoreConcept; label: string }[] = [
  { key: "high_concept", label: "高概念" },
  { key: "premise", label: "故事前提" },
  { key: "protagonist_hint", label: "主角" },
  { key: "tone", label: "基调" },
  { key: "central_conflict", label: "核心冲突" },
  { key: "inspiration", label: "灵感来源" },
];

export default function ConceptView() {
  const [session, setSession] = useState<SproutSession | null>(null);
  const [concept, setConcept] = useState<CoreConcept | null>(null);
  const [rules, setRules] = useState<string[]>([]);
  const [arcs, setArcs] = useState<OutlineArc[]>([]);
  const [input, setInput] = useState("");
  const [perspective, setPerspective] = useState<Perspective>("综合");
  const [chatLoading, setChatLoading] = useState(false);
  const [genLoading, setGenLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [starting, setStarting] = useState(false);
  const [msg, setMsg] = useState("");
  const bottomRef = useRef<HTMLDivElement | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [s, c, r, a] = await Promise.all([
        getSproutSession(),
        getConcept(),
        listRules(),
        listOutlineArcs(),
      ]);
      setSession(s);
      setConcept(c);
      setRules(r);
      setArcs(a);
    } catch (e: any) {
      setMsg(`加载失败: ${e}`);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [session?.messages.length, chatLoading]);

  const handleSend = async () => {
    const message = input.trim();
    if (!message || chatLoading) return;
    setChatLoading(true);
    setMsg("");
    // 本地乐观显示用户消息，刷新后以后端为准
    setSession((s) => s && {
      ...s,
      messages: [...s.messages, { role: "user", content: message, created_at: "" }],
    });
    setInput("");
    try {
      const reply = await sproutChat(message, perspective);
      setSession((s) => s && {
        ...s,
        messages: [
          ...s.messages,
          { role: "assistant", content: reply.content, created_at: "" },
        ],
      });
    } catch (e: any) {
      setMsg(`对话失败: ${e}`);
      refresh();
    } finally {
      setChatLoading(false);
    }
  };

  const handleStart = async () => {
    if (starting) return;
    setStarting(true);
    setMsg("");
    try {
      const reply = await sproutStart();
      setSession((s) => s && {
        ...s,
        messages: [{ role: "assistant", content: reply.content, created_at: "" }],
      });
    } catch (e: any) {
      setMsg(`开始诘问失败: ${e}`);
    } finally {
      setStarting(false);
    }
  };

  const handleGenerate = async () => {
    if (genLoading || chatLoading) return;
    setGenLoading(true);
    setMsg("");
    try {
      const proposal = await sproutGenerate();
      setSession((s) => s && { ...s, pending_proposal: proposal });
    } catch (e: any) {
      setMsg(`生成提案失败: ${e}`);
    } finally {
      setGenLoading(false);
    }
  };

  const handleApply = async () => {
    if (applying) return;
    setApplying(true);
    setMsg("");
    try {
      await sproutApply();
      setSession((s) => s && { ...s, pending_proposal: null });
      setMsg("提案已写入正典");
      refresh();
    } catch (e: any) {
      setMsg(`应用提案失败: ${e}`);
    } finally {
      setApplying(false);
    }
  };

  const handleDiscard = async () => {
    try {
      await sproutDiscard();
      setSession((s) => s && { ...s, pending_proposal: null });
    } catch (e: any) {
      setMsg(`拒绝提案失败: ${e}`);
    }
  };

  const handleClear = async () => {
    if (!confirm("确定清空萌芽对话吗？已写入正典的成果不会丢失。")) return;
    try {
      await sproutClear();
      setSession((s) => s && { ...s, messages: [], pending_proposal: null });
      setMsg("对话已清空");
    } catch (e: any) {
      setMsg(`清空失败: ${e}`);
    }
  };

  if (!session) {
    return <div className="view-card"><p>{msg || "加载中..."}</p></div>;
  }

  return (
    <div className="view-card">
      <h2>萌芽</h2>
      <p className="sprout-intro">
        AI 会像资深编辑一样诘问你的故事：一次只问一个问题，你回答后它提炼要点、
        接着追问下一题。回答完所有问题后生成提案，确认后写入正典。
      </p>
      {msg && <p className="msg">{msg}</p>}

      <div className="sprout-layout">
        <div className="sprout-main">
          {/* 视角选择 */}
          <div className="sprout-perspectives">
            {PERSPECTIVES.map((p) => (
              <button
                key={p}
                className={`btn-sm ${perspective === p ? "btn-primary" : ""}`}
                onClick={() => setPerspective(p)}
              >
                {p}
              </button>
            ))}
          </div>

          {/* 对话区 */}
          <div className="sprout-messages">
            {session.messages.length === 0 ? (
              <div className="sprout-start">
                <p className="sprout-muted">
                  准备好后点「开始诘问」，AI 会提出第一个问题；之后你只需逐题回答。
                </p>
                <button
                  className="btn-primary"
                  onClick={handleStart}
                  disabled={starting}
                >
                  {starting ? "准备中…" : "开始诘问"}
                </button>
              </div>
            ) : (
              session.messages.map((m, i) => (
                <div key={i} className={`sprout-msg ${m.role}`}>
                  <div className="sprout-bubble">{m.content}</div>
                </div>
              ))
            )}
            {chatLoading && (
              <div className="sprout-msg assistant">
                <div className="sprout-bubble sprout-thinking">思考中…</div>
              </div>
            )}
            <div ref={bottomRef} />
          </div>

          {/* 输入区 */}
          <div className="sprout-input-row">
            <textarea
              className="ps-textarea sprout-input"
              placeholder="回答当前问题（Shift + Enter 换行）"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSend();
                }
              }}
            />
            <button className="btn-primary" onClick={handleSend} disabled={chatLoading || !input.trim()}>
              回答
            </button>
          </div>

          {/* 动作区 */}
          <div className="sprout-actions">
            <button
              className="btn-primary btn-sm"
              onClick={handleGenerate}
              disabled={genLoading || chatLoading || session.messages.length === 0}
            >
              {genLoading ? "生成中…" : "素材够了，生成提案"}
            </button>
            <button className="btn-sm" onClick={handleClear} disabled={session.messages.length === 0}>
              清空对话
            </button>
          </div>
        </div>

        {/* 当前成果（正典只读摘要） */}
        <aside className="sprout-side">
          <h3>当前成果</h3>
          <div className="sprout-side-section">
            <h4>核心概念</h4>
            {concept && CONCEPT_FIELDS.some((f) => concept[f.key]) ? (
              CONCEPT_FIELDS.map(({ key, label }) =>
                concept[key] ? (
                  <p key={key}><b>{label}：</b>{concept[key]}</p>
                ) : null,
              )
            ) : (
              <p className="sprout-muted">尚未生成</p>
            )}
          </div>
          <div className="sprout-side-section">
            <h4>世界观规则</h4>
            {rules.length ? (
              <ul>{rules.map((r, i) => <li key={i}>{r}</li>)}</ul>
            ) : (
              <p className="sprout-muted">尚未生成</p>
            )}
          </div>
          <div className="sprout-side-section">
            <h4>大纲脉络</h4>
            {arcs.length ? (
              <ul>
                {arcs.map((a) => (
                  <li key={a.arc_id}>
                    <b>{a.title}</b>（{a.chapter_start}-{a.chapter_end} 章）
                    {a.description ? <p className="sprout-muted">{a.description}</p> : null}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="sprout-muted">尚未生成</p>
            )}
          </div>
        </aside>
      </div>

      {session.pending_proposal && (
        <ProposalCard
          proposal={session.pending_proposal}
          applying={applying}
          onApply={handleApply}
          onDiscard={handleDiscard}
        />
      )}
    </div>
  );
}

function ProposalCard({
  proposal,
  applying,
  onApply,
  onDiscard,
}: {
  proposal: SproutProposal;
  applying: boolean;
  onApply: () => void;
  onDiscard: () => void;
}) {
  return (
    <div className="sprout-proposal">
      <h3>待确认提案</h3>
      <p className="sprout-muted">这是 AI 根据对话整理的结构化提案，确认后才会写入正典。</p>

      <div className="proposal-grid">
        {[
          ["高概念", proposal.high_concept],
          ["故事前提", proposal.premise],
          ["主角", proposal.protagonist_hint],
          ["基调", proposal.tone],
          ["核心冲突", proposal.central_conflict],
          ["灵感来源", proposal.inspiration],
          ["题材", proposal.genre],
        ].map(([label, value]) => (
          <div key={label} className="proposal-item">
            <b>{label}</b>
            <p>{value || "未填写"}</p>
          </div>
        ))}
      </div>

      <div className="proposal-section">
        <h4>世界观规则</h4>
        {proposal.world_rules.length ? (
          <ul>{proposal.world_rules.map((r, i) => <li key={i}>{r}</li>)}</ul>
        ) : (
          <p className="sprout-muted">无</p>
        )}
      </div>

      <div className="proposal-section">
        <h4>世界观设定</h4>
        {proposal.world_settings.length ? (
          <ul>
            {proposal.world_settings.map((s, i) => (
              <li key={i}>
                <b>{s.name}</b>（{s.category || "未分类"}）
                {s.description ? <p className="sprout-muted">{s.description}</p> : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="sprout-muted">无</p>
        )}
      </div>

      <div className="proposal-section">
        <h4>大纲脉络</h4>
        {proposal.outline_arcs.length ? (
          <ul>
            {proposal.outline_arcs.map((a, i) => (
              <li key={i}>
                <b>{a.title}</b>（{a.chapter_start}-{a.chapter_end} 章）
                {a.description ? <p className="sprout-muted">{a.description}</p> : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="sprout-muted">无</p>
        )}
      </div>

      <div className="btn-group proposal-actions">
        <button className="btn-primary btn-sm" onClick={onApply} disabled={applying}>
          {applying ? "写入中…" : "确认写入正典"}
        </button>
        <button className="btn-sm" onClick={onDiscard} disabled={applying}>拒绝提案</button>
      </div>
    </div>
  );
}
