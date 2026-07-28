import { useState, useEffect, useCallback, useRef } from "react";
import { Play, CheckCircle, AlertCircle, ArrowRight, Settings, Clock, Zap, RotateCcw, Loader2, Workflow, Brain, MessageSquare } from "lucide-react";
import type { ProjectData, PluginConfig, ViewType } from "../types";
import { builtinWorkflows } from "./WorkflowView";

interface HarnessConsoleProps {
  projectData: ProjectData;
  onNavigate?: (view: ViewType) => void;
}

interface StageProgress {
  status: "pending" | "in_progress" | "completed" | "failed";
  started_at: string | null;
  completed_at: string | null;
}

interface HarnessProgress {
  current_stage_index: number;
  stages: Record<string, StageProgress>;
  started_at: string;
}

interface AgentMessage {
  id: string;
  type: "think" | "action" | "output" | "complete" | "error";
  content: string;
  stageName?: string;
  streaming?: boolean;
}

function loadProgress(projectId: string): HarnessProgress | null {
  try {
    const raw = localStorage.getItem("pensoul_harness_" + projectId);
    if (raw) return JSON.parse(raw);
  } catch {}
  return null;
}

function saveProgress(projectId: string, progress: HarnessProgress) {
  localStorage.setItem("pensoul_harness_" + projectId, JSON.stringify(progress));
}

const gateLabels: Record<string, string> = {
  auto: "自动放行",
  manual: "人工放行",
  conditional: "条件放行",
};

const gateIcons: Record<string, React.ReactNode> = {
  auto: <Zap size={13} />,
  manual: <AlertCircle size={13} />,
  conditional: <Clock size={13} />,
};

// ── Agent 思考模拟脚本 ──
// 每个阶段的思考步骤序列：think = 思考要点，action = 执行动作，output = 流式输出样本

// ── 构建项目上下文摘要 ──
// 从 projectData 中提取设置、大纲、世界观、角色信息，供 Agent 使用
function buildProjectContext(projectData: ProjectData): string {
  const parts: string[] = [];

  // 基础设置
  const s = projectData.settings;
  parts.push(`【项目设置】
类型: ${s.genre || '未设定'}
目标总章数: ${s.targetChapters || '未设定'}
目标总字数: ${s.targetWords || '未设定'}
每章目标字数: ${s.chapterTargetWords || '未设定'}
预计卷数: ${s.targetVolumes || '未设定'}`);

  // 大纲结构
  if (projectData.volumes.length > 0) {
    const outlineLines: string[] = [];
    for (const vol of projectData.volumes) {
      outlineLines.push(`\n${vol.title || '未命名卷'}:`);
      for (const ch of vol.chapters) {
        const status = ch.status === 'Published' ? '✓' : ch.status === 'Draft' ? '○' : '◐';
        outlineLines.push(`  ${status} 第${ch.chapter_id.split('-').pop()}章 ${ch.title || '未命名'} (${ch.word_count}字)`);
      }
    }
    parts.push(`【大纲结构】${outlineLines.join('\n')}`);
  } else {
    parts.push('【大纲结构】暂无章节');
  }

  // 世界观
  const w = projectData.world;
  if (w.locations.length > 0 || w.timeline_events.length > 0 || w.setting_rules.length > 0) {
    const worldLines: string[] = [];
    if (w.locations.length > 0) {
      worldLines.push('地点:');
      for (const loc of w.locations) {
        worldLines.push(`  - ${loc.name}: ${loc.description || '暂无描述'}`);
      }
    }
    if (w.timeline_events.length > 0) {
      worldLines.push('时间线:');
      for (const evt of w.timeline_events) {
        worldLines.push(`  - [${evt.story_time}] ${evt.description}`);
      }
    }
    if (w.setting_rules.length > 0) {
      worldLines.push('设定规则:');
      for (const rule of w.setting_rules) {
        worldLines.push(`  - ${rule.title}: ${rule.description}`);
      }
    }
    parts.push(`【世界观】\n${worldLines.join('\n')}`);
  } else {
    parts.push('【世界观】暂未设定');
  }

  // 角色
  if (projectData.characters.length > 0) {
    const charLines: string[] = [];
    for (const c of projectData.characters) {
      const traits = c.personality_traits.map(([t, v]) => `${t}(${(v * 100).toFixed(0)}%)`).join(', ');
      charLines.push(`  - ${c.name}: ${traits}${c.current_mood ? ', 当前情绪: ' + c.current_mood : ''}`);
    }
    parts.push(`【角色设定】\n${charLines.join('\n')}`);
  } else {
    parts.push('【角色设定】暂未设定');
  }

  // 已有章节摘要（取最近 2 章的内容摘要）
  const allChapters = projectData.volumes.flatMap(v => v.chapters);
  if (allChapters.length > 0) {
    const recent = allChapters.slice(-2);
    const chapterSummaries = recent.map(ch => {
      const preview = ch.content ? ch.content.slice(0, 200) + (ch.content.length > 200 ? '...' : '') : '暂无内容';
      return `  - ${ch.title || '未命名'}: ${preview}`;
    });
    parts.push(`【最近章节摘要】\n${chapterSummaries.join('\n')}`);
  }

  return parts.join('\n\n');
}

