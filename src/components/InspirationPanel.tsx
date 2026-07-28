import { useState, useCallback, useEffect, useRef } from "react";
import { Sparkles, Lightbulb, RefreshCw, ChevronDown, ChevronUp } from "lucide-react";
import type { InspirationItem } from "../types";
import { loadProviders, loadModels, loadApiKeys } from "../store";
import { httpRequest } from "../ipc";

interface InspirationPanelProps {
  /** 创作上下文类型：character | world | outline | writing */
  contextType: string;
  /** 当前项目上下文数据（JSON），用于 LLM 生成灵感 */
  contextData: string;
  /** 外部控制展开状态（可选，默认内部管理） */
  externalExpanded?: boolean;
  /** 展开状态变化回调 */
  onToggle?: () => void;
  /** 是否隐藏触发按钮（标题栏已有入口时使用） */
  hideTrigger?: boolean;
}

// 根据上下文类型和数据生成 system prompt
function buildPrompt(contextType: string, contextData: string): { system: string; user: string } {
  const base = "你是一位创意写作助手，擅长为小说创作提供灵感建议。请用中文回答。";
  const data = contextData ? JSON.parse(contextData) : {};

  switch (contextType) {
    case "outline":
      return {
        system: base,
        user: `当前大纲结构：${JSON.stringify(data, null, 2)}\n\n请基于此大纲给出 3 条创作灵感建议，每条包含 title（简短标题）和 content（详细说明，2-3 句话）。以 JSON 数组格式返回，例如：[{"title":"标题","content":"内容"}]`,
      };
    case "world":
      return {
        system: base,
        user: `当前世界观设定：${JSON.stringify(data, null, 2)}\n\n请基于此世界观给出 3 条灵感建议，可以是新地点、新事件或新规则的构思。每条包含 title 和 content。以 JSON 数组格式返回。`,
      };
    case "character":
      return {
        system: base,
        user: `当前角色设定：${JSON.stringify(data, null, 2)}\n\n请基于此角色列表给出 3 条灵感建议，可以是新角色构思或现有角色的发展方向。每条包含 title 和 content。以 JSON 数组格式返回。`,
      };
    default:
      return {
        system: base,
        user: `请给出 3 条创作灵感建议。每条包含 title 和 content。以 JSON 数组格式返回。`,
      };
  }
}

// 从 LLM 响应中解析灵感列表
function parseInspirationResponse(text: string): InspirationItem[] {
  // 尝试找到 JSON 数组
  const jsonMatch = text.match(/\[[\s\S]*\]/);
  if (jsonMatch) {
    try {
      const parsed = JSON.parse(jsonMatch[0]);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((item: any) => item.title && item.content)
          .map((item: any) => ({ title: String(item.title), content: String(item.content) }));
      }
    } catch {}
  }
  // 回退：按行解析
  const lines = text.split("\n").filter(l => l.trim());
  const items: InspirationItem[] = [];
  for (let i = 0; i < lines.length && items.length < 5; i++) {
    const line = lines[i].trim();
    if (line.startsWith("- ") || line.startsWith("· ") || /^\d+[.、]/.test(line)) {
      const clean = line.replace(/^[-·]\s*|^\d+[.、]\s*/, "");
      const colonIdx = clean.indexOf("：");
      if (colonIdx > 0) {
        items.push({ title: clean.slice(0, colonIdx), content: clean.slice(colonIdx + 1) });
      } else if (colonIdx === -1 && clean.indexOf(":") > 0) {
        const ci = clean.indexOf(":");
        items.push({ title: clean.slice(0, ci), content: clean.slice(ci + 1) });
      }
    }
  }
  return items;
}

