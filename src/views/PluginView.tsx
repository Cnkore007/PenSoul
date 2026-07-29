import { useState, useEffect, useCallback } from "react";
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
import { listPlugins, installPlugin, removePlugin, togglePlugin } from "../ipc";

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
  const [plugins, setPlugins] = useState<PluginConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedPlugin, setExpandedPlugin] = useState<string | null>("standard-novel");
  const [yamlMode, setYamlMode] = useState(false);
  const [yamlContent, setYamlContent] = useState("");
  const [importing, setImporting] = useState(false);
  const [editingStage, setEditingStage] = useState<{ pluginId: string; stageIndex: number } | null>(null);

  // 从后端加载插件列表
  const loadPluginsFromBackend = useCallback(async () => {
    setLoading(true);
    try {
      const raw = await listPlugins();
      const mapped: PluginConfig[] = raw.map((p: any) => ({
        plugin_id: p.plugin_id,
        name: p.name,
        version: p.version,
        description: p.description,
        enabled: p.enabled,
        stages: (p.stages || []).map((s: any) => ({
          name: s.name,
          display_name: s.display_name,
          tool: s.tool,
          gate: s.gate,
          runner: s.runner,
          prompt_template: s.prompt_template || "",
          allowed_tools: s.allowed_tools || [],
          denied_tools: s.denied_tools || [],
          timeout_seconds: s.timeout_seconds || 300,
          max_retries: s.max_retries || 2,
        })),
      }));
      setPlugins(mapped);
    } catch (e) {
      console.error("加载插件列表失败:", e);
      setPlugins([]);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    loadPluginsFromBackend();
  }, [loadPluginsFromBackend]);

  function handleToggle(pluginId: string, enabled: boolean) {
    setPlugins(prev =>
      prev.map(p => (p.plugin_id === pluginId ? { ...p, enabled } : p))
    );
    togglePlugin(pluginId, enabled).catch(e =>
      console.error("切换插件状态失败:", e)
    );
  }

  function handleDelete(pluginId: string) {
    setPlugins(prev => prev.filter(p => p.plugin_id !== pluginId));
    if (expandedPlugin === pluginId) setExpandedPlugin(null);
    removePlugin(pluginId).catch(e =>
      console.error("删除插件失败:", e)
    );
  }

  function handleImport() {
    setImporting(true);
  }

  function handleImportSubmit() {
    if (yamlContent.trim()) {
      installPlugin(yamlContent.trim())
        .then(() => {
          loadPluginsFromBackend();
        })
        .catch(e => {
          alert("导入失败: " + (e?.message || String(e)));
        });
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
    const yaml = `plugin_id: ${newPlugin.plugin_id}
name: ${newPlugin.name}
version: ${newPlugin.version}
description: ${newPlugin.description}
stages: []`;
    installPlugin(yaml)
      .then(() => loadPluginsFromBackend())
      .catch(e => console.error("创建插件失败:", e));
  }

  if (loading) {
    return (
      <div className="plugin-view">
        <div className="pv-header">
          <div className="pv-header-left">
            <h2 className="pv-title">造化工坊</h2>
            <p className="pv-subtitle">声明式工作流 — 零代码可换的创作引擎</p>
          </div>
        </div>
        <div className="empty-state">
          <div className="empty-state-text">加载中...</div>
        </div>
      </div>
    );
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