// 根据阶段名和上下文动态生成 Agent 脚本
function buildStageScript(
  stageName: string,
  projectData: ProjectData,
): Array<{ type: "think" | "action" | "output"; content: string }> {
  const s = projectData.settings;
  const totalChapters = projectData.volumes.reduce((sum, v) => sum + v.chapters.length, 0);
  const totalWords = projectData.volumes.reduce((sum, v) => sum + v.chapters.reduce((s2, c) => s2 + c.word_count, 0), 0);
  const allChapters = projectData.volumes.flatMap(v => v.chapters);
  const lastChapter = allChapters.length > 0 ? allChapters[allChapters.length - 1] : null;

  switch (stageName) {
    case "outline_generation":
      return [
        { type: "think", content: `读取项目设定: 类型=${s.genre || '未设定'}, 目标章数=${s.targetChapters || '未设定'}, 目标字数=${s.targetWords || '未设定'}` },
        { type: "think", content: `当前已有 ${totalChapters} 个章节, 共 ${totalWords} 字` },
        { type: "think", content: "分析故事核心冲突和主线脉络..." },
        { type: "action", content: "基于项目设置生成大纲框架..." },
        { type: "output", content: `根据项目设定（类型: ${s.genre || '未设定'}），建议按以下结构组织大纲：

共规划 ${s.targetVolumes || '待定'} 卷, ${s.targetChapters || '待定'} 章, 总字数目标 ${s.targetWords || '待定'} 字。

（此为框架建议，请在「大纲」中按此结构创建卷和章节。）` },
        { type: "think", content: "大纲框架已生成" },
      ];
    case "chapter_writing":
      return [
        { type: "think", content: `读取大纲: 共 ${totalChapters} 章, 已完成 ${totalWords} 字` },
        { type: "think", content: lastChapter ? `上一章「${lastChapter.title}」结尾: ${lastChapter.content ? lastChapter.content.slice(-100) : '无内容'}...` : "这是第一章" },
        { type: "think", content: "回顾角色状态和世界观设定..." },
        { type: "action", content: "撰写章节正文..." },
        { type: "output", content: lastChapter
          ? `接续上一章「${lastChapter.title}」的结尾，继续推进剧情。

（请在「写作」视图中查看和编辑完整内容。）`
          : `开始创作第一章。

（请在「写作」视图中查看和编辑完整内容。）` },
        { type: "think", content: "章节初稿完成" },
      ];
    case "consistency_check":
      return [
        { type: "think", content: `扫描 ${totalChapters} 个章节的一致性...` },
        { type: "think", content: "检查人物状态与世界观规则..." },
        { type: "action", content: "生成一致性审查报告..." },
        { type: "output", content: `一致性检查完成

共检查 ${totalChapters} 章, ${projectData.characters.length} 个角色, ${projectData.world.locations.length} 个地点

（详细结果请查看「一致性」视图。）` },
      ];
    case "style_review":
      return [
        { type: "think", content: `分析 ${totalWords} 字的文风特征...` },
        { type: "think", content: "检测 AI 写作痕迹和模式化表达..." },
        { type: "action", content: "生成文风诊断报告..." },
        { type: "output", content: `文风分析完成

总字数: ${totalWords}, 平均每章: ${totalChapters > 0 ? Math.round(totalWords / totalChapters) : 0} 字

（详细分析请查看「文风」视图。）` },
      ];
    case "state_injection":
      return [
        { type: "think", content: "解析最新章节的角色状态变化..." },
        { type: "think", content: "更新世界观演化记录..." },
        { type: "action", content: "回灌状态到记忆系统..." },
        { type: "output", content: `状态回灌完成

角色: ${projectData.characters.length} 个
地点: ${projectData.world.locations.length} 个
时间线事件: ${projectData.world.timeline_events.length} 个
设定规则: ${projectData.world.setting_rules.length} 个` },
      ];
    default:
      return [
        { type: "think", content: `分析任务「${stageName}」的上下文...` },
        { type: "action", content: "执行阶段任务..." },
        { type: "output", content: `「${stageName}」阶段完成。

项目概览: ${totalChapters} 章, ${totalWords} 字` },
      ];
  }
}


