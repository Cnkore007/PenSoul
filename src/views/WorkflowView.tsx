import { useState } from "react";
import { Play, Pause, ArrowRight, CheckCircle, Zap, AlertCircle, Clock, Save } from "lucide-react";
import type { ProjectData, PluginConfig, ViewType } from "../types";

interface WorkflowViewProps {
  projectData: ProjectData;
  persistProjectData: (updater: (prev: ProjectData) => ProjectData) => void;
  onNavigate?: (view: ViewType) => void;
}

// 内置工作流模板
export const builtinWorkflows: PluginConfig[] = [
  {
    plugin_id: "standard-novel",
    name: "标准小说工作流",
    version: "1.0.0",
    description: "完整的长篇小说创作流程：大纲生成 → 章节写作 → 一致性审查 → 文风校准 → 状态回灌",
    enabled: true,
    stages: [
      { name: "outline_generation", display_name: "大纲生成", tool: "llm_generate", gate: "manual", runner: "local", prompt_template: "根据用户提供的核心设定，生成完整的小说大纲...", allowed_tools: ["read_settings", "write_outline"], denied_tools: ["generate_prose"], timeout_seconds: 600, max_retries: 3 },
      { name: "chapter_writing", display_name: "章节写作", tool: "llm_generate", gate: "auto", runner: "local", prompt_template: "根据大纲和前文内容，撰写本章正文...", allowed_tools: ["read_chapter", "read_outline", "read_character_state", "generate_prose"], denied_tools: ["modify_settings", "modify_outline"], timeout_seconds: 300, max_retries: 3 },
      { name: "consistency_check", display_name: "一致性审查", tool: "llm_analyze", gate: "conditional", runner: "delegated", prompt_template: "对本章进行一致性检查...", allowed_tools: ["read_chapter", "read_character_state", "run_consistency_check"], denied_tools: ["generate_prose", "modify_settings"], timeout_seconds: 300, max_retries: 2 },
      { name: "style_review", display_name: "文风审查", tool: "llm_analyze", gate: "conditional", runner: "delegated", prompt_template: "检查本章文风是否符合预设风格指纹...", allowed_tools: ["read_chapter", "analyze_style"], denied_tools: ["generate_prose"], timeout_seconds: 300, max_retries: 2 },
      { name: "state_injection", display_name: "状态回灌", tool: "system", gate: "auto", runner: "local", prompt_template: "将本章关键信息回灌到角色状态和记忆系统...", allowed_tools: ["update_character_state", "update_memory"], denied_tools: ["generate_prose"], timeout_seconds: 60, max_retries: 1 },
    ],
  },
  {
    plugin_id: "quick-novel",
    name: "快速创作工作流",
    version: "1.0.0",
    description: "精简流程，适合快速产出：大纲 → 写作 → 快速检查",
    enabled: false,
    stages: [
      { name: "quick_outline", display_name: "快速大纲", tool: "llm_generate", gate: "auto", runner: "local", prompt_template: "快速生成3-5章的简要大纲...", allowed_tools: ["read_settings", "write_outline"], denied_tools: [], timeout_seconds: 120, max_retries: 2 },
      { name: "batch_write", display_name: "批量写作", tool: "llm_generate", gate: "auto", runner: "local", prompt_template: "根据大纲连续生成多章正文...", allowed_tools: ["read_outline", "generate_prose"], denied_tools: [], timeout_seconds: 600, max_retries: 3 },
      { name: "quick_check", display_name: "快速检查", tool: "llm_analyze", gate: "auto", runner: "local", prompt_template: "快速检查关键一致性问题...", allowed_tools: ["run_consistency_check"], denied_tools: [], timeout_seconds: 60, max_retries: 1 },
    ],
  },
];

const gateIcons: Record<string, React.ReactNode> = { auto: <Zap size={13} />, manual: <AlertCircle size={13} />, conditional: <Clock size={13} /> };
const gateLabels: Record<string, string> = { auto: "自动", manual: "人工", conditional: "条件" };

