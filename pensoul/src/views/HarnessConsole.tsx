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
const stageScripts: Record<string, Array<{ type: "think" | "action" | "output"; content: string }>> = {
  outline_generation: [
    { type: "think", content: "正在读取项目设定和已有章节结构..." },
    { type: "think", content: "分析故事核心冲突和主线脉络..." },
    { type: "think", content: "评估各章节之间的起承转合关系..." },
    { type: "action", content: "生成大纲框架..." },
    { type: "output", content: "第一卷·江湖再起\n  第一章 暮色剑鸣：开篇以一场雨夜对决引入主角沈舟，埋下师门血案的伏笔。\n  第二章 旧信新痕：沈舟收到一封匿名信，指向十五年前灭门案的真相。\n  第三章 画舫夜谈：关键配角登场，各方势力在秦淮画舫上暗中角力。\n\n第二卷·迷雾渐深\n  第四章 古卷残章：沈舟在一处废墟中发现半部失传的武学秘典。\n  第五章 双面谍影：同伴中出现叛徒，主角陷入信任危机。\n  第六章 深谷围杀：正邪势力在幽深峡谷中展开遭遇战。」" },
    { type: "think", content: "大纲生成完毕，检查各章节之间的逻辑连贯性..." },
  ],
  chapter_writing: [
    { type: "think", content: "读取大纲中本章节的核心设定和情节目标..." },
    { type: "think", content: "回顾前文的人物状态和世界观一致性..." },
    { type: "think", content: "设计本章节的叙事节奏和情感起伏..." },
    { type: "think", content: "构思开场画面和关键对话..." },
    { type: "action", content: "撰写章节正文..." },
    { type: "output", content: "第一章 暮色剑鸣\n\n雨下了整整三日。\n\n沈舟站在听雨楼的檐下，望着青石板上溅起的水花，手中的剑已出鞘三寸又归回原位。对面的黑衣人同样没动，两人隔着七步的距离，像两尊被雨水浸透的石像。\n\n「这场架非打不可？」沈舟终于开口，声音被雨声压得有些模糊。\n\n黑衣人没有答话，只是微微侧身，露出了腰间的令牌——青铜质地，虎纹缠绕，正是十五年前灭门惨案现场遗留的那块。沈舟的瞳孔骤然收缩。\n\n剑光破开雨幕的瞬间，整条街的灯笼同时熄灭了。\n\n这一战持续了不到一盏茶的工夫。当沈舟的剑尖抵住对方咽喉时，黑衣人却笑了：」你果然还是和当年一样，出手留三分情面。」说完便纵身跃入雨中，消失不见。\n\n沈舟收剑回鞘，低头看着剑刃上残留的水珠。那封信还揣在怀中，已被体温捂得温热。信上只有八个字：\n\n「欲知真相，来金陵见。」\n\n没有落款，没有日期，但那笔迹沈舟认得——是师父的。」" },
    { type: "think", content: "完成初稿后检查文风一致性..." },
  ],
  consistency_check: [
    { type: "think", content: "逐章扫描人物状态和设定的一致性..." },
    { type: "think", content: "检查时间线是否存在矛盾..." },
    { type: "think", content: "对比角色性格与对话行为是否匹配..." },
    { type: "action", content: "生成一致性审查报告..." },
    { type: "output", content: "一致性检查完成\n\n✓ 时间线自洽：所有章节的时间顺序正确，无前后矛盾。\n✓ 人物状态一致：沈舟的性格发展连贯，从隐忍到坚定的转变合理。\n⚠ 发现 1 处潜在冲突：第三章中提及「秦淮画舫」位于金陵，但第四章的场景设置在洛阳，两地相距约 600 公里，需确认时间跨度是否合理。\n⚠ 建议：第二章黑衣人使用的令牌在第五章未有呼应，如为伏笔请确保后文提及。\n✓ 世界观规则：武功设定、江湖势力分布保持稳定。」" },
  ],
  style_review: [
    { type: "think", content: "提取全文文风特征进行量化分析..." },
    { type: "think", content: "对比预设风格指纹模板..." },
    { type: "think", content: "检测 AI 写作痕迹和模式化表达..." },
    { type: "action", content: "生成文风诊断报告..." },
    { type: "output", content: "文风分析报告\n\n✎ 平均句长: 18.3 字/句（目标范围 15-22 → ✓）\n✎ 词汇丰富度: 73.2%（目标 >65% → ✓）\n✎ 对话占比: 31.5%（目标 25-40% → ✓）\n✎ 叙事节奏分: 0.72（偏紧凑）\n✎ AI 模式指数: 8.1%（阈值 <20% → ✓ 无明显 AI 痕迹）\n\n风格评价：文白相间，用词考究，符合古风武侠设定。建议在对话中适当增加方言特征词以增强人物辨识度。」" },
  ],
  state_injection: [
    { type: "think", content: "解析本章新增的角色状态变化..." },
    { type: "think", content: "提取关键剧情节点更新世界观演化记录..." },
    { type: "think", content: "更新角色关系图谱..." },
    { type: "action", content: "回灌状态到记忆系统..." },
    { type: "output", content: "状态回灌完成\n\n✓ 沈舟：心境从【隐忍】→【坚定】，新增关联道具「匿名信」\n✓ 世界观：触发事件「金陵来信」，新增地点「听雨楼」已收录\n✓ 势力关系：揭示第三方势力的存在，标记为【未知·青铜令】\n✓ 未完成线索：黑衣人身份待确认，已加入长期记忆队列" },
  ],
  quick_outline: [
    { type: "think", content: "快速扫描核心设定提取关键要素..." },
    { type: "think", content: "生成精简大纲框架..." },
    { type: "output", content: "【快速大纲】\n第一章 开局·风波起（2000字）\n  主角在平凡生活中遭遇变故，被迫踏上征途。\n第二章 探索·遇新知（2500字）\n  进入新环境，结识同伴，发现世界真相的一角。\n第三章 冲突·初交锋（2200字）\n  与反派势力首次正面碰撞，初尝失败。」" },
  ],
  batch_write: [
    { type: "think", content: "准备批量写作环境..." },
    { type: "think", content: "按大纲顺序连续生成章节..." },
    { type: "action", content: "批量生成中..." },
    { type: "output", content: "【批量写作进度】\n\n■ 第一章 完成 (2200字)\n■ 第二章 完成 (2450字)\n□ 第三章 进行中...\n\n当前输出：\n夜幕降临的时候，队伍终于在密林的边缘找到了一处废弃的驿站。篝火升起来，跳动的火光把每个人的影子拉得很长。林远靠在柱子上，手里捏着那张从黑衣人身上搜来的地图，纸页已经被汗水浸得有些模糊。\n\n「我们离那里还有多远？」苏晚的声音从背后传来。\n\n「按照地图上的标记，大概还要走三天。」林远没有回头，目光仍停留在地图上那个用红圈标出的位置——那里标注着一个他从未听过的地名。\n\n「三天...」苏晚在他身边坐下，你觉得那里真的会有答案吗？」\n\n林远沉默了一会儿，最终只是说了句：」总得去看看。」\n\n火堆里传来木柴爆裂的声响，像是替他说出了那些未竟之言。」" },
    { type: "think", content: "批量写作进度：66%，预计剩余时间 45 秒..." },
  ],
  quick_check: [
    { type: "think", content: "执行快速一致性扫描..." },
    { type: "output", content: "快速检查通过 ✓\n\n未发现严重的一致性问题。2 处轻微的时间表述差异已标记，可后续手动调整。」" },
  ],
};

// 对每个阶段补充默认脚本
const defaultStageScript: Array<{ type: "think" | "action" | "output"; content: string }> = [
  { type: "think", content: "正在分析任务目标和上下文..." },
  { type: "think", content: "评估可用的工具和方法..." },
  { type: "action", content: "执行阶段任务..." },
  { type: "output", content: "任务执行完成。输出结果已保存至项目数据。」" },
];

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
    const script = stageScripts[stage.name] || defaultStageScript;

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
