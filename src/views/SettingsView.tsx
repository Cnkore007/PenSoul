// SettingsView — 全局 LLM 配置管理
// 标签页布局：配置管理 / 模型档案 / 上下文检测

import { useState, useEffect, useCallback, useRef } from "react";
import {
  listLlmConfigs,
  createLlmConfig,
  updateLlmConfig,
  deleteLlmConfig,
  setDefaultLlmConfig,
  getLlmStatus,
  testLlm,
  contextCheck,
  fetchModelDoc,
  pullLlmModels,
  listAgentConfigs,
  updateAgentConfig,
  addDistillCorpus,
  listDistillCorpus,
  deleteDistillCorpus,
  analyzeDistill,
  getStyleRecipe,
  updateStyleRecipe,
  deleteStyleRecipe,
  listOperations,
  rollbackOperations,
  compressArchive,
  listArchive,
  getCostReport,
} from "../ipc";
import type { StyleRecipe, BookSource, OperationLogList, CostReport } from "../ipc";
import type {
  LlmConfigs,
  LlmStatus,
  ProviderConfig,
  LlmTestResult,
  ContextCheckResult,
  PullModelsResult,
  ModelDocResult,
} from "../types";
import type { AgentConfigs } from "../ipc";
import { providerLabels, thinkingModeLabels, label } from "../labels";

type Tab = "configs" | "agents" | "distill" | "logs" | "context";
type EditMode = "new" | "edit" | null;

const EMPTY_FORM: Record<string, string> = {
  name: "",
  provider: "deepseek",
  model_id: "",
  base_url: "",
  api_key: "",
  context_window: "64000",
  max_output_tokens: "4096",
  thinking_mode: "None",
  supports_streaming: "true",
  temperature: "0.7",
  top_p: "",
  frequency_penalty: "",
  presence_penalty: "",
  stop_sequences: "",
  json_mode: "false",
  thinking_budget: "",
  timeout_seconds: "120",
  doc_url: "",
  notes: "",
  enabled: "true",
};

