# PenSoul IPC 契约（前后端统一）

> 本文档是前后端 IPC 通信的单一事实来源。所有命令名、参数、返回值以此为准。

## 命名规则

- IPC 命令名：`snake_case`（如 `create_project`）
- 前端封装函数：`camelCase`（如 `createProject`）
- 参数名：`snake_case`
- 前端类型接口：`PascalCase`

---

## 1. 项目管理（6 个命令）

### `create_project`
- **参数**：`{ title: string }`
- **返回**：`string`（项目 ID）
- **说明**：创建新项目，初始化 ontology 并持久化

### `list_projects`
- **参数**：无
- **返回**：`ProjectMeta[]`
- **说明**：列出所有已创建的项目元数据

### `get_project`
- **参数**：`{ project_id: string }`
- **返回**：`Project`
- **说明**：获取项目详情（含卷、章节数、总字数）

### `update_project`
- **参数**：`{ project_id: string, title: string, description: string }`
- **返回**：`void`
- **说明**：更新项目标题和描述

### `delete_project`
- **参数**：`{ project_id: string }`
- **返回**：`void`
- **说明**：删除项目及其所有数据

### `open_project`
- **参数**：`{ project_id: string }`
- **返回**：`void`
- **说明**：打开/切换当前活跃项目，加载 ontology 到内存

---

## 2. 章节管理（3 个命令）

### `list_chapters`
- **参数**：无
- **返回**：`Chapter[]`
- **说明**：列出当前项目所有章节

### `get_chapter`
- **参数**：`{ chapter_id: string }`
- **返回**：`Chapter`
- **说明**：获取章节完整内容

### `save_chapter`
- **参数**：`{ chapter_id: string, content: string, expected_version: number }`
- **返回**：`number`（新版本号）
- **说明**：保存章节内容，带乐观锁并发控制。版本冲突时抛出错误。

---

## 3. 角色与世界观（4 个命令）

### `get_characters`
- **参数**：无
- **返回**：`Character[]`
- **说明**：获取当前项目所有角色

### `save_characters`
- **参数**：`{ characters: Character[] }`
- **返回**：`void`
- **说明**：保存角色列表（全量替换）

### `get_world`
- **参数**：无
- **返回**：`WorldLayer`
- **说明**：获取世界观数据（地点、时间线、设定规则）

### `save_world`
- **参数**：`{ world: WorldLayer }`
- **返回**：`void`
- **说明**：保存世界观数据（全量替换）

---

## 4. 创作设定（6 个命令）

### `save_settings`
- **参数**：`{ settings: ProjectSettings }`
- **返回**：`void`
- **说明**：保存创作规划设定

### `load_settings`
- **参数**：无
- **返回**：`ProjectSettings`
- **说明**：读取创作规划设定

### `save_concept`
- **参数**：`{ concept: CoreConcept }`
- **返回**：`void`
- **说明**：保存核心概念（高概念、前提、主角雏形等）

### `load_concept`
- **参数**：无
- **返回**：`CoreConcept`
- **说明**：读取核心概念

### `save_sprout`
- **参数**：`{ sprout: SproutData }`
- **返回**：`void`
- **说明**：保存萌芽数据（想法描述 + Agent 讨论配置）

### `load_sprout`
- **参数**：无
- **返回**：`SproutData`
- **说明**：读取萌芽数据

---

## 5. Harness 流程引擎（4 个命令）

### `get_harness_status`
- **参数**：无
- **返回**：`HarnessStatus { current_stage, stage_status, memo, retry_count }`
- **说明**：获取引擎当前状态

### `start_harness_stage`
- **参数**：无
- **返回**：`StageInstance`（JSON）
- **说明**：启动下一个 Harness 阶段

### `complete_harness_stage`
- **参数**：`{ result: Value }`（JSON）
- **返回**：`void`
- **说明**：以提供的结果完成当前阶段，触发门控判定

### `inject_memo`
- **参数**：`{ key: string, value: Value }`
- **返回**：`void`
- **说明**：向引擎注入备忘录（创作方向记录）

---

## 6. 一致性与 CDA（3 个命令）

### `check_consistency`
- **参数**：无
- **返回**：`ConsistencyViolation[]`
- **说明**：对当前项目执行增量一致性检查

### `find_affected`
- **参数**：`{ chapter_id: string, changed_entities: string[] }`
- **返回**：`AffectedItem[]`
- **说明**：BFS 查找受影响的章节和叙事元素

### `get_impact_graph`
- **参数**：无
- **返回**：`ImpactGraphStats`
- **说明**：获取影响图统计信息

---

## 7. 记忆系统（3 个命令）

### `build_memory_packet`
- **参数**：`{ chapter_id: string }`
- **返回**：`MemoryPacket { hot, warm, cold, narrative, total_tokens }`
- **说明**：按 8000 token 预算构建四层记忆包

### `get_hot_memory`
- **参数**：无
- **返回**：`{ is_empty: boolean, window_size: number }`
- **说明**：获取热记忆状态

### `get_warm_memory`
- **参数**：无
- **返回**：`{ chapter_count: number }`
- **说明**：获取温记忆状态

---

## 8. LLM 管理（5 个命令）

