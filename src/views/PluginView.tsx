import { useState, useEffect } from "react";
import {
  Puzzle,
  Trash2,
  ChevronDown,
  ChevronRight,
  Upload,
  Download,
  Settings,
  Play,
  Pause,
  ArrowRight,
  CheckCircle,
  AlertCircle,
  Clock,
  Zap,
  Plus,
} from "lucide-react";
import type { PluginConfig } from "../types";
import { loadPlugins, savePlugins } from "../store";

const builtinPlugins: PluginConfig[] = [
  {
    plugin_id: "standard-novel",
    name: "标准小说工作流",
    version: "1.0.0",
    description: "默认的长篇小说创作流程，包含大纲生成、章节写作、一致性审查、文风校准等阶段。",
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
    plugin_id: "webnovel-optimized",
    name: "网文快写工作流",
    version: "1.0.0",
    description: "面向网络文学的快速创作流程，精简审查环节，加速产出。",
    enabled: false,
    stages: [
      { name: "quick_outline", display_name: "快速大纲", tool: "llm_generate", gate: "auto", runner: "local", prompt_template: "快速生成3-5章的简要大纲...", allowed_tools: ["read_settings", "write_outline"], denied_tools: [], timeout_seconds: 120, max_retries: 2 },
      { name: "batch_write", display_name: "批量写作", tool: "llm_generate", gate: "auto", runner: "local", prompt_template: "根据大纲连续生成多章正文...", allowed_tools: ["read_outline", "generate_prose"], denied_tools: [], timeout_seconds: 600, max_retries: 3 },
      { name: "quick_check", display_name: "快速检查", tool: "llm_analyze", gate: "auto", runner: "local", prompt_template: "快速检查关键一致性问题...", allowed_tools: ["run_consistency_check"], denied_tools: [], timeout_seconds: 60, max_retries: 1 },
    ],
  },
];

const gateIcons: Record<string, React.ReactNode> = {
  auto: <Zap size={13} />,
  manual: <AlertCircle size={13} />,
  conditional: <Clock size={13} />,
};

const gateLabels: Record<string, string> = {
  auto: "自动放行",
  manual: "人工放行",
  conditional: "条件放行",
};

const runnerLabels: Record<string, string> = {
  local: "本地执行",
  delegated: "委托执行",
};

