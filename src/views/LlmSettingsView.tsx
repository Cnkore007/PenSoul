import { useState, useEffect, useCallback } from "react";
import {
  CheckCircle, XCircle, TestTube, Eye, EyeOff, Zap,
  Plus, Trash2, RefreshCw, Globe, Key, Loader2, Wifi, WifiOff,
  ChevronDown, ChevronRight, Star,
} from "lucide-react";
import type { LlmProvider, LlmModel } from "../types";
import { listProviders, listModels, saveProviders, saveModels, saveApiKey, loadApiKeys, httpRequest, setDefaultModel } from "../ipc";

interface ProviderForm {
  provider_id: string;
  name: string;
  display_name: string;
  api_base: string;
  requires_api_key: boolean;
}

export default function LlmSettingsView() {
  const [providers, setProviders] = useState<LlmProvider[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({});
  const [showApiKey, setShowApiKey] = useState<Record<string, boolean>>({});
  const [activeTab, setActiveTab] = useState<'providers' | 'models'>('providers');
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [testingConn, setTestingConn] = useState<Record<string, 'testing' | 'success' | 'error'>>({});
  const [fetchingModels, setFetchingModels] = useState<Record<string, boolean>>({});
  const [editingBase, setEditingBase] = useState<Record<string, string>>({});
  const [editingName, setEditingName] = useState<Record<string, string>>({});
  const [expandedProviders, setExpandedProviders] = useState<Record<string, boolean>>({});
  const [testingModel, setTestingModel] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, 'success' | 'error'>>({});
  const [loading, setLoading] = useState(true);

  // 新增供应商表单
  const [showAddForm, setShowAddForm] = useState(false);
  const [newProvider, setNewProvider] = useState<ProviderForm>({
    provider_id: "",
    name: "",
    display_name: "",
    api_base: "",
    requires_api_key: true,
  });

  // 从后端加载数据
  const loadFromBackend = useCallback(async () => {
    setLoading(true);
    try {
      const [savedProviders, savedModels, savedKeys] = await Promise.all([
        listProviders(),
        listModels(),
        loadApiKeys(),
      ]);
      setApiKeys(savedKeys || {});
      setProviders(savedProviders);
      setModels(savedModels);

      // 初始化编辑态
      const bases: Record<string, string> = {};
      const names: Record<string, string> = {};
      savedProviders.forEach((p: LlmProvider) => {
        bases[p.provider_id] = p.api_base;
        names[p.provider_id] = p.display_name;
      });
      setEditingBase(bases);
      setEditingName(names);
    } catch (e) {
      console.error("加载 LLM 配置失败:", e);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    loadFromBackend();
  }, [loadFromBackend]);

  function flashSave(msg?: string) {
    setSaveMsg(msg || "已保存");
    setTimeout(() => setSaveMsg(null), 2000);
  }

  // 持久化供应商（本地状态 + 后端磁盘）
  function persistProviders(updated: LlmProvider[]) {
    setProviders(updated);
    // 确保编辑态同步
    const bases = { ...editingBase };
    const names = { ...editingName };
    updated.forEach(p => {
      if (!(p.provider_id in bases)) bases[p.provider_id] = p.api_base;
      if (!(p.provider_id in names)) names[p.provider_id] = p.display_name;
    });
    setEditingBase(bases);
    setEditingName(names);
    // 保存到后端磁盘
    saveProviders(updated).catch(e => console.error("保存供应商失败:", e));
    flashSave("供应商已保存");
  }

  // 保存 API 地址
  function saveApiBase(providerId: string) {
    const newBase = editingBase[providerId];
    if (!newBase) return;
    const updated = providers.map(p =>
      p.provider_id === providerId ? { ...p, api_base: newBase } : p
    );
    persistProviders(updated);
  }

  // 保存显示名称
  function saveDisplayName(providerId: string) {
    const newName = editingName[providerId];
    if (!newName) return;
    const updated = providers.map(p =>
      p.provider_id === providerId ? { ...p, display_name: newName } : p
    );
    persistProviders(updated);
  }

  // 保存 API Key — 通过 IPC 持久化到后端
  async function handleSaveApiKey(providerId: string, key: string) {
    setApiKeys(prev => ({ ...prev, [providerId]: key }));
    if (key) {
      try {
        await saveApiKey(providerId, key);
      } catch (e) {
        console.error("保存 API Key 失败:", e);
      }
    }
  }

  // 测试连接
  async function handleTestConnection(providerId: string) {
    const provider = providers.find(p => p.provider_id === providerId);
    if (!provider) return;
    setTestingConn(prev => ({ ...prev, [providerId]: 'testing' }));

    try {
      const headers: Record<string, string> = {};
      const key = apiKeys[providerId];
      if (key) headers["Authorization"] = `Bearer ${key}`;

      const resp = await httpRequest(provider.api_base.replace(/\/+$/, "") + "/models", "GET", headers);

      if (resp.ok) {
        setTestingConn(prev => ({ ...prev, [providerId]: 'success' }));
        setTimeout(() => setTestingConn(prev => {
          const next = { ...prev };
          delete next[providerId];
          return next;
        }), 3000);
      } else {
        setTestingConn(prev => ({ ...prev, [providerId]: 'error' }));
        flashSave(`连接失败: HTTP ${resp.status} ${resp.statusText} — ${resp.body.slice(0, 120)}`);
        setTimeout(() => setTestingConn(prev => {
          const next = { ...prev };
          delete next[providerId];
          return next;
        }), 3000);
      }
    } catch (e: any) {
      setTestingConn(prev => ({ ...prev, [providerId]: 'error' }));
      flashSave(`连接异常: ${e?.message || '无法到达服务端'}`);
      setTimeout(() => setTestingConn(prev => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      }), 3000);
    }
  }

  // 获取模型列表
  async function handleFetchModels(providerId: string) {
    const provider = providers.find(p => p.provider_id === providerId);
    if (!provider) return;
    setFetchingModels(prev => ({ ...prev, [providerId]: true }));

    try {
      const headers: Record<string, string> = {};
      const key = apiKeys[providerId];
      if (key) headers["Authorization"] = `Bearer ${key}`;

      const resp = await httpRequest(provider.api_base.replace(/\/+$/, "") + "/models", "GET", headers);

      if (resp.ok) {
        const data = JSON.parse(resp.body);
        const remoteModels: LlmModel[] = (data.data || []).map((m: any) => ({
          model_id: m.id || m.model_id || `${providerId}-${Date.now()}`,
          provider_id: providerId,
          display_name: m.id || m.display_name || m.model_id || "未知模型",
          max_tokens: m.max_tokens || m.max_total_tokens || 128000,
          supports_tools: m.supports_tools ?? m.tool_calls ?? true,
          cost_per_1k_tokens: 0,
          avg_quality_score: 0.5,
          avg_latency_ms: 0,
          is_available: true,
          api_key_configured: !!key && key.length > 5,
        }));

        if (remoteModels.length > 0) {
          const newModels = [...models.filter(m => m.provider_id !== providerId), ...remoteModels];
          setModels(newModels);
          // 保存到后端磁盘
          saveModels(newModels).catch(e => console.error("保存模型列表失败:", e));
          flashSave(`从 ${provider.display_name} 获取到 ${remoteModels.length} 个模型并已更新列表`);
        } else {
          flashSave("未获取到模型列表");
        }
      } else {
        flashSave(`获取失败: ${resp.status} ${resp.statusText}`);
      }
    } catch (e: any) {
      flashSave(`连接失败: ${e?.message || "未知错误"}`);
    }
    setFetchingModels(prev => ({ ...prev, [providerId]: false }));
  }

  // 删除供应商
  function handleDeleteProvider(providerId: string) {
    const updated = providers.filter(p => p.provider_id !== providerId);
    persistProviders(updated);
    const newModels = models.filter(m => m.provider_id !== providerId);
    setModels(newModels);
    saveModels(newModels).catch(e => console.error("保存模型列表失败:", e));
  }

  // 新增供应商
  function handleAddProvider() {
    if (!newProvider.display_name.trim() || !newProvider.api_base.trim()) return;
    const providerId = newProvider.provider_id || `provider-${Date.now()}`;
    const p: LlmProvider = {
      provider_id: providerId,
      name: newProvider.name || providerId,
      display_name: newProvider.display_name.trim(),
      api_base: newProvider.api_base.trim().replace(/\/+$/, ""),
      requires_api_key: newProvider.requires_api_key,
    };
    persistProviders([...providers, p]);
    setShowAddForm(false);
    setNewProvider({ provider_id: "", name: "", display_name: "", api_base: "", requires_api_key: true });
  }

  // 测试模型
  const handleTestModel = async (modelId: string) => {
    const model = models.find(m => m.model_id === modelId);
    if (!model) { flashSave("模型不存在"); setTestingModel(null); return; }
    const provider = providers.find(p => p.provider_id === model.provider_id);
    if (!provider) { flashSave("供应商不存在"); setTestingModel(null); return; }

    setTestingModel(modelId);
    setTestResults(prev => {
      const next = { ...prev };
      delete next[modelId];
      return next;
    });

    try {
      const headers: Record<string, string> = { "Content-Type": "application/json" };
      const key = apiKeys[provider.provider_id];
      if (key) headers["Authorization"] = "Bearer " + key;

      const baseUrl = provider.api_base.replace(/\/+$/, "");

      const startTime = Date.now();
      const resp = await httpRequest(baseUrl + "/chat/completions", "POST", headers, JSON.stringify({
        model: model.model_id,
        messages: [{ role: "user", content: "Hi" }],
        max_tokens: 5,
      }));
      const latency = Date.now() - startTime;

      if (resp.ok) {
        setTestResults(prev => ({ ...prev, [modelId]: 'success' }));
        flashSave("✅ " + model.display_name + " 响应正常 (" + latency + "ms)");
      } else {
        setTestResults(prev => ({ ...prev, [modelId]: 'error' }));
        flashSave("❌ " + model.display_name + " 测试失败: HTTP " + resp.status);
      }
    } catch (e: any) {
      setTestResults(prev => ({ ...prev, [modelId]: 'error' }));
      flashSave("❌ " + model.display_name + " 连接失败: " + (e?.message || "未知错误"));
    }

    setTimeout(() => {
      setTestResults(prev => {
        const next = { ...prev };
        delete next[modelId];
        return next;
      });
    }, 4000);

    setTestingModel(null);
  };

  // 切换模型启用
  const handleToggle = async (modelId: string, enabled: boolean) => {
    // user_managed=true：后端 list_models 不再因 api_key 存在而自动点亮该模型
    const newModels = models.map(m => m.model_id === modelId ? { ...m, is_available: enabled, user_managed: true } : m);
    setModels(newModels);
    // 启用状态持久化到 models.json（全局唯一数据源，所有功能共享）
    saveModels(newModels).catch(e => console.error("保存模型列表失败:", e));
  };

  // 设为全局默认模型（唯一）
  const handleSetDefault = async (modelId: string) => {
    const newModels = models.map(m => ({ ...m, is_default: m.model_id === modelId }));
    setModels(newModels);
    try {
      await setDefaultModel(modelId);
      flashSave("已设为默认模型");
    } catch (e) {
      console.error("设置默认模型失败:", e);
      loadFromBackend();
    }
  };

  // 获取供应商对应的模型列表
  function getModelsForProvider(providerId: string): LlmModel[] {
    return models.filter(m => m.provider_id === providerId);
  }

  // 按供应商分组
  const modelGroups = providers.map(p => ({
    provider: p,
    models: getModelsForProvider(p.provider_id),
  }));

  // 供应商标签配色
  function providerTag(id: string): { bg: string; fg: string } {
    const hues = [24, 210, 340, 153, 68, 195, 270, 330];
    let hash = 0;
    for (let i = 0; i < id.length; i++) hash = ((hash << 5) - hash) + id.charCodeAt(i);
    const h = hues[Math.abs(hash) % hues.length];
    return { bg: "oklch(92% 0.025 " + h + ")", fg: "oklch(28% 0.04 " + h + ")" };
  }

  if (loading) {
    return (
      <div className="view-container">
        <div className="view-header"><h2>模型设置</h2></div>
        <div className="empty-state">
          <div className="empty-state-text">加载中...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="view-container">
      <div className="view-header">
        <h2>模型设置</h2>
        {saveMsg && <span className="tag tag-success">{saveMsg}</span>}
      </div>

      <div className="tab-bar">
        <button className={"tab-item" + (activeTab === 'providers' ? " active" : "")} onClick={() => setActiveTab('providers')}>
          <Globe size={16} />
          <span>供应商</span>
        </button>
        <button className={"tab-item" + (activeTab === 'models' ? " active" : "")} onClick={() => setActiveTab('models')}>
          <Zap size={16} />
          <span>模型列表</span>
        </button>
      </div>

      {/* ============================
          供应商页签
          ============================ */}
      {activeTab === 'providers' && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(420px, 1fr))", gap: "var(--space-md)" }}>
            {providers.map(provider => {
              const connStatus = testingConn[provider.provider_id];
              const modelCount = getModelsForProvider(provider.provider_id).length;
              return (
                <div key={provider.provider_id} className="card" style={{ padding: "var(--space-lg)" }}>
                  {/* 供应商头部 */}
                  <div className="flex-between" style={{ marginBottom: "var(--space-md)" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <input
                        value={editingName[provider.provider_id] || provider.display_name}
                        onChange={e => setEditingName(prev => ({ ...prev, [provider.provider_id]: e.target.value }))}
                        onBlur={() => saveDisplayName(provider.provider_id)}
                        onKeyDown={e => e.key === 'Enter' && saveDisplayName(provider.provider_id)}
                        style={{
                          fontSize: "var(--text-md)", fontWeight: 500,
                          border: "none", borderBottom: "1px dashed var(--color-rule)",
                          background: "transparent", color: "var(--color-ink)",
                          padding: "2px 4px", fontFamily: "var(--font-ui)", width: 200,
                        }}
                        placeholder="供应商名称"
                      />
                    </div>
                    <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                      {connStatus === 'testing' && <Loader2 size={14} style={{ animation: "spin 1s linear infinite", color: "var(--color-ink-3)" }} />}
                      {connStatus === 'success' && <span style={{ color: "var(--color-jade)", fontSize: "var(--text-xs)" }}><Wifi size={14} /> 连通</span>}
                      {connStatus === 'error' && <span style={{ color: "var(--color-error)", fontSize: "var(--text-xs)" }}><WifiOff size={14} /> 失败</span>}
                      <button
                        className="btn btn-secondary"
                        style={{ padding: "4px 8px", fontSize: "11px", color: "var(--color-error)" }}
                        onClick={() => handleDeleteProvider(provider.provider_id)}
                        title="删除供应商"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>

                  {/* API 地址 */}
                  <div style={{ marginBottom: "var(--space-sm)" }}>
                    <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2, letterSpacing: "0.5px" }}>
                      <Globe size={11} style={{ verticalAlign: "middle", marginRight: 3 }} />
                      API 地址
                    </div>
                    <div style={{ display: "flex", gap: 4 }}>
                      <input
                        value={editingBase[provider.provider_id] || provider.api_base}
                        onChange={e => setEditingBase(prev => ({ ...prev, [provider.provider_id]: e.target.value }))}
                        onBlur={() => saveApiBase(provider.provider_id)}
                        onKeyDown={e => e.key === 'Enter' && saveApiBase(provider.provider_id)}
                        style={{
                          flex: 1, padding: "6px 10px",
                          border: "1px solid var(--color-rule)", borderRadius: "var(--radius-sm)",
                          fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)",
                          background: "var(--color-paper)", color: "var(--color-ink)",
                        }}
                        placeholder="https://api.example.com/v1"
                      />
                    </div>
                  </div>

                  {/* API Key */}
                  {provider.requires_api_key && (
                    <div style={{ marginBottom: "var(--space-sm)" }}>
                      <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2, letterSpacing: "0.5px" }}>
                        <Key size={11} style={{ verticalAlign: "middle", marginRight: 3 }} />
                        接口密钥
                      </div>
                      <div style={{ display: "flex", gap: 4 }}>
                        <div style={{ flex: 1, display: "flex", gap: 4 }}>
                          <input
                            type={showApiKey[provider.provider_id] ? "text" : "password"}
                            value={apiKeys[provider.provider_id] || ""}
                            onChange={e => {
                              const val = e.target.value;
                              setApiKeys(prev => ({ ...prev, [provider.provider_id]: val }));
                            }}
                            onBlur={() => {
                              const key = apiKeys[provider.provider_id] || "";
                              if (key) handleSaveApiKey(provider.provider_id, key);
                            }}
                            onKeyDown={e => {
                              if (e.key === 'Enter') {
                                const key = apiKeys[provider.provider_id] || "";
                                if (key) handleSaveApiKey(provider.provider_id, key);
                              }
                            }}
                            placeholder="sk-..."
                            style={{
                              flex: 1, padding: "6px 10px",
                              border: "1px solid var(--color-rule)", borderRadius: "var(--radius-sm)",
                              fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)",
                              background: "var(--color-paper)", color: "var(--color-ink)",
                            }}
                          />
                          <button
                            className="btn btn-secondary"
                            style={{ padding: "6px" }}
                            onClick={() => setShowApiKey(prev => ({ ...prev, [provider.provider_id]: !prev[provider.provider_id] }))}
                          >
                            {showApiKey[provider.provider_id] ? <EyeOff size={14} /> : <Eye size={14} />}
                          </button>
                        </div>
                      </div>
                      {apiKeys[provider.provider_id] && apiKeys[provider.provider_id].length > 5 && (
                        <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-jade)", marginTop: 2, display: "inline-block" }}>
                          <CheckCircle size={10} style={{ verticalAlign: "middle", marginRight: 2 }} />
                          已配置
                        </span>
                      )}
                    </div>
                  )}

                  {/* 操作按钮组 */}
                  <div style={{ display: "flex", gap: 6, marginTop: "var(--space-sm)", flexWrap: "wrap" }}>
                    <button
                      className="btn btn-secondary"
                      style={{ padding: "5px 12px", fontSize: "var(--text-xs)" }}
                      onClick={() => handleTestConnection(provider.provider_id)}
                      disabled={testingConn[provider.provider_id] === 'testing'}
                    >
                      {testingConn[provider.provider_id] === 'testing' ? (
                        <><Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} /> 测试中...</>
                      ) : (
                        <><TestTube size={12} /> 测试连接</>
                      )}
                    </button>
                    <button
                      className="btn btn-secondary"
                      style={{ padding: "5px 12px", fontSize: "var(--text-xs)" }}
                      onClick={() => handleFetchModels(provider.provider_id)}
                      disabled={!!fetchingModels[provider.provider_id]}
                    >
                      {fetchingModels[provider.provider_id] ? (
                        <><Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} /> 获取中...</>
                      ) : (
                        <><RefreshCw size={12} /> 获取模型</>
                      )}
                    </button>
                    <span style={{
                      fontSize: "var(--text-xs)", color: "var(--color-ink-3)",
                      alignSelf: "center", marginLeft: "auto",
                      background: "var(--color-paper-warm)", padding: "2px 8px",
                      borderRadius: "var(--radius-xs)",
                    }}>
                      {modelCount} 个模型
                    </span>
                  </div>
                </div>
              );
            })}
          </div>

          {/* 新增供应商 */}
          {!showAddForm ? (
            <button
              className="btn btn-secondary"
              style={{ marginTop: "var(--space-md)", padding: "10px 20px", width: "100%", justifyContent: "center" }}
              onClick={() => setShowAddForm(true)}
            >
              <Plus size={16} /> 新增供应商
            </button>
          ) : (
            <div className="card" style={{ marginTop: "var(--space-md)" }}>
              <div style={{ fontWeight: 500, fontSize: "var(--text-sm)", marginBottom: "var(--space-sm)" }}>新增供应商</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-sm)" }}>
                <div>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2 }}>显示名称 *</div>
                  <input
                    className="pm-input"
                    style={{ marginBottom: 0 }}
                    placeholder="如：智谱 AI"
                    value={newProvider.display_name}
                    onChange={e => setNewProvider(prev => ({ ...prev, display_name: e.target.value }))}
                  />
                </div>
                <div>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2 }}>标识 (英文)</div>
                  <input
                    className="pm-input"
                    style={{ marginBottom: 0 }}
                    placeholder="如：zhipu"
                    value={newProvider.provider_id}
                    onChange={e => setNewProvider(prev => ({ ...prev, provider_id: e.target.value }))}
                  />
                </div>
                <div style={{ gridColumn: "1 / -1" }}>
                  <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 2 }}>API 地址 *</div>
                  <input
                    className="pm-input"
                    style={{ marginBottom: 0 }}
                    placeholder="https://api.example.com/v1"
                    value={newProvider.api_base}
                    onChange={e => setNewProvider(prev => ({ ...prev, api_base: e.target.value }))}
                  />
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, marginTop: "var(--space-sm)" }}>
                <button className="btn btn-primary" onClick={handleAddProvider} disabled={!newProvider.display_name.trim() || !newProvider.api_base.trim()}>
                  <Plus size={14} /> 添加
                </button>
                <button className="btn btn-secondary" onClick={() => { setShowAddForm(false); setNewProvider({ provider_id: "", name: "", display_name: "", api_base: "", requires_api_key: true }); }}>
                  取消
                </button>
              </div>
            </div>
          )}

          {/* 没有供应商时的空状态 */}
          {providers.length === 0 && (
            <div className="empty-state" style={{ padding: "60px 20px" }}>
              <div className="empty-state-text">暂无供应商</div>
              <div className="empty-state-sub">点击「新增供应商」添加你的第一个 LLM 供应商</div>
            </div>
          )}
        </div>
      )}

      {/* ============================
          模型列表页签
          ============================ */}
      {activeTab === 'models' && (
        <div>
          {modelGroups.map(({ provider, models: provModels }) => (
            <div key={provider.provider_id} className="card" style={{ padding: 0, marginBottom: "var(--space-md)", overflow: "hidden" }}>
              {/* 供应商分组头部 */}
              <div
                style={{
                  display: "flex", alignItems: "center", gap: 8,
                  padding: "10px 16px",
                  background: "var(--color-paper-warm)",
                  borderBottom: "1px solid var(--color-rule-light)",
                  cursor: "pointer",
                  userSelect: "none",
                }}
                onClick={() => setExpandedProviders(prev => ({ ...prev, [provider.provider_id]: !prev[provider.provider_id] }))}
              >
                {expandedProviders[provider.provider_id] !== false ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                <div style={{
                  width: 6, height: 6, borderRadius: "50%",
                  background: provider.requires_api_key
                    ? (apiKeys[provider.provider_id]?.length > 5 ? "var(--color-jade)" : "var(--color-ochre)")
                    : "var(--color-jade)",
                  flexShrink: 0,
                }} />
                <span style={{ fontWeight: 500, fontSize: "var(--text-sm)" }}>{provider.display_name}</span>
                <span className="badge badge-draft" style={{ fontSize: "10px" }}>{provider.api_base}</span>
                <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginLeft: "auto" }}>
                  {provModels.length} 个模型
                </span>
              </div>

              {/* 模型列表 */}
              {expandedProviders[provider.provider_id] !== false && (
                <div>
                  {provModels.length === 0 ? (
                    <div style={{ padding: "20px", textAlign: "center", fontSize: "var(--text-sm)", color: "var(--color-ink-3)" }}>
                      暂无模型 — 点击上方「获取模型」从供应商同步
                    </div>
                  ) : (
                    provModels.map(model => (
                      <div key={model.model_id} className="list-item" style={{
                        padding: "14px 16px", display: "flex", alignItems: "center", gap: 12,
                        borderBottom: "1px solid var(--color-rule-light)",
                      }}>
                        {/* 模型信息 */}
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                            <span style={{ fontWeight: 500, fontSize: "var(--text-sm)" }}>{model.display_name}</span>
                            {model.is_default && (
                              <span className="tag tag-success" style={{ fontSize: "9px", display: "inline-flex", alignItems: "center", gap: 3 }}>
                                <Star size={9} /> 默认
                              </span>
                            )}
                            <span style={{
                              fontSize: "9px", padding: "1px 6px", borderRadius: "var(--radius-xs)",
                              background: providerTag(provider.provider_id).bg,
                              color: providerTag(provider.provider_id).fg,
                              fontWeight: 500, letterSpacing: "0.3px",
                            }}>
                              {provider.display_name}
                            </span>
                            {!model.api_key_configured && (
                              <span className="tag tag-warning" style={{ fontSize: "9px" }}>需配置密钥</span>
                            )}
                            {model.supports_tools && (
                              <span style={{ fontSize: "9px", color: "var(--color-indigo)" }}>工具调用</span>
                            )}
                          </div>
                          <div style={{ display: "flex", gap: 12, fontSize: "11px", color: "var(--color-ink-3)" }}>
                            <span>质量 {(model.avg_quality_score * 100).toFixed(0)}%</span>
                            <span>{model.avg_latency_ms > 0 ? `${model.avg_latency_ms}ms` : "-"}</span>
                            <span>{model.max_tokens.toLocaleString()} tokens</span>
                          </div>
                        </div>

                        {/* 操作 */}
                        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                          <button
                            className="btn btn-secondary"
                            style={{ padding: "3px 8px", fontSize: "10px" }}
                            onClick={() => handleSetDefault(model.model_id)}
                            disabled={!!model.is_default}
                            title={model.is_default ? "当前默认模型" : "设为全局默认模型（各环节未手动选择时优先使用）"}
                          >
                            <Star size={10} style={{ marginRight: 3, verticalAlign: -1 }} />
                            {model.is_default ? "默认" : "设默认"}
                          </button>
                          <button
                            className="btn btn-secondary"
                            style={{ padding: "4px 8px", fontSize: "11px" }}
                            onClick={() => handleTestModel(model.model_id)}
                            disabled={testingModel === model.model_id}
                            title="向模型发送测试请求检查连通性"
                          >
                            {testingModel === model.model_id ? <><Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} /> 测试中</> : <><TestTube size={11} /> 测试</>}
                          </button>
                          {testResults[model.model_id] && (
                            testResults[model.model_id] === 'success'
                              ? <span style={{ color: "var(--color-jade)", display: "flex", alignItems: "center", gap: 2, fontSize: "10px" }}><CheckCircle size={12} /> 正常</span>
                              : <span style={{ color: "var(--color-error)", display: "flex", alignItems: "center", gap: 2, fontSize: "10px" }}><XCircle size={12} /> 异常</span>
                          )}
                          <button
                            className={`btn ${model.is_available ? "btn-success" : "btn-secondary"}`}
                            style={{ padding: "3px 8px", fontSize: "10px", minWidth: 38 }}
                            onClick={() => handleToggle(model.model_id, !model.is_available)}
                          >
                            {model.is_available ? "ON" : "OFF"}
                          </button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              )}
            </div>
          ))}

          {modelGroups.length === 0 && (
            <div className="empty-state" style={{ padding: "60px 20px" }}>
              <div className="empty-state-text">暂无模型</div>
              <div className="empty-state-sub">请先在「供应商」页签中添加供应商并获取模型</div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