### `list_providers`
- **参数**：无
- **返回**：`LlmProvider[]`
- **说明**：列出所有 LLM 供应商配置

### `list_models`
- **参数**：无
- **返回**：`LlmModel[]`
- **说明**：列出所有可用模型

### `save_api_key`
- **参数**：`{ provider_id: string, api_key: string }`
- **返回**：`void`
- **说明**：保存供应商 API Key（后端安全存储）

### `test_model`
- **参数**：`{ model_id: string }`
- **返回**：`boolean`
- **说明**：测试模型连通性（发送简单请求验证）

### `route_model`
- **参数**：`{ task_type: string }`
- **返回**：`ModelRouteResult`
- **说明**：根据任务类型路由最佳模型

---

## 9. 灵感生成（1 个命令）

### `generate_inspiration`
- **参数**：`{ context_type: string, context_data: string }`
- **返回**：`InspirationItem[]`
- **说明**：基于上下文通过 LLM 生成灵感建议

---

## 10. 插件/工作流（4 个命令）

### `list_plugins`
- **参数**：无
- **返回**：`PluginConfig[]`
- **说明**：列出所有已注册插件

### `install_plugin`
- **参数**：`{ yaml_content: string }`
- **返回**：`void`
- **说明**：通过 YAML 内容安装插件

### `remove_plugin`
- **参数**：`{ plugin_id: string }`
- **返回**：`void`
- **说明**：移除指定插件

### `toggle_plugin`
- **参数**：`{ plugin_id: string, enabled: boolean }`
- **返回**：`void`
- **说明**：启用或禁用指定插件

---

## 11. 专家库（2 个命令）

### `save_experts`
- **参数**：`{ experts: Expert[] }`
- **返回**：`void`
- **说明**：保存专家列表到独立文件

### `load_experts`
- **参数**：无
- **返回**：`Expert[]`
- **说明**：加载专家列表

---

## 12. HTTP 代理（1 个命令）

### `http_request`
- **参数**：`{ url: string, method: string, headers?: Record<string, string>, body?: string }`
- **返回**：`{ status: number, status_text: string, body: string, ok: boolean }`
- **说明**：通过 Rust 端发 HTTP 请求，绕过 WebView CSP 限制

---

## 数据类型定义

### 前端 TypeScript 类型（`src/types.ts`）

```typescript
interface ProjectMeta {
  project_id: string;
  title: string;
  description: string;
  created_at: string;
  updated_at: string;
  total_chapters: number;
  total_words: number;
}

interface Project {
  project_id: string;
  title: string;
  volumes: Volume[];
  total_chapters: number;
  total_words: number;
}

interface Chapter {
  chapter_id: string;
  volume_id?: string;
  title: string;
  content: string;
  word_count: number;
  version: number;
  status: 'Draft' | 'Reviewing' | 'Reviewed' | 'Polished' | 'Published';
}

interface Character {
  id: string;
  name: string;
  personality_traits: Array<[string, number]>;
  current_mood?: string;
  relationships: Array<{ from: string; to: string; relation_type: string; strength: number }>;
}

interface WorldLayer {
  locations: Array<{ id: string; name: string; description: string }>;
  timeline_events: Array<{ event_id: string; story_time: string; description: string }>;
  setting_rules: Array<{ rule_id: string; title: string; description: string }>;
}

interface StyleMetrics {
  avg_sentence_length: number;
  vocabulary_richness: number;
  dialogue_ratio: number;
  pace_score: number;
  ai_pattern_score: number;
}

interface ConsistencyViolation {
  violation_id: string;
  entity_id: string;
  entity_type: string;
  chapter_a: number;
  chapter_b: number;
  description: string;
  severity: 'Error' | 'Warning' | 'Info';
}

interface LlmProvider {
  provider_id: string;
  name: string;
  display_name: string;
  api_base: string;
  requires_api_key: boolean;
}

interface LlmModel {
  model_id: string;
  provider_id: string;
  display_name: string;
  max_tokens: number;
  supports_tools: boolean;
  cost_per_1k_tokens: number;
  avg_quality_score: number;
  avg_latency_ms: number;
  is_available: boolean;
  api_key_configured: boolean;
}

interface PluginConfig {
  plugin_id: string;
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  stages: PluginStage[];
}

interface HarnessStatus {
  current_stage: string;
  stage_status: string;
  memo: Record<string, unknown>;
  retry_count: number;
}

interface InspirationItem {
  title: string;
  content: string;
}

interface CoreConcept {
  high_concept: string;
  premise: string;
  protagonist_hint: string;
  tone: string;
  central_conflict: string;
  inspiration: string;
}

interface ProjectSettings {
  target_chapters: number;
  target_words: number;
  chapter_target_words: number;
  target_volumes: number;
  genre: string;
}

interface SproutData {
  idea_description: string;
  agents: AgentConfig[];
}

interface AgentConfig {
  id: string;
  name: string;
  model: string;
  prompt: string;
  perspective: string;
  enabled: boolean;
}

interface Expert {
  id: string;
  name: string;
  description: string;
  source_persona: string;
  model_id: string;
  perspective: string;
  default_prompt: string;
  created_at: string;
  skill_path: string | null;
  skill_summary: string | null;
}
```