export function InspirationPanel({ contextType, contextData, externalExpanded, onToggle, hideTrigger }: InspirationPanelProps) {
  const [internalExpanded, setInternalExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [items, setItems] = useState<InspirationItem[]>([]);
  const [hasGenerated, setHasGenerated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string>("");
  const generatedRef = useRef(false);

  const expanded = externalExpanded !== undefined ? externalExpanded : internalExpanded;

  // 从 localStorage 加载可用模型
  const providers = loadProviders();
  const allModels = loadModels();
  const apiKeys = loadApiKeys();
  const availableModels = allModels.filter(m => m.is_available && m.api_key_configured);

  // 初始化选中的模型
  useEffect(() => {
    if (!selectedModelId && availableModels.length > 0) {
      setSelectedModelId(availableModels[0].model_id);
    }
  }, [availableModels, selectedModelId]);

  const toggleExpanded = useCallback(() => {
    if (onToggle) onToggle();
    else setInternalExpanded(prev => !prev);
  }, [onToggle]);

  // 通过 httpRequest 调用 LLM API
  const callLlmApi = useCallback(async (modelId: string, systemPrompt: string, userPrompt: string): Promise<string> => {
    const model = allModels.find(m => m.model_id === modelId);
    if (!model) throw new Error("未找到模型: " + modelId);
    const provider = providers.find(p => p.provider_id === model.provider_id);
    if (!provider) throw new Error("未找到供应商: " + model.provider_id);
    const apiKey = apiKeys[provider.provider_id];
    if (!apiKey) throw new Error("未配置 API Key: " + provider.display_name);

    const baseUrl = provider.api_base.replace(/\/+$/, "");
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "Authorization": "Bearer " + apiKey,
    };

    // Anthropic 使用不同的 API 格式
    if (provider.provider_id === "anthropic") {
      const resp = await httpRequest(baseUrl + "/v1/messages", "POST", headers, JSON.stringify({
        model: modelId,
        max_tokens: 1024,
        system: systemPrompt,
        messages: [{ role: "user", content: userPrompt }],
      }));
      if (!resp.ok) throw new Error("API 请求失败: HTTP " + resp.status + " — " + resp.body.slice(0, 200));
      const data = JSON.parse(resp.body);
      return data.content?.[0]?.text || "";
    }

    // OpenAI 兼容格式
    const resp = await httpRequest(baseUrl + "/chat/completions", "POST", headers, JSON.stringify({
      model: modelId,
      messages: [
        { role: "system", content: systemPrompt },
        { role: "user", content: userPrompt },
      ],
      max_tokens: 1024,
      temperature: 0.8,
    }));
    if (!resp.ok) throw new Error("API 请求失败: HTTP " + resp.status + " — " + resp.body.slice(0, 200));
    const data = JSON.parse(resp.body);
    return data.choices?.[0]?.message?.content || "";
  }, [allModels, providers, apiKeys]);

  const handleGenerate = useCallback(async () => {
    const modelId = selectedModelId || availableModels[0]?.model_id;
    if (!modelId) {
      setError("请先在「模型设置」中配置并启用至少一个模型");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const { system, user } = buildPrompt(contextType, contextData);
      const raw = await callLlmApi(modelId, system, user);
      const parsed = parseInspirationResponse(raw);
      setItems(parsed);
      setHasGenerated(true);
      generatedRef.current = true;
    } catch (e: any) {
      setError(e?.message || "生成失败");
      setItems([]);
      setHasGenerated(true);
    } finally {
      setLoading(false);
    }
  }, [contextType, contextData, selectedModelId, availableModels, callLlmApi]);

  // hideTrigger 模式下，展开时自动触发生成
  useEffect(() => {
    if (hideTrigger && expanded && !generatedRef.current && !loading) {
      generatedRef.current = true;
      handleGenerate();
    }
  }, [hideTrigger, expanded, loading, handleGenerate]);

  // 关闭时重置 generatedRef，下次展开可重新生成
  useEffect(() => {
    if (!expanded) {
      generatedRef.current = false;
    }
  }, [expanded]);

  return (
    <div className="inspiration-panel" style={{
      borderTop: hasGenerated ? "1px solid var(--color-border)" : "none",
      marginTop: hasGenerated ? "var(--space-md)" : 0,
    }}>
      {/* 触发按钮 */}
      {!hideTrigger && (
        <div
          style={{
            display: "flex", alignItems: "center", gap: 8,
            padding: "8px 0", cursor: "pointer", userSelect: "none",
          }}
          onClick={toggleExpanded}
        >
          <Sparkles size={16} style={{ color: "var(--color-accent)" }} />
          <span style={{ fontSize: "var(--text-sm)", fontWeight: 500, color: "var(--color-accent)", flex: 1 }}>
            灵感
          </span>
          {loading && <RefreshCw size={14} style={{ color: "var(--color-ink-faint)", animation: "spin 1s linear infinite" }} />}
          {hasGenerated && !loading && (
            <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-faint)" }}>
              {items.length > 0 ? items.length + " 条建议" : error ? "生成失败" : ""}
            </span>
          )}
          {hasGenerated && (expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />)}
        </div>
      )}

      {/* 灵感内容 */}
      {expanded && (
        <div style={{
          display: "flex", flexDirection: "column", gap: 10,
          paddingBottom: "var(--space-sm)",
        }}>
          {/* 模型选择器 */}
          {availableModels.length > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", flexShrink: 0 }}>模型:</span>
              <select
                value={selectedModelId}
                onChange={e => setSelectedModelId(e.target.value)}
                disabled={loading}
                style={{
                  fontSize: "var(--text-xs)", padding: "3px 8px",
                  border: "1px solid var(--color-rule)", borderRadius: "var(--radius-sm)",
                  background: "var(--color-paper)", color: "var(--color-ink)",
                  fontFamily: "var(--font-mono)", maxWidth: 260,
                }}
              >
                {availableModels.map(m => {
                  const p = providers.find(pp => pp.provider_id === m.provider_id);
                  return (
                    <option key={m.model_id} value={m.model_id}>
                      {m.display_name} ({p?.display_name || m.provider_id})
                    </option>
                  );
                })}
              </select>
            </div>
          )}

          {/* 加载中 */}
          {loading && (
            <div style={{
              display: "flex", alignItems: "center", gap: 8,
              padding: "12px 0", color: "var(--color-ink-3)", fontSize: "var(--text-sm)",
            }}>
              <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} />
              正在生成灵感建议...
            </div>
          )}

          {/* 错误 */}
          {error && !loading && (
            <div style={{
              padding: "10px 12px", background: "oklch(95% 0.02 25)", borderRadius: "var(--radius-sm)",
              fontSize: "var(--text-xs)", color: "var(--color-error)", lineHeight: 1.6,
            }}>
              {error}
            </div>
          )}

          {/* 空状态 */}
          {!loading && hasGenerated && items.length === 0 && !error && (
            <div style={{ fontSize: "var(--text-sm)", color: "var(--color-ink-faint)", padding: "12px 0", textAlign: "center" }}>
              暂无灵感建议
            </div>
          )}

          {/* 灵感卡片 */}
          {items.map((item, i) => (
            <div key={i} style={{
              background: "var(--color-paper-warm)", borderRadius: 8,
              padding: "10px 12px", borderLeft: "3px solid var(--color-accent)",
            }}>
              <div style={{ display: "flex", alignItems: "flex-start", gap: 6 }}>
                <Lightbulb size={14} style={{ color: "var(--color-accent)", flexShrink: 0, marginTop: 2 }} />
                <div>
                  <div style={{ fontSize: "var(--text-sm)", fontWeight: 600, marginBottom: 2, color: "var(--color-ink)" }}>
                    {item.title}
                  </div>
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-2)", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                    {item.content}
                  </div>
                </div>
              </div>
            </div>
          ))}

          {/* 换一批按钮 */}
          {!loading && hasGenerated && items.length > 0 && (
            <button className="btn btn-ghost" style={{ fontSize: "var(--text-xs)", padding: "4px 8px", alignSelf: "flex-end", gap: 4 }} onClick={handleGenerate}>
              <RefreshCw size={12} /> 换一批
            </button>
          )}

          {/* 无可用模型提示 */}
          {availableModels.length === 0 && !loading && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ochre)", padding: "8px 0" }}>
              请先在「模型设置」中添加供应商、配置 API Key 并启用模型
            </div>
          )}
        </div>
      )}
    </div>
  );
}