export default function SettingsView() {
  const [tab, setTab] = useState<Tab>("configs");

  // P3 风格蒸馏
  const [distillBooks, setDistillBooks] = useState<BookSource[]>([]);
  const [recipe, setRecipe] = useState<StyleRecipe | null>(null);
  const [distillMsg, setDistillMsg] = useState("");
  const [distillBusy, setDistillBusy] = useState(false);

  // P6 日志与归档
  const [logs, setLogs] = useState<OperationLogList | null>(null);
  const [cost, setCost] = useState<CostReport | null>(null);
  const [archiveVolumes, setArchiveVolumes] = useState<{ chapter_no: number; title: string; word_count: number }[]>([]);
  const [logsMsg, setLogsMsg] = useState("");
  const [rollbackN, setRollbackN] = useState("1");
  const [keepRecent, setKeepRecent] = useState("20");
  const [configs, setConfigs] = useState<LlmConfigs | null>(null);
  const [status, setStatus] = useState<LlmStatus | null>(null);
  const [msg, setMsg] = useState("");
  const [okMsg, setOkMsg] = useState("");

  // 编辑表单
  const [editMode, setEditMode] = useState<EditMode>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<Record<string, string>>({ ...EMPTY_FORM });
  const [saving, setSaving] = useState(false);

  // 测试
  const [testConfigId, setTestConfigId] = useState("");
  const [prompt, setPrompt] = useState("介绍一下你自己，一句话。");
  const [testResult, setTestResult] = useState<LlmTestResult | null>(null);
  const [testing, setTesting] = useState(false);

  // 模型列表拉取
  const [pullingId, setPullingId] = useState<string | null>(null);
  const [pulled, setPulled] = useState<PullModelsResult | null>(null);

  // 拉取模型的文档与参数（面板化：加载/失败/成功三态）
  const [docPanel, setDocPanel] = useState<{
    configId: string;
    modelId: string;
    loading: boolean;
    error: string | null;
    docUrl: string;
    result: ModelDocResult | null;
  } | null>(null);

  // 上下文检测
  const [ctxConfigId, setCtxConfigId] = useState("");
  const [ctxText, setCtxText] = useState("");
  const [ctxResult, setCtxResult] = useState<ContextCheckResult | null>(null);

  // Agent 注册表（P0b）
  const [agents, setAgents] = useState<AgentConfigs | null>(null);
  const [agentsMsg, setAgentsMsg] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [c, s] = await Promise.all([listLlmConfigs(), getLlmStatus()]);
      setConfigs(c);
      setStatus(s);
      setTestConfigId((prev) => prev || c.default_provider_id || c.providers[0]?.id || "");
      setCtxConfigId((prev) => prev || c.default_provider_id || c.providers[0]?.id || "");
    } catch (e: any) {
      setMsg(`加载失败: ${e}`);
    }
    listAgentConfigs().then(setAgents).catch(() => setAgentsMsg("Agent 配置加载失败"));
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  // ---- P3 风格蒸馏 ----

  const refreshDistill = useCallback(async () => {
    try {
      const [books, r] = await Promise.all([listDistillCorpus(), getStyleRecipe()]);
      setDistillBooks(books);
      setRecipe(r);
    } catch (e: any) {
      setDistillMsg(`风格配方加载失败: ${e}`);
    }
  }, []);

  useEffect(() => {
    if (tab === "distill") refreshDistill();
  }, [tab, refreshDistill]);

  const handleDistillUpload = async (file: File) => {
    setDistillMsg("");
    setDistillBusy(true);
    try {
      const ext = (file.name.split(".").pop() || "").toLowerCase();
      const format = ["txt", "md", "epub", "pdf"].includes(ext) ? ext : "txt";
      const b64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          const result = String(reader.result || "");
          resolve(result.split(",")[1] || "");
        };
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
      });
      const title = file.name.replace(/\.[^.]+$/, "");
      await addDistillCorpus(title, format, b64);
      setDistillMsg(
        `语料已摄取：${file.name}（已保存全本，点「开始蒸馏」时才调用模型做全本多点采样分析）。`,
      );
      await refreshDistill();
    } catch (e: any) {
      setDistillMsg(`语料上传失败: ${e}`);
    } finally {
      setDistillBusy(false);
    }
  };

  const handleDistillDelete = async (id: string) => {
    const inRecipe = recipe?.books.some((b) => b.id === id);
    const tip = inRecipe
      ? "该语料已参与当前配方，删除语料不会移除配方，可在配方区点「删除配方」或重新蒸馏。\n\n"
      : "";
    if (!confirm(`${tip}确定删除这本语料？`)) return;
    try {
      await deleteDistillCorpus(id);
      await refreshDistill();
    } catch (e: any) {
      setDistillMsg(`删除失败: ${e}`);
    }
  };

  const handleDistillAnalyze = async () => {
    if (distillBooks.length === 0) {
      setDistillMsg("请先上传至少一本语料。");
      return;
    }
    setDistillBusy(true);
    setDistillMsg("");
    try {
      const weights: Record<string, number> = {};
      distillBooks.forEach((b) => (weights[b.id] = b.weight));
      const result = await analyzeDistill(weights);
      // P2-5：正文缺失被跳过的语料显式提示
      const missing =
        result.missing_books && result.missing_books.length > 0
          ? `。⚠ 以下语料正文缺失已跳过：${result.missing_books.join("、")}`
          : "";
      setDistillMsg(
        `蒸馏完成：${result.books.join("、")} → ${result.dimension_count} 个维度特征 / ${result.gene_count} 条写作基因 / ${result.ban_count} 条禁用（强度 ${Math.round(result.strength * 100)}%，模型 ${result.model}）${missing}`,
      );
      await refreshDistill();
    } catch (e: any) {
      setDistillMsg(`蒸馏失败: ${e}`);
    } finally {
      setDistillBusy(false);
    }
  };

  const handleRecipeStrength = async (strength: number) => {
    try {
      await updateStyleRecipe(strength);
      setDistillMsg(`风格强度已调整为 ${Math.round(strength * 100)}%。`);
    } catch (e: any) {
      setDistillMsg(`强度调整失败: ${e}`);
    }
  };

  const handleDistillDeleteRecipe = async () => {
    if (!confirm("删除当前风格配方？语料会保留，删除后需重新蒸馏才能生成新配方。")) return;
    try {
      await deleteStyleRecipe();
      setDistillMsg("风格配方已删除。");
      await refreshDistill();
    } catch (e: any) {
      setDistillMsg(`配方删除失败: ${e}`);
    }
  };

  // P2-12 修复：滑块拖动只更新本地显示，停止拖动（400ms 防抖）后才落盘，避免请求风暴
  const strengthTimer = useRef<number | null>(null);
  const scheduleStrengthSave = (strength: number) => {
    setRecipe((prev) => (prev ? { ...prev, strength } : prev));
    if (strengthTimer.current !== null) window.clearTimeout(strengthTimer.current);
    strengthTimer.current = window.setTimeout(() => {
      handleRecipeStrength(strength);
    }, 400);
  };

  // ---- P6 日志与归档 ----

  const refreshLogs = useCallback(async () => {
    try {
      const [l, c] = await Promise.all([listOperations(30), getCostReport()]);
      setLogs(l);
      setCost(c);
    } catch (e: any) {
      setLogsMsg(`日志加载失败: ${e}`);
    }
  }, []);

  useEffect(() => {
    if (tab === "logs") refreshLogs();
  }, [tab, refreshLogs]);

  const handleRollback = async () => {
    if (!confirm(`回滚最近 ${rollbackN} 条自动操作（删除新建实体 / 恢复旧值）？此操作不可撤销。`)) return;
    setLogsMsg("");
    try {
      const result = await rollbackOperations(parseInt(rollbackN) || 1);
      // P2-1：日志截断失败时提示勿重复回滚
      setLogsMsg(
        result.log_warning
          ? `已回滚 ${result.rolled_back} 条：${result.undone.slice(0, 5).join("；")}${result.undone.length > 5 ? "…" : ""}。${result.log_warning}`
          : `已回滚 ${result.rolled_back} 条：${result.undone.slice(0, 5).join("；")}${result.undone.length > 5 ? "…" : ""}`,
      );
      await refreshLogs();
    } catch (e: any) {
      setLogsMsg(`回滚失败: ${e}`);
    }
  };

  const handleCompress = async () => {
    setLogsMsg("");
    try {
      const result = await compressArchive(parseInt(keepRecent) || 20);
      setLogsMsg(result.note);
      const archive = await listArchive();
      setArchiveVolumes(archive.volumes || []);
    } catch (e: any) {
      setLogsMsg(`归档失败: ${e}`);
    }
  };

  const handleLoadArchive = async () => {
    try {
      const archive = await listArchive();
      setArchiveVolumes(archive.volumes || []);
    } catch (e: any) {
      setLogsMsg(`归档读取失败: ${e}`);
    }
  };

  const handleAgentBind = async (roleId: string, llmConfigId: string | null) => {
    try {
      await updateAgentConfig(roleId, llmConfigId);
      setAgentsMsg(`角色「${roleId}」已绑定${llmConfigId ? "指定模型" : "全局默认"}。`);
      listAgentConfigs().then(setAgents).catch(() => {});
    } catch (e: any) {
      setAgentsMsg(`绑定失败: ${e}`);
    }
  };

  const showOk = (text: string) => {
    setOkMsg(text);
    setTimeout(() => setOkMsg(""), 3000);
  };

  const startNew = () => {
    setForm({ ...EMPTY_FORM });
    setEditingId(null);
    setEditMode("new");
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const startEdit = (config: ProviderConfig) => {
    setForm({
      name: config.name,
      provider: config.provider,
      model_id: config.model_id,
      base_url: config.base_url,
      api_key: "",
      context_window: String(config.context_window),
      max_output_tokens: String(config.max_output_tokens),
      thinking_mode: config.thinking_mode,
      supports_streaming: String(config.supports_streaming),
      temperature: config.temperature != null ? String(config.temperature) : "",
      top_p: config.top_p != null ? String(config.top_p) : "",
      frequency_penalty: config.frequency_penalty != null ? String(config.frequency_penalty) : "",
      presence_penalty: config.presence_penalty != null ? String(config.presence_penalty) : "",
      stop_sequences: config.stop_sequences || "",
      json_mode: String(config.json_mode === true),
      thinking_budget: config.thinking_budget != null ? String(config.thinking_budget) : "",
      timeout_seconds: String(config.timeout_seconds || 120),
      doc_url: config.doc_url || "",
      notes: config.notes || "",
      enabled: String(config.enabled),
    });
    setEditingId(config.id);
    setEditMode("edit");
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const handleSave = async () => {
    if (!form.name.trim() || !form.model_id.trim()) {
      setMsg("名称与模型 ID 必填");
      return;
    }
    setSaving(true);
    try {
      if (editMode === "new") {
        await createLlmConfig(form);
      } else if (editingId) {
        await updateLlmConfig({ ...form, id: editingId });
      }
      setEditMode(null);
      setEditingId(null);
      showOk(editMode === "new" ? "配置已创建" : "配置已更新");
      refresh();
    } catch (e: any) {
      setMsg(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (configId: string, name: string) => {
    if (!confirm(`确定删除配置 "${name}" 吗？`)) return;
    try {
      await deleteLlmConfig(configId);
      showOk("配置已删除");
      refresh();
    } catch (e: any) {
      setMsg(`删除失败: ${e}`);
    }
  };

  const handleSetDefault = async (configId: string) => {
    try {
      await setDefaultLlmConfig(configId);
      showOk("已设为默认配置");
      refresh();
    } catch (e: any) {
      setMsg(`设置失败: ${e}`);
    }
  };

  const handleTest = async (configId: string) => {
    setTestConfigId(configId);
    setTestResult(null);
    setTesting(true);
    try {
      setTestResult(await testLlm(configId, prompt));
      setTab("configs");
    } catch (e: any) {
      setMsg(`测试失败: ${e}`);
    } finally {
      setTesting(false);
    }
  };

  const handlePullModels = async (configId: string) => {
    setPullingId(configId);
    setPulled(null);
    try {
      const result = await pullLlmModels(configId);
      setPulled(result);
      showOk(`已拉取 ${result.models.length} 个模型`);
    } catch (e: any) {
      setMsg(`拉取失败: ${e}`);
    } finally {
      setPullingId(null);
    }
  };

  const handleFetchModelDoc = async (configId: string, modelId: string, docUrl = "") => {
    setDocPanel({
      configId,
      modelId,
      loading: true,
      error: null,
      docUrl,
      result: null,
    });
    try {
      const result = await fetchModelDoc(configId, modelId, docUrl || undefined);
      setDocPanel((p) => (p ? { ...p, loading: false, result } : p));
      showOk("模型文档已下载并完成参数提取");
    } catch (e: any) {
      setDocPanel((p) =>
        p ? { ...p, loading: false, error: String(e) } : p
      );
    }
  };

  const importModelParams = (modelId: string, result: ModelDocResult) => {
    const source = providers.find((p) => p.id === pulled?.config_id);
    setForm({
      ...EMPTY_FORM,
      name: modelId,
      provider: source?.provider || "custom",
      model_id: modelId,
      base_url: source?.base_url || "",
      context_window: result.params.context_window ? String(result.params.context_window) : "64000",
      max_output_tokens: result.params.max_output_tokens ? String(result.params.max_output_tokens) : "4096",
      thinking_mode: result.params.thinking_supported ? "Always" : "None",
      thinking_budget: result.params.thinking_supported ? "2048" : "",
      doc_url: result.suggested_url,
    });
    setEditingId(null);
    setEditMode("new");
    setTab("configs");
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

  const handleContextCheck = async () => {
    if (!ctxText.trim()) {
      setMsg("请输入要检测的文本");
      return;
    }
    try {
      setCtxResult(await contextCheck(ctxConfigId, null, ctxText));
    } catch (e: any) {
      setMsg(`检测失败: ${e}`);
    }
  };

  const set = (key: string, value: string) => setForm((f) => ({ ...f, [key]: value }));

  const providers = configs?.providers || [];
  const isDefault = (id: string) => configs?.default_provider_id === id;

  return (
    <div className="view-card">
      <h2>LLM 设定</h2>
      <p className="empty" style={{ marginTop: "-0.5rem", textAlign: "left" }}>
        统一管理模型配置、密钥与生成参数，配置只保存在本地。
      </p>
      {msg && <p className="msg">{msg}</p>}
      {okMsg && <p className="msg-ok">{okMsg}</p>}

      <div className="tab-bar">
        {([["configs", "配置管理"], ["agents", "Agent 模型"], ["distill", "风格蒸馏"], ["logs", "日志与归档"], ["context", "上下文检测"]] as [Tab, string][]).map(([key, label]) => (
          <button key={key} className={`tab-btn ${tab === key ? "active" : ""}`} onClick={() => setTab(key)}>
            {label}
          </button>
        ))}
      </div>

      {tab === "configs" && (
        <div>
          {status && (
            <div className="stats-row">
              <span>已配置密钥 {status.configured_count}/{status.total_count}</span>
              <span>默认配置：{status.has_default ? "有" : "未设置"}</span>
              <span>文件：data/{status.config_file}</span>
            </div>
          )}

          <div className="section">
            <div className="detail-header">
              <h3>全局配置</h3>
              <button className="btn-primary btn-sm" onClick={startNew}>+ 新增配置</button>
            </div>

            {editMode && (
              <div className="detail-panel llm-editor">
                <div className="detail-header">
                  <h3>{editMode === "new" ? "新增配置" : `编辑：${form.name || "未命名"}`}</h3>
                  <div className="btn-group">
                    <button className="btn-primary btn-sm" onClick={handleSave} disabled={saving}>
                      {saving ? "保存中..." : "保存"}
                    </button>
                    <button className="btn-sm" onClick={() => { setEditMode(null); setEditingId(null); }}>取消</button>
                  </div>
                </div>
                <div className="llm-form-grid">
                  <label className="llm-field">名称（Name）*
                    <input className="ps-input" value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="如 DeepSeek 官方" />
                  </label>
                  <label className="llm-field">供应商（Provider）
                    <select className="ps-input" value={form.provider} onChange={(e) => set("provider", e.target.value)}>
                      {Object.entries(providerLabels).map(([value, display]) => (
                        <option key={value} value={value}>{display}</option>
                      ))}
                    </select>
                  </label>
                  <label className="llm-field">模型 ID（Model ID）*
                    <input className="ps-input" value={form.model_id} onChange={(e) => set("model_id", e.target.value)} placeholder="如 deepseek-chat" />
                  </label>
                  <label className="llm-field">接口地址（Base URL）
                    <input className="ps-input" value={form.base_url} onChange={(e) => set("base_url", e.target.value)} placeholder="留空使用供应商默认地址" />
                  </label>
                  <label className="llm-field llm-field-wide">密钥（API Key）{editMode === "edit" && <em className="llm-hint">（留空表示不修改）</em>}
                    <input className="ps-input" type="password" value={form.api_key} onChange={(e) => set("api_key", e.target.value)} placeholder="sk-..." />
                  </label>
                  <label className="llm-field">上下文窗口（Context Window）
                    <input className="ps-input" type="number" min="1" value={form.context_window} onChange={(e) => set("context_window", e.target.value)} />
                  </label>
                  <label className="llm-field">最大输出 token（max_output_tokens）
                    <input className="ps-input" type="number" min="1" value={form.max_output_tokens} onChange={(e) => set("max_output_tokens", e.target.value)} />
                  </label>
                  <label className="llm-field">思考模式（Thinking Mode）
                    <select className="ps-input" value={form.thinking_mode} onChange={(e) => set("thinking_mode", e.target.value)}>
                      <option value="None">无（None）</option>
                      <option value="Always">总是（Always）</option>
                      <option value="Toggleable">可切换（Toggleable）</option>
                    </select>
                  </label>
                  <label className="llm-field">流式支持（Streaming）
                    <select className="ps-input" value={form.supports_streaming} onChange={(e) => set("supports_streaming", e.target.value)}>
                      <option value="true">支持</option>
                      <option value="false">不支持</option>
                    </select>
                  </label>
                  <label className="llm-field">温度（temperature，0~2）
                    <input className="ps-input" type="number" step="0.1" value={form.temperature} onChange={(e) => set("temperature", e.target.value)} placeholder="0.7" />
                  </label>
                  <label className="llm-field">概率采样（top_p，0~1）
                    <input className="ps-input" type="number" step="0.05" value={form.top_p} onChange={(e) => set("top_p", e.target.value)} placeholder="可空" />
                  </label>
                  <label className="llm-field">频率惩罚（frequency_penalty）
                    <input className="ps-input" type="number" step="0.1" value={form.frequency_penalty} onChange={(e) => set("frequency_penalty", e.target.value)} placeholder="-2 ~ 2" />
                  </label>
                  <label className="llm-field">存在惩罚（presence_penalty）
                    <input className="ps-input" type="number" step="0.1" value={form.presence_penalty} onChange={(e) => set("presence_penalty", e.target.value)} placeholder="-2 ~ 2" />
                  </label>
                  <label className="llm-field llm-field-wide">停止序列（stop，逗号分隔）
                    <input className="ps-input" value={form.stop_sequences} onChange={(e) => set("stop_sequences", e.target.value)} placeholder="如 </s>, 再见" />
                  </label>
                  <label className="llm-field">JSON 输出模式（json_mode）
                    <select className="ps-input" value={form.json_mode} onChange={(e) => set("json_mode", e.target.value)}>
                      <option value="false">关闭</option>
                      <option value="true">开启</option>
                    </select>
                  </label>
                  <label className="llm-field">思考预算（thinking_budget）
                    <input className="ps-input" type="number" min="0" value={form.thinking_budget} onChange={(e) => set("thinking_budget", e.target.value)} placeholder="如 2048" />
                  </label>
                  <label className="llm-field">请求超时（timeout_seconds，5~600 秒）
                    <input className="ps-input" type="number" min="5" max="600" value={form.timeout_seconds} onChange={(e) => set("timeout_seconds", e.target.value)} />
                  </label>
                  <label className="llm-field llm-field-wide">官方文档链接（Documentation URL）
                    <input className="ps-input" value={form.doc_url} onChange={(e) => set("doc_url", e.target.value)} placeholder="https://..." />
                  </label>
                  <label className="llm-field llm-field-wide">备注（Notes）
                    <textarea className="ps-input ps-textarea" value={form.notes} onChange={(e) => set("notes", e.target.value)} placeholder="用途、中转说明等" />
                  </label>
                  <label className="llm-field">启用状态（Enabled）
                    <select className="ps-input" value={form.enabled} onChange={(e) => set("enabled", e.target.value)}>
                      <option value="true">启用</option>
                      <option value="false">停用</option>
                    </select>
                  </label>
                </div>
              </div>
            )}

            {providers.length === 0 && !editMode && (
              <div className="llm-empty-guide">
                <p>还没有任何 LLM 配置。先手动新增一条配置，再在配置卡片上「拉取模型」获取供应商模型列表。</p>
                <div className="btn-group">
                  <button className="btn-primary btn-sm" onClick={startNew}>手动新增</button>
                </div>
              </div>
            )}

            <div className="llm-config-grid">
              {providers.map((c) => (
                <div key={c.id} className={`llm-config-card ${c.enabled ? "" : "llm-disabled"}`}>
                  <div className="llm-card-head">
                    <span className="tag-soft">{label(providerLabels, c.provider)}</span>
                    {isDefault(c.id) && <span className="tag-hard">默认</span>}
                    {!c.enabled && <span className="tag-soft">停用</span>}
                  </div>
                  <div className="llm-card-title">{c.name}</div>
                  <div className="llm-card-sub">{c.model_id}</div>
                  <div className="llm-card-line">{c.base_url || "未填地址"}</div>
                  <div className="llm-card-line">
                    {c.has_key
                      ? <span className="text-ok">密钥已配置 {c.api_key_masked}</span>
                      : <span className="text-warn">未配置密钥</span>}
                  </div>
                  <div className="llm-card-line">
                    窗口 {c.context_window.toLocaleString()} · 预算 {c.input_budget.toLocaleString()} · 输出 {c.max_output_tokens.toLocaleString()}
                  </div>
                  <div className="llm-card-line">
                    温度 {c.temperature ?? "默认"} · {label(thinkingModeLabels, c.thinking_mode)}{c.supports_streaming ? " · 流式" : ""}
                  </div>
                  <div className="llm-card-line">
                    {c.thinking_budget != null && `思考预算 ${c.thinking_budget} · `}
                    {c.json_mode ? "JSON 输出 · " : ""}
                    超时 {c.timeout_seconds}s
                  </div>
                  <div className="btn-group llm-card-actions">
                    <button className="btn-sm" onClick={() => startEdit(c)}>编辑</button>
                    <button className="btn-sm" onClick={() => handleTest(c.id)} disabled={testing && testConfigId === c.id}>
                      {testing && testConfigId === c.id ? "测试中..." : "测试"}
                    </button>
                    <button className="btn-sm" onClick={() => handlePullModels(c.id)} disabled={pullingId === c.id}>
                      {pullingId === c.id ? "拉取中..." : "拉取模型"}
                    </button>
                    {!isDefault(c.id) && (
                      <button className="btn-sm" onClick={() => handleSetDefault(c.id)}>设默认</button>
                    )}
                    <button className="btn-sm btn-danger" onClick={() => handleDelete(c.id, c.name)}>删除</button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {pulled && (
            <div className="detail-panel">
              <div className="detail-header">
                <h3>供应商模型列表（{pulled.models.length} 个）</h3>
                <button className="btn-sm" onClick={() => setPulled(null)}>收起</button>
              </div>
              <p className="llm-hint">点击任一模型可直接用它新建配置</p>
              <div className="llm-model-picker">
                {pulled.models.map((m) => (
                  <div
                    key={m.id}
                    className="llm-model-item"
                  >
                    <span className="llm-model-id">{m.id}</span>
                    {m.display_name && <span className="entity-detail">{m.display_name}</span>}
                    <div className="btn-group">
                      <button
                        className="btn-sm"
                        onClick={() => {
                          const source = providers.find((p) => p.id === pulled.config_id);
                          setForm({
                            ...EMPTY_FORM,
                            name: m.display_name || m.id,
                            provider: source?.provider || "custom",
                            model_id: m.id,
                            base_url: source?.base_url || "",
                            context_window: source ? String(source.context_window) : "64000",
                            max_output_tokens: source ? String(source.max_output_tokens) : "4096",
                            thinking_mode: source?.thinking_mode || "None",
                            temperature: source?.temperature != null ? String(source.temperature) : "0.7",
                            doc_url: source?.doc_url || "",
                          });
                          setEditingId(null);
                          setEditMode("new");
                          setTab("configs");
                          window.scrollTo({ top: 0, behavior: "smooth" });
                        }}
                      >
                        建档
                      </button>
                      <button
                        className="btn-sm"
                        onClick={() => handleFetchModelDoc(pulled.config_id, m.id)}
                      >
                        文档与参数
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {docPanel && (
            <div className="detail-panel">
              <div className="detail-header">
                <h3>{docPanel.modelId} 的文档与参数</h3>
                <button className="btn-sm" onClick={() => setDocPanel(null)}>收起</button>
              </div>

              {docPanel.loading && (
                <p className="llm-hint">正在定位并抓取官方文档，可能需要十几秒...</p>
              )}

              {!docPanel.loading && docPanel.error && (
                <div>
                  <p className="msg">文档获取失败：{docPanel.error}</p>
                  <p className="llm-hint">可手动填写该模型的官方文档地址后重试：</p>
                  <div className="form-row">
                    <input
                      className="ps-input"
                      value={docPanel.docUrl}
                      onChange={(e) =>
                        setDocPanel((p) => (p ? { ...p, docUrl: e.target.value } : p))
                      }
                      placeholder="如 https://platform.moonshot.cn/docs"
                    />
                    <button
                      className="btn-primary btn-sm"
                      disabled={!docPanel.docUrl.trim()}
                      onClick={() =>
                        handleFetchModelDoc(docPanel.configId, docPanel.modelId, docPanel.docUrl.trim())
                      }
                    >
                      重试抓取
                    </button>
                  </div>
                </div>
              )}

              {!docPanel.loading && docPanel.result && (
                <>
                  <div className="stats-row">
                    <span>上下文窗口：{docPanel.result.params.context_window?.toLocaleString() ?? "未识别"}</span>
                    <span>最大输出：{docPanel.result.params.max_output_tokens?.toLocaleString() ?? "未识别"}</span>
                    <span>思考模式：{docPanel.result.params.thinking_supported ? "支持" : docPanel.result.params.thinking_supported === false ? "不支持" : "未识别"}</span>
                  </div>
                  {docPanel.result.params.notes.length > 0 && (
                    <ul className="constraint-list">
                      {docPanel.result.params.notes.map((note) => (
                        <li key={note}>{note}</li>
                      ))}
                    </ul>
                  )}
                  {docPanel.result.params.sources.length > 0 && (
                    <p className="llm-hint">
                      摘取来源：
                      {docPanel.result.params.sources.map((s, i) => (
                        <span key={s.url}>
                          {i > 0 && " · "}
                          <a href={s.url} target="_blank" rel="noreferrer">{s.title || s.url}</a>
                        </span>
                      ))}
                    </p>
                  )}
                  {docPanel.result.doc.description && <p>{docPanel.result.doc.description}</p>}
                  <p className="llm-hint">来源：<a href={docPanel.result.suggested_url} target="_blank" rel="noreferrer">{docPanel.result.suggested_url}</a></p>
                  <p className="llm-hint">本地文件：{docPanel.result.doc.saved_file}（抓取于 {docPanel.result.doc.fetched_at}）</p>
                  {docPanel.result.doc.text_preview && (
                    <div className="llm-doc-preview">
                      <strong>内容预览</strong>
                      <p>{docPanel.result.doc.text_preview}</p>
                    </div>
                  )}
                  <div className="btn-group" style={{ marginTop: "0.75rem" }}>
                    <button className="btn-primary btn-sm" onClick={() => importModelParams(docPanel.result!.doc.model_id, docPanel.result!)}>
                      导入参数并建档
                    </button>
                  </div>
                </>
              )}
            </div>
          )}

          <div className="section">
            <h3>连接测试</h3>
            <div className="form-row">
              <select className="ps-input" value={testConfigId} onChange={(e) => setTestConfigId(e.target.value)}>
                <option value="">选择配置...</option>
                {providers.map((c) => (
                  <option key={c.id} value={c.id}>{c.name}（{c.model_id}）</option>
                ))}
              </select>
            </div>
            <textarea className="ps-input ps-textarea" value={prompt} onChange={(e) => setPrompt(e.target.value)} />
            <button className="btn-primary" onClick={() => handleTest(testConfigId)} disabled={testing || !testConfigId}>
              {testing ? "测试中..." : "测试连接"}
            </button>
            {testResult && (
              <div className="report">
                <p><strong>{testResult.model}</strong></p>
                <p>{testResult.content}</p>
                {testResult.usage && (
                  <p className="llm-hint">提示 {testResult.usage.prompt_tokens} · 输出 {testResult.usage.completion_tokens} · 总计 {testResult.usage.total_tokens}</p>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      {tab === "context" && (
        <div className="section">
          <h3>上下文检测</h3>
          <p className="llm-hint">估算文本占用，对照模型输入预算，避免创作时超出窗口。</p>
          <div className="form-row">
            <select className="ps-input" value={ctxConfigId} onChange={(e) => setCtxConfigId(e.target.value)}>
              <option value="">选择配置...</option>
              {providers.map((c) => (
                <option key={c.id} value={c.id}>{c.name}（{c.model_id}）</option>
              ))}
            </select>
          </div>
          <textarea className="ps-input ps-textarea" value={ctxText} onChange={(e) => setCtxText(e.target.value)} placeholder="粘贴要检测的文本，如某章草稿或设定汇总..." />
          <button className="btn-primary" onClick={handleContextCheck}>开始检测</button>
          {ctxResult && (
            <div className="report">
              <p className={ctxResult.fits ? "text-ok" : "text-warn"}>
                {ctxResult.fits ? "可以放入上下文" : "超出输入预算，需要裁剪"}
              </p>
              <p>字符 {ctxResult.chars}（中文 {ctxResult.cjk_chars}）· 估算 {ctxResult.estimated_tokens} tokens</p>
              <p>窗口 {ctxResult.context_window.toLocaleString()} · 输入预算 {ctxResult.input_budget.toLocaleString()} · 占用 {ctxResult.usage_percent}%</p>
            </div>
          )}
        </div>
      )}

      {tab === "agents" && (
        <div>
          <div className="section">
            <h3>Agent 模型配置</h3>
            <p className="llm-hint">
              写作、审校等角色可各自绑定不同的 LLM 模型（如写作用高质量模型、审校用低成本模型）。
              留空 = 使用全局默认配置。审校建议与写作使用不同模型，以保持独立视角。
            </p>
            {agentsMsg && <p className="msg">{agentsMsg}</p>}
            {agents && agents.agents.length > 0 ? (
              <table className="ps-table">
                <thead>
                  <tr>
                    <th>角色</th>
                    <th>当前模型</th>
                    <th>绑定模型</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {agents.agents.map((a) => (
                    <tr key={a.role_id}>
                      <td>{a.display_name}<em className="llm-hint">（{a.role_id}）</em></td>
                      <td>{a.bound_model ? `${a.bound_model.name}（${a.bound_model.model_id}）` : "全局默认"}</td>
                      <td>
                        <select
                          className="ps-input"
                          defaultValue={a.llm_config_id || ""}
                          onChange={(e) => handleAgentBind(a.role_id, e.target.value || null)}
                        >
                          <option value="">全局默认</option>
                          {configs?.providers.map((c) => (
                            <option key={c.id} value={c.id}>{c.name}（{c.model_id}）</option>
                          ))}
                        </select>
                      </td>
                      <td>
                        {a.llm_config_id ? (
                          <button className="btn-sm" onClick={() => handleAgentBind(a.role_id, null)}>
                            重置为默认
                          </button>
                        ) : (
                          <span className="text-ok">未绑定</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <p className="empty">暂无 Agent 配置。</p>
            )}
          </div>
        </div>
      )}

      {tab === "logs" && (
        <div>
          <div className="section">
            <h3>成本档位</h3>
            {cost ? (
              <div className="gen-meta-grid">
                <span className="gen-meta-item">档位: <b>{cost.tier}</b></span>
                <span className="gen-meta-item">事实提取 {cost.fact_extract_count} 次</span>
                <span className="gen-meta-item">级联同步 {cost.cascade_count} 次</span>
                <span className="gen-meta-item">蒸馏语料 {cost.distilled_books} 本</span>
              </div>
            ) : (
              <p className="empty">暂无成本数据。</p>
            )}
            <p className="llm-hint">{cost?.note}</p>
          </div>

          <div className="section">
            <h3>归档压缩</h3>
            <div className="form-row">
              <input
                className="ps-input ps-input-sm"
                value={keepRecent}
                onChange={(e) => setKeepRecent(e.target.value)}
                placeholder="保留最近 N 章"
                style={{ width: 120 }}
              />
              <button className="btn-sm" onClick={handleCompress}>执行归档</button>
              <button className="btn-sm" onClick={handleLoadArchive}>查看卷摘要</button>
            </div>
            {archiveVolumes.length > 0 && (
              <table className="ps-table">
                <thead>
                  <tr><th>章</th><th>标题</th><th>字数</th></tr>
                </thead>
                <tbody>
                  {archiveVolumes.map((v) => (
                    <tr key={v.chapter_no}>
                      <td>{v.chapter_no}</td>
                      <td>{v.title}</td>
                      <td>{v.word_count.toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="section">
            <h3>操作日志与全局回滚</h3>
            <p className="llm-hint">
              事实提取与级联同步的自动写入均有审计轨迹。回滚会逆应用最近 N 条操作（删除新建实体、恢复角色旧值、回退伏笔状态）。
            </p>
            <div className="form-row">
              <select className="ps-input ps-input-sm" value={rollbackN} onChange={(e) => setRollbackN(e.target.value)}>
                <option value="1">最近 1 条</option>
                <option value="5">最近 5 条</option>
                <option value="10">最近 10 条</option>
              </select>
              <button className="btn-sm btn-danger" onClick={handleRollback}>回滚</button>
            </div>
            {logsMsg && <p className="msg">{logsMsg}</p>}
            {logs && logs.entries.length > 0 ? (
              <ul className="log-list">
                {logs.entries.map((entry, i) => (
                  <li key={i} className="log-item">
                    <span className="log-time">{entry.time}</span>
                    {entry.applied.length > 0 && <span className="log-applied">{entry.applied.join("；")}</span>}
                    {entry.warnings.length > 0 && <span className="gen-meta-warn">{entry.warnings.join("；")}</span>}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="empty">暂无操作记录。</p>
            )}
          </div>
        </div>
      )}

      {tab === "distill" && (
        <div>
          <div className="section">
            <h3>书籍蒸馏 · 风格配方</h3>
            <p className="llm-hint">
              上传书籍（txt / md / epub / pdf），蒸馏出可执行的写作风格配方并注入 AI 生成与审校。
              只提炼抽象风格规律，不保留原书句子（版权红线）。多书可混合，权重调整后重新分析。
            </p>
            {distillMsg && <p className="msg">{distillMsg}</p>}
            <div className="distill-upload">
              <input
                type="file"
                accept=".txt,.md,.epub,.pdf"
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) handleDistillUpload(file);
                  e.target.value = "";
                }}
                disabled={distillBusy}
              />
              <button className="btn-sm" onClick={handleDistillAnalyze} disabled={distillBusy || distillBooks.length === 0}>
                {distillBusy ? "处理中…" : "开始蒸馏"}
              </button>
            </div>
            {distillBooks.length > 0 && (
              <table className="ps-table">
                <thead>
                  <tr>
                    <th>书名</th>
                    <th>格式</th>
                    <th>采样字数</th>
                    <th>权重</th>
                    <th>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {distillBooks.map((b) => (
                    <tr key={b.id}>
                      <td>{b.title}</td>
                      <td>{b.format}</td>
                      <td>{b.chars.toLocaleString()}</td>
                      <td>{b.weight.toFixed(1)}</td>
                      <td>
                        <button className="btn-mini danger" onClick={() => handleDistillDelete(b.id)}>删除</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            {recipe && recipe.books.length > 0 && (
              <div className="recipe-view">
                <h4>
                  当前配方（{recipe.books.map((b) => b.title).join(" + ")}）
                  <button className="btn-mini danger" onClick={handleDistillDeleteRecipe} disabled={distillBusy}>
                    删除配方
                  </button>
                </h4>
                <div className="recipe-strength">
                  <label>风格强度：{Math.round(recipe.strength * 100)}%</label>
                  <input
                    type="range"
                    min={0.3}
                    max={1}
                    step={0.05}
                    value={recipe.strength}
                    onChange={(e) => scheduleStrengthSave(parseFloat(e.target.value))}
                  />
                </div>
                {recipe.dimensions.map((d) => (
                  <p key={d.dimension} className="recipe-dim">
                    <b>{d.dimension}</b>：{d.features.join("；")}
                  </p>
                ))}
                {recipe.genes.length > 0 && (
                  <p className="recipe-genes"><b>写作基因</b>：{recipe.genes.join("；")}</p>
                )}
                {recipe.bans.length > 0 && (
                  <p className="recipe-bans"><b>禁用清单</b>：{recipe.bans.join("；")}</p>
                )}
                <p className="llm-hint">模型: {recipe.model} · 生成于 {recipe.generated_at}</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