export function PluginView() {
  const [plugins, setPlugins] = useState<PluginConfig[]>(() => {
    const saved = loadPlugins();
    // 区分首次加载（植入默认）和用户主动删除（即使为空也不回退）
    const raw = localStorage.getItem("pensoul_plugins");
    if (raw === null) {
      savePlugins(builtinPlugins);
      return builtinPlugins;
    }
    return saved;
  });
  const [expandedPlugin, setExpandedPlugin] = useState<string | null>("standard-novel");

  useEffect(() => {
    savePlugins(plugins);
  }, [plugins]);
  const [yamlMode, setYamlMode] = useState(false);
  const [yamlContent, setYamlContent] = useState("");
  const [importing, setImporting] = useState(false);
  const [editingStage, setEditingStage] = useState<{ pluginId: string; stageIndex: number } | null>(null);

  function handleToggle(pluginId: string, enabled: boolean) {
    setPlugins(prev =>
      prev.map(p => (p.plugin_id === pluginId ? { ...p, enabled } : p))
    );
  }

  function handleDelete(pluginId: string) {
    setPlugins(prev => prev.filter(p => p.plugin_id !== pluginId));
    if (expandedPlugin === pluginId) setExpandedPlugin(null);
  }

  function handleImport() {
    setImporting(true);
  }

  function handleImportSubmit() {
    if (yamlContent.trim()) {
      alert("YAML 导入功能需要后端支持解析");
    }
    setImporting(false);
    setYamlContent("");
  }

  function handleExport(pluginId: string) {
    const plugin = plugins.find(p => p.plugin_id === pluginId);
    if (plugin) {
      const yaml = `plugin_id: ${plugin.plugin_id}
name: ${plugin.name}
version: ${plugin.version}
description: ${plugin.description}
stages:
${plugin.stages
  .map(
    s => `  - name: ${s.name}
    display_name: ${s.display_name}
    tool: ${s.tool}
    gate: ${s.gate}
    runner: ${s.runner}`
  )
  .join("\n")}`;
      alert(`导出 YAML:\n\n${yaml}`);
    }
  }

  function handleCreateNew() {
    const newPlugin: PluginConfig = {
      plugin_id: `custom-${Date.now()}`,
      name: "自定义工作流",
      version: "1.0.0",
      description: "新创建的工作流",
      enabled: false,
      stages: [],
    };
    setPlugins(prev => [...prev, newPlugin]);
    setExpandedPlugin(newPlugin.plugin_id);
  }

  return (
    <div className="plugin-view">
      {/* Header */}
      <div className="pv-header">
        <div className="pv-header-left">
          <h2 className="pv-title">造化工坊</h2>
          <p className="pv-subtitle">声明式工作流 — 零代码可换的创作引擎</p>
        </div>
        <div className="pv-header-actions">
          <button className="pv-btn pv-btn-ghost" onClick={handleImport}>
            <Upload size={15} /> 导入
          </button>
          <button className="pv-btn pv-btn-ghost" onClick={() => setYamlMode(!yamlMode)}>
            <Settings size={15} /> {yamlMode ? "可视化" : "YAML"}
          </button>
          <button className="pv-btn pv-btn-primary" onClick={handleCreateNew}>
            <Plus size={15} /> 新建工作流
          </button>
        </div>
      </div>

      {/* Import panel */}
      {importing && (
        <div className="pv-import-panel">
          <h3>导入 YAML 配置</h3>
          <textarea
            className="pv-yaml-input"
            value={yamlContent}
            onChange={e => setYamlContent(e.target.value)}
            placeholder="粘贴 YAML 配置内容..."
          />
          <div className="pv-import-actions">
            <button className="pv-btn pv-btn-primary" onClick={handleImportSubmit}>确认导入</button>
            <button className="pv-btn pv-btn-ghost" onClick={() => setImporting(false)}>取消</button>
          </div>
        </div>
      )}

      {/* Plugin list */}
      {plugins.map(plugin => (
        <div key={plugin.plugin_id} className="pv-plugin">
          <div className="pv-plugin-header">
            <div className="pv-plugin-header-left">
              <button
                className={`pv-toggle ${plugin.enabled ? "pv-toggle-on" : ""}`}
                onClick={() => handleToggle(plugin.plugin_id, !plugin.enabled)}
              >
                {plugin.enabled ? <Play size={12} /> : <Pause size={12} />}
              </button>
              <div>
                <h3 className="pv-plugin-name">{plugin.name}</h3>
                <span className="pv-plugin-version">v{plugin.version}</span>
              </div>
            </div>
            <div className="pv-plugin-header-right">
              <button
                className="pv-icon-btn"
                onClick={() =>
                  setExpandedPlugin(expandedPlugin === plugin.plugin_id ? null : plugin.plugin_id)
                }
              >
                {expandedPlugin === plugin.plugin_id ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              </button>
              <button
                className="pv-icon-btn pv-icon-btn-ghost"
                onClick={() => handleExport(plugin.plugin_id)}
              >
                <Download size={14} />
              </button>
              <button
                className="pv-icon-btn pv-icon-btn-danger"
                onClick={() => handleDelete(plugin.plugin_id)}
              >
                <Trash2 size={14} />
              </button>
            </div>
          </div>
          <p className="pv-plugin-desc">{plugin.description}</p>

          {/* Expanded: Visual Pipeline */}
          {expandedPlugin === plugin.plugin_id && (
            <div className="pv-pipeline">
              {yamlMode ? (
                <textarea
                  className="pv-yaml-editor"
                  readOnly
                  value={plugin.stages.map(s =>
                    `- name: ${s.name}\n  display_name: ${s.display_name}\n  tool: ${s.tool}\n  gate: ${s.gate}\n  runner: ${s.runner}`
                  ).join("\n\n")}
                />
              ) : (
                <div className="pv-flow">
                  {/* Start node */}
                  <div className="pv-node pv-node-start">
                    <div className="pv-node-dot pv-dot-green" />
                    <span>开始</span>
                  </div>

                  {plugin.stages.map((stage, index) => (
                    <div key={stage.name} className="pv-stage-wrapper">
                      {/* Connector arrow */}
                      <div className="pv-connector">
                        <div className="pv-connector-line" />
                        <ArrowRight size={12} className="pv-connector-arrow" />
                      </div>

                      {/* Stage card */}
                      <div
                        className={`pv-node pv-node-stage ${editingStage?.pluginId === plugin.plugin_id && editingStage?.stageIndex === index ? "pv-node-editing" : ""}`}
                        onClick={() =>
                          setEditingStage(
                            editingStage?.pluginId === plugin.plugin_id && editingStage?.stageIndex === index
                              ? null
                              : { pluginId: plugin.plugin_id, stageIndex: index }
                          )
                        }
                      >
                        <div className="pv-stage-header">
                          <span className="pv-stage-number">{index + 1}</span>
                          <span className="pv-stage-name">{stage.display_name}</span>
                        </div>
                        <div className="pv-stage-tags">
                          <span className={`pv-tag pv-tag-gate pv-gate-${stage.gate}`}>
                            {gateIcons[stage.gate]} {gateLabels[stage.gate]}
                          </span>
                          <span className="pv-tag pv-tag-runner">{runnerLabels[stage.runner]}</span>
                          <span className="pv-tag pv-tag-tool">{stage.tool}</span>
                        </div>
                      </div>
                    </div>
                  ))}

                  {/* End node */}
                  <div className="pv-connector">
                    <div className="pv-connector-line" />
                    <ArrowRight size={12} className="pv-connector-arrow" />
                  </div>
                  <div className="pv-node pv-node-end">
                    <CheckCircle size={14} />
                    <span>完成</span>
                  </div>
                </div>
              )}

              {/* Stage detail panel */}
              {editingStage && editingStage.pluginId === plugin.plugin_id && (
                <div className="pv-stage-detail">
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">阶段名称</span>
                    <span className="pv-detail-value">{plugin.stages[editingStage.stageIndex].display_name}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">工具</span>
                    <span className="pv-detail-value">{plugin.stages[editingStage.stageIndex].tool}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">门控</span>
                    <span className="pv-detail-value">{gateLabels[plugin.stages[editingStage.stageIndex].gate]}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">执行者</span>
                    <span className="pv-detail-value">{runnerLabels[plugin.stages[editingStage.stageIndex].runner]}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">超时</span>
                    <span className="pv-detail-value">{plugin.stages[editingStage.stageIndex].timeout_seconds}s</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">最大重试</span>
                    <span className="pv-detail-value">{plugin.stages[editingStage.stageIndex].max_retries}</span>
                  </div>
                  <div className="pv-detail-row">
                    <span className="pv-detail-label">允许工具</span>
                    <span className="pv-detail-value pv-detail-tools">
                      {plugin.stages[editingStage.stageIndex].allowed_tools.map(t => (
                        <span key={t} className="pv-tag pv-tag-tool-small">{t}</span>
                      ))}
                    </span>
                  </div>
                  {plugin.stages[editingStage.stageIndex].denied_tools.length > 0 && (
                    <div className="pv-detail-row">
                      <span className="pv-detail-label">禁止工具</span>
                      <span className="pv-detail-value pv-detail-tools pv-detail-denied">
                        {plugin.stages[editingStage.stageIndex].denied_tools.map(t => (
                          <span key={t} className="pv-tag pv-tag-denied">{t}</span>
                        ))}
                      </span>
                    </div>
                  )}
                  <div className="pv-detail-row pv-detail-row-full">
                    <span className="pv-detail-label">工作手册</span>
                    <p className="pv-detail-prompt">{plugin.stages[editingStage.stageIndex].prompt_template}</p>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      ))}

      {plugins.length === 0 && (
        <div className="pv-empty">
          <Puzzle size={48} strokeWidth={1} />
          <div className="pv-empty-title">暂无工作流</div>
          <div className="pv-empty-sub">点击「新建工作流」创建你的第一个创作引擎</div>
        </div>
      )}
    </div>
  );
}