export function WorkflowView({ projectData, persistProjectData, onNavigate }: WorkflowViewProps) {
  const [expandedStage, setExpandedStage] = useState<number | null>(null);
  const [saved, setSaved] = useState(false);

  // 当前项目关联的工作流
  const activeId = projectData.workflow_id;
  const activeWorkflow = builtinWorkflows.find(w => w.plugin_id === activeId);

  function selectWorkflow(pluginId: string) {
    persistProjectData(prev => ({ ...prev, workflow_id: pluginId }));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }

  function goToHarness() {
    onNavigate?.("harness");
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>工作流配置</h2>
        {saved && <span className="tag tag-success"><Save size={12} /> 已保存</span>}
      </div>

      <div className="empty-state" style={{ padding: "16px 0 24px", textAlign: "left" }}>
        <div className="empty-state-sub" style={{ fontSize: "var(--text-sm)" }}>
          选择一个工作流模板，Agent 将按照工作流中的阶段自动推进创作。
          每个阶段可设置门控（自动/人工/条件），决定是否需要人工确认后才进入下一阶段。
        </div>
      </div>

      {/* 已选工作流 > 快速入口 */}
      {activeWorkflow && (
        <div style={{
          display: "flex", alignItems: "center", gap: "var(--space-md)",
          padding: "var(--space-md) var(--space-lg)",
          background: "var(--color-jade-wash)",
          borderRadius: "var(--radius-md)",
          marginBottom: "var(--space-lg)",
          border: "1px solid oklch(44% 0.105 153 / 15%)",
        }}>
          <CheckCircle size={20} style={{ color: "var(--color-jade)", flexShrink: 0 }} />
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 500, fontSize: "var(--text-sm)", color: "var(--color-jade)" }}>
              已选择：{activeWorkflow.name}
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)", marginTop: 2 }}>
              共 {activeWorkflow.stages.length} 个阶段
            </div>
          </div>
          <button className="btn btn-primary" onClick={goToHarness}>
            <Play size={15} /> 前往造化工坊
          </button>
        </div>
      )}

      {builtinWorkflows.map(wf => {
        const isActive = activeId === wf.plugin_id;
        return (
          <div key={wf.plugin_id} className="card" style={{ marginBottom: "var(--space-md)", border: isActive ? "1px solid var(--color-jade)" : undefined }}>
            <div className="card-header" style={{ justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
                <button
                  className={`btn ${isActive ? "btn-success" : "btn-secondary"}`}
                  style={{ padding: "4px 10px", fontSize: "12px" }}
                  onClick={() => selectWorkflow(isActive ? "" : wf.plugin_id)}
                >
                  {isActive ? <Pause size={12} /> : <Play size={12} />}
                </button>
                <div>
                  <h3 className="pv-plugin-name">{wf.name}</h3>
                  <span className="pv-plugin-version">v{wf.version}</span>
                  {isActive && <span className="tag tag-success" style={{ marginLeft: 8 }}>当前使用</span>}
                </div>
              </div>
            </div>
            <p className="pv-plugin-desc">{wf.description}</p>

            {/* 可视化流程 */}
            <div className="pv-pipeline">
              <div className="pv-flow">
                <div className="pv-node pv-node-start"><div className="pv-node-dot pv-dot-green" /><span>开始</span></div>
                {wf.stages.map((stage, idx) => (
                  <div key={stage.name} style={{ display: "flex", alignItems: "center" }}>
                    <div className="pv-connector"><div className="pv-connector-line" /><ArrowRight size={12} className="pv-connector-arrow" /></div>
                    <div
                      className={`pv-node pv-node-stage ${expandedStage === idx ? "pv-node-editing" : ""}`}
                      onClick={() => setExpandedStage(expandedStage === idx ? null : idx)}
                    >
                      <div className="pv-stage-header">
                        <span className="pv-stage-number">{idx + 1}</span>
                        <span className="pv-stage-name">{stage.display_name}</span>
                      </div>
                      <div className="pv-stage-tags">
                        <span className={`pv-tag pv-tag-gate pv-gate-${stage.gate}`}>{gateIcons[stage.gate]} {gateLabels[stage.gate]}</span>
                        <span className="pv-tag pv-tag-tool">{stage.tool}</span>
                      </div>
                    </div>
                  </div>
                ))}
                <div className="pv-connector"><div className="pv-connector-line" /><ArrowRight size={12} className="pv-connector-arrow" /></div>
                <div className="pv-node pv-node-end"><CheckCircle size={14} /><span>完成</span></div>
              </div>

              {/* 阶段详情 */}
              {expandedStage !== null && expandedStage < wf.stages.length && (
                <div className="pv-stage-detail">
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">阶段</span>
                    <span className="pv-detail-value">{wf.stages[expandedStage].display_name}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">工具</span>
                    <span className="pv-detail-value">{wf.stages[expandedStage].tool}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">门控</span>
                    <span className="pv-detail-value">{gateLabels[wf.stages[expandedStage].gate]}放行</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">超时</span>
                    <span className="pv-detail-value">{wf.stages[expandedStage].timeout_seconds}s</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">允许工具</span>
                    <span className="pv-detail-value pv-detail-tools">
                      {wf.stages[expandedStage].allowed_tools.map(t => <span key={t} className="pv-tag pv-tag-tool-small">{t}</span>)}
                    </span>
                  </div>
                  {wf.stages[expandedStage].denied_tools.length > 0 && (
                    <div className="pv-detail-row">
                      <span className="pv-detail-label">禁止工具</span>
                      <span className="pv-detail-value pv-detail-tools pv-detail-denied">
                        {wf.stages[expandedStage].denied_tools.map(t => <span key={t} className="pv-tag pv-tag-denied">{t}</span>)}
                      </span>
                    </div>
                  )}
                  <div className="pv-detail-row pv-detail-row-full">
                    <span className="pv-detail-label">工作手册</span>
                    <p className="pv-detail-prompt">{wf.stages[expandedStage].prompt_template}</p>
                  </div>
                </div>
              )}
            </div>
          </div>
        );
      })}

      {!activeWorkflow && (
        <div style={{ marginTop: 24, padding: 16, background: "var(--color-ochre-wash)", borderRadius: "var(--radius-sm)", color: "var(--color-ochre)", fontSize: "var(--text-sm)" }}>
          <AlertCircle size={14} style={{ verticalAlign: "middle", marginRight: 6 }} />
          尚未选择工作流。请在上方选择一个模板启用，Agent 将按工作流阶段自动推进创作。
        </div>
      )}
    </div>
  );
}