function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// 逐字流式输出
async function* streamText(text: string, chunkSize = 1, interval = 30) {
  let pos = 0;
  while (pos < text.length) {
    const end = Math.min(pos + chunkSize, text.length);
    yield text.slice(pos, end);
    pos = end;
    await sleep(interval);
  }
}

export function HarnessConsole({ projectData, onNavigate }: HarnessConsoleProps) {
  const workflowId = projectData.workflow_id;
  const workflow: PluginConfig | undefined = builtinWorkflows.find(w => w.plugin_id === workflowId);

  const [progress, setProgress] = useState<HarnessProgress | null>(() => loadProgress(projectData.project_id));
  const [advancing, setAdvancing] = useState(false);
  const [agentWorking, setAgentWorking] = useState(false);
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [streamingContent, setStreamingContent] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [confirmManual, setConfirmManual] = useState<number | null>(null);
  const [autoAdvance, setAutoAdvance] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingContent]);

  // 初始化进度
  useEffect(() => {
    if (!workflowId) {
      setProgress(null);
      setMessages([]);
      return;
    }
    const saved = loadProgress(projectData.project_id);
    if (saved) {
      setProgress(saved);
    } else {
      const initial: HarnessProgress = {
        current_stage_index: 0,
        stages: Object.fromEntries(
          (workflow?.stages || []).map((s, i) => [
            s.name,
            { status: i === 0 ? "in_progress" : "pending", started_at: i === 0 ? new Date().toISOString() : null, completed_at: null },
          ])
        ),
        started_at: new Date().toISOString(),
      };
      setProgress(initial);
      saveProgress(projectData.project_id, initial);
      setMessages([{
        id: "init",
        type: "think",
        content: `工作流「${workflow?.name}」已就绪，共 ${workflow?.stages.length || 0} 个阶段。点击「推进」开始。`,
      }]);
    }
  }, [workflowId, projectData.project_id, workflow]);

  const stageList = workflow?.stages || [];
  const currentStageIndex = progress?.current_stage_index ?? 0;
  const currentStage = stageList[currentStageIndex];
  const allCompleted = progress && stageList.length > 0 && stageList.every(s => progress.stages[s.name]?.status === "completed");

  // 执行一个阶段的流式对话
  const executeStageStreaming = useCallback(async (stageIdx: number) => {
    const stage = stageList[stageIdx];
    if (!stage) return;

    setAgentWorking(true);
    const stageName = stage.display_name;
    const script = buildStageScript(stage.name, projectData);

    // 添加阶段开始消息
    setMessages(prev => [...prev, {
      id: `stage-${stageIdx}-start`,
      type: "action",
      content: `开始阶段: ${stageName}`,
      stageName,
    }]);

    let msgId = 0;

    for (const step of script) {
      if (step.type === "think" || step.type === "action") {
        // 等待一段时间模拟思考
        await sleep(400 + Math.random() * 600);
        msgId++;
        setMessages(prev => [...prev, {
          id: `msg-${stageIdx}-${msgId}`,
          type: step.type,
          content: step.content,
          stageName,
        }]);
      } else if (step.type === "output") {
        // 等待准备输出
        await sleep(300);
        const outputId = `out-${stageIdx}-${msgId++}`;

        // 先添加空消息占位，然后用流式填充
        setMessages(prev => [...prev, {
          id: outputId,
          type: "output",
          content: "",
          stageName,
          streaming: true,
        }]);
        setIsStreaming(true);

        // 逐字流式输出
        let accumulated = "";
        for await (const chunk of streamText(step.content, 1, 25)) {
          accumulated += chunk;
          setStreamingContent(accumulated);
        }

        // 流式完成，更新为最终内容
        setMessages(prev => prev.map(m =>
          m.id === outputId ? { ...m, content: accumulated, streaming: false } : m
        ));
        setStreamingContent("");
        setIsStreaming(false);
        await sleep(200);
      }
    }

    // 阶段完成
    msgId++;
    setMessages(prev => [...prev, {
      id: `msg-${stageIdx}-done`,
      type: "complete",
      content: `「${stageName}」阶段完成`,
      stageName,
    }]);

    setAgentWorking(false);

    // 更新进度
    if (!progress) return;
    const updated: HarnessProgress = { ...progress, stages: { ...progress.stages } };
    updated.stages[stage.name] = {
      ...updated.stages[stage.name],
      status: "completed",
      completed_at: new Date().toISOString(),
    };

    if (stageIdx + 1 < stageList.length) {
      const nextName = stageList[stageIdx + 1].name;
      updated.stages[nextName] = {
        ...updated.stages[nextName],
        status: "in_progress",
        started_at: new Date().toISOString(),
      };
      updated.current_stage_index = stageIdx + 1;
    } else {
      updated.current_stage_index = stageIdx;
    }

    setProgress(updated);
    saveProgress(projectData.project_id, updated);
  }, [stageList, progress, projectData.project_id]);

  // 推进至下一阶段
  const handleAdvance = useCallback(async () => {
    if (!workflow || !currentStage || !progress || advancing || allCompleted) return;

    // 手动门控
    if (currentStage.gate === "manual" && confirmManual === null) {
      setConfirmManual(currentStageIndex);
      return;
    }

    // 检查是否有章节内容
    const totalChapters = projectData.volumes.reduce((s, v) => s + v.chapters.length, 0);
    if (totalChapters === 0 && currentStageIndex > 0) {
      setMessages(prev => [...prev, {
        id: `err-${Date.now()}`,
        type: "error",
        content: "请先在「大纲」中创建章节",
      }]);
      return;
    }

    setAdvancing(true);
    try {
      await executeStageStreaming(currentStageIndex);
    } catch (e) {
      console.error("advance failed", e);
      setMessages(prev => [...prev, {
        id: `err-${Date.now()}`,
        type: "error",
        content: `执行出错: ${e instanceof Error ? e.message : "未知错误"}`,
      }]);
    }
    setAdvancing(false);
    setConfirmManual(null);
  }, [workflow, currentStage, currentStageIndex, progress, advancing, allCompleted, projectData, executeStageStreaming, confirmManual]);

  // 自动推进模式
  const handleAutoAdvance = useCallback(async () => {
    if (!workflow || !progress) return;
    setAutoAdvance(true);
    let idx = currentStageIndex;
    while (idx < stageList.length) {
      const stage = stageList[idx];
      if (!stage) break;

      // 手动门控时停下来
      if (stage.gate === "manual") {
        setConfirmManual(idx);
        setAutoAdvance(false);
        return;
      }

      await executeStageStreaming(idx);

      // 检查是否已全部完成
      const isLast = idx === stageList.length - 1;
      if (isLast) break;

      // 等待一下再自动进入下一阶段
      await sleep(1000);
      idx++;
    }
    setAutoAdvance(false);
  }, [workflow, stageList, currentStageIndex, progress, executeStageStreaming]);

  const handleReset = useCallback(() => {
    if (progress) {
      const initial: HarnessProgress = {
        current_stage_index: 0,
        stages: Object.fromEntries(
          (workflow?.stages || []).map((s, i) => [
            s.name,
            { status: i === 0 ? "in_progress" : "pending", started_at: i === 0 ? new Date().toISOString() : null, completed_at: null },
          ])
        ),
        started_at: new Date().toISOString(),
      };
      setProgress(initial);
      saveProgress(projectData.project_id, initial);
      setMessages([{
        id: "reset",
        type: "think",
        content: "进度已重置，可以重新开始。」",
      }]);
      setStreamingContent("");
    }
  }, [workflow, projectData.project_id]);

  const btnDisabled = advancing || agentWorking || !workflow || !currentStage || autoAdvance;

  function handleManualConfirm() {
    handleAdvance();
  }

  function goToWorkflow() {
    onNavigate?.("workflow");
  }

  // 无工作流状态
  if (!workflow) {
    const totalChapters = projectData.volumes.reduce((s, v) => s + v.chapters.length, 0);
    return (
      <div className="view-container">
        <div className="view-header">
          <h2>造化工坊</h2>
        </div>
        <div className="card" style={{ padding: "var(--space-xl)", textAlign: "center" }}>
          <div className="empty-state" style={{ padding: "var(--space-2xl) 0" }}>
            <div style={{
              width: 64, height: 64, borderRadius: "50%",
              background: "var(--color-ochre-wash)", color: "var(--color-ochre)",
              display: "flex", alignItems: "center", justifyContent: "center",
              margin: "0 auto 16px", fontSize: 28,
            }}>
              <Workflow size={28} />
            </div>
            <div className="empty-state-text" style={{ marginBottom: 8 }}>尚未配置工作流</div>
            <div className="empty-state-sub" style={{ maxWidth: 400, margin: "0 auto 24px", lineHeight: 1.7 }}>
              Agent 需要你选择一个工作流模板，才知道如何自动推进创作。
            </div>
            {totalChapters === 0 && (
              <div style={{
                fontSize: "var(--text-xs)", color: "var(--color-ink-3)",
                marginBottom: 16, padding: "8px 16px",
                background: "var(--color-accent-wash)", borderRadius: "var(--radius-sm)",
                display: "inline-block",
              }}>
                提示：建议先在「大纲」中创建卷和章节
              </div>
            )}
            <button className="btn btn-primary" onClick={goToWorkflow} style={{ padding: "10px 28px" }}>
              <Settings size={16} /> 配置工作流
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>造化工坊</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-secondary" onClick={goToWorkflow}>
            <Settings size={14} /> 更换工作流
          </button>
          {progress && (
            <button className="btn btn-secondary" onClick={handleReset}>
              <RotateCcw size={14} /> 重置
            </button>
          )}
        </div>
      </div>

      {/* 工作流管线图 */}
      <div className="card" style={{ marginTop: "var(--space-md)" }}>
        <div className="card-header">
          <Play size={15} color="var(--color-accent)" />
          <h3>{workflow.name}</h3>
          <span className="tag tag-accent">{workflow.stages.length} 个阶段</span>
        </div>
        <div className="pv-pipeline">
          <div className="pv-flow">
            {workflow.stages.map((stage, idx) => {
              const st = progress?.stages[stage.name];
              const isCurrent = idx === currentStageIndex;
              const isDone = st?.status === "completed";
              return (
                <div key={stage.name} style={{ display: "flex", alignItems: "center" }}>
                  {idx > 0 && (
                    <div className="pv-connector">
                      <div className="pv-connector-line" style={{ background: isDone ? "var(--color-jade)" : undefined }} />
                      <ArrowRight size={12} className="pv-connector-arrow" />
                    </div>
                  )}
                  <div
                    className={`pv-node pv-node-stage ${isCurrent ? "pv-node-editing" : ""} ${isDone ? "pv-node-done" : ""}`}
                    style={{
                      borderColor: isDone ? "var(--color-jade)" : isCurrent ? "var(--color-accent)" : undefined,
                    }}
                  >
                    <div className="pv-stage-header">
                      <span className="pv-stage-number" style={{ background: isDone ? "var(--color-jade)" : isCurrent ? "var(--color-accent)" : undefined }}>
                        {isDone ? <CheckCircle size={12} /> : idx + 1}
                      </span>
                      <span className="pv-stage-name">{stage.display_name}</span>
                    </div>
                    <div className="pv-stage-tags">
                      <span className={`pv-tag pv-tag-gate pv-gate-${stage.gate}`}>
                        {gateIcons[stage.gate]} {gateLabels[stage.gate]}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}
            <div className="pv-connector">
              <div className="pv-connector-line" style={{ background: allCompleted ? "var(--color-jade)" : undefined }} />
              <ArrowRight size={12} className="pv-connector-arrow" />
            </div>
            <div className="pv-node pv-node-end" style={{ borderColor: allCompleted ? "var(--color-jade)" : undefined }}>
              <CheckCircle size={14} style={{ color: allCompleted ? "var(--color-jade)" : undefined }} />
              <span>完成</span>
            </div>
          </div>
        </div>
      </div>

      {/* 项目上下文预览 */}
      {(() => {
        const ctx = buildProjectContext(projectData);
        return (
          <details style={{ marginTop: "var(--space-md)" }}>
            <summary style={{
              cursor: "pointer", fontSize: "var(--text-xs)", color: "var(--color-ink-3)",
              padding: "6px 0", userSelect: "none",
            }}>
              查看 Agent 可读取的项目上下文
            </summary>
            <div className="card" style={{
              padding: "var(--space-md)",
              fontSize: "var(--text-2xs)", lineHeight: 1.7,
              color: "var(--color-ink-2)", whiteSpace: "pre-wrap",
              fontFamily: "var(--font-mono, monospace)",
              maxHeight: 300, overflowY: "auto",
            }}>
              {ctx}
            </div>
          </details>
        );
      })()}

      {/* Agent 对话面板 */}
      <div className="card" style={{ marginTop: "var(--space-md)", padding: 0, overflow: "hidden" }}>
        <div className="card-header" style={{
          padding: "var(--space-md) var(--space-lg)", margin: 0, borderBottom: "1px solid var(--color-rule-light)",
        }}>
          <Brain size={15} color="var(--color-accent)" />
          <h3>Agent 实时反馈</h3>
          {agentWorking && (
            <span style={{ display: "flex", alignItems: "center", gap: 4, marginLeft: "auto", fontSize: "var(--text-2xs)", color: "var(--color-accent)" }}>
              <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
              思考中
            </span>
          )}
        </div>

        <div style={{
          padding: "var(--space-md) var(--space-lg)",
          maxHeight: 420, overflowY: "auto",
          display: "flex", flexDirection: "column", gap: 8,
          background: "var(--color-paper-cream)",
        }}>
          {messages.length === 0 && !agentWorking && (
            <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--color-ink-faint)", fontSize: "var(--text-sm)" }}>
              <MessageSquare size={32} strokeWidth={1} style={{ margin: "0 auto 12px", display: "block", opacity: 0.3 }} />
              <div>Agent 等待指令</div>
              <div style={{ fontSize: "var(--text-xs)", marginTop: 4 }}>点击「推进」或「自动推进」开始创作</div>
            </div>
          )}

          {messages.map(msg => (
            <div key={msg.id} style={{
              display: "flex",
              gap: 8,
              alignItems: "flex-start",
              animation: "inkSettle 0.3s ease-out",
            }}>
              {/* 消息图标 */}
              <div style={{
                flexShrink: 0, width: 22, height: 22, borderRadius: "50%",
                display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: 11, marginTop: 1,
                background:
                  msg.type === "think" ? "var(--color-indigo-wash)" :
                  msg.type === "action" ? "var(--color-accent-wash)" :
                  msg.type === "output" ? "var(--color-paper-warm)" :
                  msg.type === "complete" ? "var(--color-jade-wash)" :
                  "var(--color-ochre-wash)",
                color:
                  msg.type === "think" ? "var(--color-indigo)" :
                  msg.type === "action" ? "var(--color-accent)" :
                  msg.type === "output" ? "var(--color-ink-3)" :
                  msg.type === "complete" ? "var(--color-jade)" :
                  "var(--color-error)",
              }}>
                {msg.type === "think" ? "思" :
                 msg.type === "action" ? ">" :
                 msg.type === "output" ? "文" :
                 msg.type === "complete" ? "✓" : "!"}
              </div>

              {/* 消息内容 */}
              <div style={{ flex: 1, minWidth: 0 }}>
                {msg.stageName && msg.type !== "complete" && (
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2, letterSpacing: "0.5px" }}>
                    [{msg.stageName}]
                  </div>
                )}

                {msg.type === "output" ? (
                  <div style={{
                    fontSize: "var(--text-xs)", lineHeight: 1.7,
                    color: "var(--color-ink)",
                    background: "var(--color-paper)",
                    borderRadius: "var(--radius-sm)",
                    padding: "10px 14px",
                    border: "1px solid var(--color-rule-light)",
                    whiteSpace: "pre-wrap",
                    fontFamily: "var(--font-mono, monospace)",
                  }}>
                    {msg.streaming && msg.id.endsWith(String(messages.length - 1))
                      ? streamingContent || ""
                      : msg.content}
                    {msg.streaming && (
                      <span style={{ display: "inline-block", width: 6, height: 14, background: "var(--color-accent)", marginLeft: 2, animation: "blink 0.6s step-end infinite", verticalAlign: "text-bottom" }} />
                    )}
                  </div>
                ) : msg.type === "think" ? (
                  <div style={{
                    fontSize: "var(--text-xs)", lineHeight: 1.6,
                    color: "var(--color-indigo)",
                    fontStyle: "italic",
                    letterSpacing: "0.3px",
                  }}>
                    {msg.content}
                    {agentWorking && msg === messages[messages.length - 1] && msg.type === "think" && (
                      <span style={{ display: "inline-block", width: 4, height: 4, borderRadius: "50%", background: "var(--color-indigo)", marginLeft: 4, animation: "pulse 1s ease-in-out infinite" }} />
                    )}
                  </div>
                ) : msg.type === "action" ? (
                  <div style={{
                    fontSize: "var(--text-xs)", lineHeight: 1.6,
                    color: "var(--color-accent)", fontWeight: 500,
                  }}>
                    {msg.content}
                  </div>
                ) : msg.type === "complete" ? (
                  <div style={{
                    fontSize: "var(--text-xs)", lineHeight: 1.6,
                    color: "var(--color-jade)", fontWeight: 500,
                    display: "flex", alignItems: "center", gap: 4,
                  }}>
                    <CheckCircle size={12} /> {msg.content}
                  </div>
                ) : (
                  <div style={{ fontSize: "var(--text-xs)", lineHeight: 1.6, color: "var(--color-error)" }}>
                    {msg.content}
                  </div>
                )}
              </div>
            </div>
          ))}

          {/* 流式输出占位 */}
          {isStreaming && (
            <div style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
              <div style={{
                flexShrink: 0, width: 22, height: 22, borderRadius: "50%",
                display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: 11, background: "var(--color-paper-warm)", color: "var(--color-ink-3)", marginTop: 1,
              }}>文</div>
              <div style={{
                fontSize: "var(--text-xs)", lineHeight: 1.7, color: "var(--color-ink)",
                background: "var(--color-paper)", borderRadius: "var(--radius-sm)",
                padding: "10px 14px", border: "1px solid var(--color-rule-light)",
                whiteSpace: "pre-wrap", fontFamily: "var(--font-mono, monospace)", flex: 1,
              }}>
                {streamingContent}
                <span style={{ display: "inline-block", width: 6, height: 14, background: "var(--color-accent)", marginLeft: 2, animation: "blink 0.6s step-end infinite", verticalAlign: "text-bottom" }} />
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* 控制区 */}
      <div className="card" style={{ marginTop: "var(--space-md)", padding: "var(--space-lg)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <div style={{ fontWeight: 500, fontSize: "var(--text-sm)", color: "var(--color-ink)" }}>
            {allCompleted ? "所有阶段已完成" : currentStage ? `当前阶段: ${currentStage.display_name}` : "就绪"}
          </div>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 2 }}>
            {stageList.length} 个阶段 · 已完成 {stageList.filter(s => progress?.stages[s.name]?.status === "completed").length} 个
          </div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={handleAutoAdvance}
            disabled={!!(btnDisabled || allCompleted)}
            className="btn btn-accent"
            style={{ padding: "7px 16px" }}
          >
            {autoAdvance ? (
              <><Loader2 size={15} style={{ animation: "spin 1s linear infinite" }} /> 自动推进中...</>
            ) : (
              <><Play size={15} /> 自动推进</>
            )}
          </button>
          <button
            onClick={handleAdvance}
            disabled={btnDisabled}
            className={"btn btn-primary" + (btnDisabled ? " btn-disabled" : "")}
          >
            {agentWorking ? (
              <><Loader2 size={15} style={{ animation: "spin 1s linear infinite" }} /> 工作中...</>
            ) : allCompleted ? (
              <><CheckCircle size={15} /> 全部完成</>
            ) : (
              <><Play size={15} /> 推进至下一阶段</>
            )}
          </button>
        </div>
      </div>

      {/* 完成状态 */}
      {allCompleted && (
        <div style={{ marginTop: "var(--space-md)", padding: "var(--space-lg)", background: "var(--color-jade-wash)", borderRadius: "var(--radius-md)", border: "1px solid oklch(44% 0.105 153 / 15%)", textAlign: "center" }}>
          <CheckCircle size={24} style={{ color: "var(--color-jade)", marginBottom: 8 }} />
          <div style={{ fontWeight: 500, color: "var(--color-jade)" }}>工作流执行完毕</div>
          <div style={{ fontSize: "var(--text-sm)", color: "var(--color-ink-3)", marginTop: 4 }}>
            Agent 已完成所有创作阶段。如需重新执行，请点击「重置」。
          </div>
        </div>
      )}

      {/* 空章节提示 */}
      {projectData.volumes.reduce((s, v) => s + v.chapters.length, 0) === 0 && (
        <div style={{ marginTop: "var(--space-md)", padding: 12, background: "var(--color-ochre-wash)", borderRadius: "var(--radius-sm)", fontSize: "var(--text-xs)", color: "var(--color-ochre)" }}>
          <AlertCircle size={12} style={{ verticalAlign: "middle", marginRight: 4 }} />
          当前作品还没有章节。建议先去「大纲」创建卷和章节，再回到这里让 Agent 自动写作。
        </div>
      )}

      {/* 人工放行确认对话框 */}
      {confirmManual !== null && currentStage && (
        <div className="modal-overlay" onClick={() => setConfirmManual(null)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3 style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-lg)", fontWeight: 400, marginBottom: 12, letterSpacing: "1px" }}>
              确认执行：{currentStage.display_name}
            </h3>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--color-ink-2)", marginBottom: 16, lineHeight: 1.6 }}>
              <p style={{ marginBottom: 8 }}>此阶段需要人工确认后才能执行。Agent 将使用以下工作手册：</p>
              <div style={{
                background: "var(--color-paper-warm)", padding: 12, borderRadius: "var(--radius-sm)",
                fontSize: "var(--text-xs)", color: "var(--color-ink-3)", fontStyle: "italic",
                lineHeight: 1.5, maxHeight: 120, overflow: "auto",
              }}>
                {currentStage.prompt_template}
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-secondary" onClick={() => setConfirmManual(null)}>取消</button>
              <button className="btn btn-primary" onClick={handleManualConfirm}>
                <Play size={14} /> 确认执行
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
