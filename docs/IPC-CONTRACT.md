# PenSoul IPC 契约（全量）

> 最后更新：2026-08-01 · 版本：v3 · 对齐 2026-08-01 代码（`crates/pensoul-app/src/main.rs` invoke_handler + `src/ipc.ts`）
> 维护规则：后端新增/修改命令时，必须同步本文件（见 `agent.md` 与 `docs/DEVELOPMENT.md`）

## 一、总览

- 传输：Tauri 2 `invoke`（前端 `src/ipc.ts` 是唯一入口封装）。
- 命名：后端命令 `snake_case`，前端封装 `camelCase`，转换在 `ipc.ts` 收敛。
- 错误：命令返回 `Result<T, String>`，前端统一把错误字符串抛给调用方。
- 事件：4 种后端推送事件（见第四节），前端 `listen` 消费，切页后通过状态命令重放。
- 数据：项目本体（NovelOntology）整体持久化在 `pensoul-project.json`；专家库为全局配置 `experts.json`；API 密钥在 `data/_config/api-keys.json`（不进 IPC 返回值明文，仅 `load_api_keys` 例外——只回前端内存）。

## 二、命令清单（按模块）

### 2.1 项目管理（commands/project.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `create_project` | `title: String` | `String`（project_id） | 新建项目并返回 ID |
| `list_projects` | — | `Vec<ProjectMeta>` | 扫描 base_dir 子目录 |
| `get_project` | `project_id: String` | `ProjectInfo` | 标题/章数/总字数/卷数摘要 |
| `update_project` | `project_id, title, description` | `()` | 读取→修改→回写 |
| `delete_project` | `project_id: String` | `()` | 删除项目目录 |
| `open_project` | `project_id: String` | `()` | 切换活动项目（加载+迁移+重建派生状态） |
| `save_project` | — | `()` | 手动整体保存活动项目 |

### 2.2 章节与卷（commands/chapter.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `get_chapter` | `chapter_id: String` | `Chapter`（JSON） | 按字符串主键查章节 |
| `list_chapters` | — | `Vec<Chapter>` | 全部章节（含正文） |
| `save_chapter` | `chapter_id, content, expected_version: i32` | `i32`（新版本号） | 乐观锁保存正文；首次保存自动从本体恢复版本号；成功后增量更新派生状态；版本冲突返回错误 |
| `upsert_chapter` | `chapter_id, volume_id, title, content, summary, status` | `()` | 新建/插入式更新（大纲层章节走这里，梗概与正文分离） |
| `save_volumes` | `volumes: Vec<{volume_id,title,summary?}>` | `()` | 整体保存卷 |
| `get_volumes` | — | `Vec<Volume>` | 读取卷 |
| `delete_chapter` | `chapter_id: String` | `()` | 删除章节 |
| `delete_volume` | `volume_id: String` | `()` | 删除卷 |

### 2.3 情节脉络与细纲（commands/outline.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `list_outline_arcs` | — | `Vec<OutlineArc>` | 全部脉络节点 |
| `save_outline_arcs` | `arcs: Vec<OutlineArc>` | `()` | 整体保存（新建/编辑/删除） |
| `expand_outline_arc` | `arc_id, model?, batch?=20, skill_cards?` | `ExpandResult{created, from, to, arc_done}` | 展开下一批细纲（章号后端分配，不信任模型）；模型与技法卡缺省时按「项目覆盖 → 模板绑定」解析（与造化工坊同一套规则） |

### 2.4 Harness 引擎（commands/harness.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `start_harness_stage` | — | `String`（stage_id JSON） | 启动当前阶段 |
| `complete_harness_stage` | `result: Value` | `()` | 提交阶段结果，触发门控判定 |
| `approve_harness_stage` | `stage_name: String` | `()` | 人工批准 Manual 门控（带外通道，防 AI 自我批准） |
| `inject_memo` | `key, value` | `()` | 注入滚动备忘录 KV |
| `get_harness_status` | — | `HarnessStatus` | 引擎状态查询 |

> 注意：新功能请走 2.6 章节管线；此组命令为引擎层通用接口。

### 2.5 旧模拟器执行（commands/harness_exec.rs，遗留）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `execute_harness_step` | `stage_name, project_context, stage_prompt` | `HarnessStepResult{stage_name, thinking, output}` | 单次 LLM 调用，无门控/无落库。**遗留兼容入口，新功能勿用** |

### 2.6 章节连写管线（commands/pipeline.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `run_chapter_pipeline` | `chapter_ids?, writing_model?, review_model?, writing_cards?, review_cards?` | `Value` | 长跑命令，进度经 `harness-event` 推送；缺省选章=有梗概且正文为空按 chapter_no 升序 |
| `pause_pipeline` | — | `()` | 暂停（阶段边界停住）；未运行报错 |
| `resume_pipeline` | — | `()` | 继续 |
| `stop_pipeline` | — | `()` | 立即中断 LLM（select!）；已落库章节保留 |
| `get_pipeline_state` | — | `PipelineState` | `{running, paused, current_chapter, events, writing_model, review_model}`，供切页重放 |

### 2.7 记忆系统（commands/memory.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `build_memory_packet` | `chapter_id, mode?` | `MemoryPacket` | 按编辑模式（drafting/revising/reviewing）构建四层记忆包 |
| `get_hot_memory` | — | `{is_empty, window_size}` | 热记忆概况 |
| `get_warm_memory` | — | `{...}` | 温记忆概况 |

### 2.8 影响分析 CDA（commands/cda.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `find_affected_chapters` | `chapter_id: String`（u32 数字章号）, `changed_entities: Vec<String>` | `Vec<ImpactItem>` | BFS 反向传播（深度上限 5），Direct/Indirect/Cascading 分级 |
| `get_impact_graph` | — | `Value`（统计） | 影响图节点/边统计 |

### 2.9 一致性（commands/consistency.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `check_consistency` | — | `Vec<ConsistencyViolation>` | 全书检查（5 条规则） |

### 2.10 世界观/人物志/文风/优化

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `get_world` / `save_world` | — / `world` | `WorldLayer` / `()` | 世界观读写 |
| `get_characters` / `save_characters` | — / `characters` | `CharacterLayer` / `()` | 人物志读写 |
| `get_style_metrics` | — | `StyleMetrics` | 文风指标 |
| `optimize_content` | `content_type: "world"\|"character"`, `content_json: String`, `model_id?` | `String`（同结构优化后 JSON） | 只优化不新增，保持条目数与含义 |

### 2.11 LLM 设置（commands/llm.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `list_providers` | — | `Vec<LlmProvider>` | 供应商列表 |
| `save_providers` | `providers` | `()` | 保存供应商 |
| `list_models` | — | `Vec<LlmModel>` | 模型列表 |
| `save_models` | `models` | `()` | 保存模型 |
| `save_api_key` | `provider_id, api_key` | `()` | 保存密钥到 `data/_config/api-keys.json` |
| `load_api_keys` | — | `Record<provider_id, key>` | 仅前端内存使用，不得落文档/日志 |

### 2.12 概念讨论（commands/discussion.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `discuss_concept` | `idea_description, settings_context, agents: Vec<AgentConfig>` | `DiscussionOutput` | 长跑命令：立论→交锋→成果；进度经 `discussion-event` 推送；Agent 可带 `skill_path`（专家库技能） |
| `get_discussion_state` | — | `DiscussionState` | 讨论控制面快照（运行旗标 + 事件缓冲重放） |

### 2.13 专家蒸馏与专家库（commands/expert_distill.rs / experts.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `distill_expert` | `persona: String, model?` | `Expert` | 长跑命令，进度经 `distill-phase` 推送；产物 `Experts/<名字>-expert/` |
| `save_experts` | `experts: Vec<Expert>` | `()` | 保存专家列表（全局 `experts.json`） |
| `load_experts` | — | `Vec<Expert>` | 读取专家列表 |
| `scan_nuwa_skills` | — | `Vec<Expert>` | 扫描工作区技能（nuwa） |
| `scan_experts_folder` | `path: String` | `Vec<Expert>` | 扫描指定目录下的技能 |
| `delete_expert_skill` | `skill_path: String` | `()` | 删除技能目录（多重安全校验：SKILL.md 文件名 + `-expert`/`-perspective` 后缀 + 受信任根目录，防路径逃逸） |
| `get_experts_folder` | — | `String`（绝对路径） | 查询 Experts 根目录 |

### 2.14 书籍蒸馏（commands/book_distill.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `distill_book` | `title, author?, file_path?, sample_text?, dimensions?, model?` | `BookPackage` | 长跑命令，进度经 `book-distill-phase` 推送；产物 `WritingCard/<书名>-book/`；维度 slug：style/structure/character/tension/genre |
| `list_book_packages` | — | `Vec<BookPackage>` | 列出技能包 |
| `delete_book_package` | `package_dir: String` | `()` | 删除技能包 |

### 2.15 工作流与项目设定（commands/settings.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `save_settings` / `load_settings` | `settings` / — | `()` / `ProjectSettings` | 创作设定读写 |
| `save_concept` / `load_concept` | `concept` / — | `()` / `CoreConcept` | 核心概念读写 |
| `save_sprout` / `load_sprout` | `sprout` / — | `()` / `SproutData` | 萌芽数据读写；保存时若 `last_discussion` 为 None 保留后端旧结果（防旧副本覆盖） |
| `save_workflow_skills` / `load_workflow_skills` | `config: Value\|null` / — | `()` / `Value\|null` | **遗留**：旧版工作流技能卡绑定（透明存储），已由 `workflow_ref` 取代，仅做迁移兜底 |

### 2.16 全局工作流模板与项目引用（commands/workflow_templates.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `list_workflow_templates` | — | `Vec<WorkflowTemplate>` | 列出全部全局模板（每次重新加载磁盘，跨页面一致） |
| `save_workflow_templates` | `templates: Vec<WorkflowTemplate>` | `()` | 整体保存模板库；ID 非空唯一，内置模板缺失自动补回、`builtin` 标志不可篡改 |
| `reset_workflow_templates` | — | `Vec<WorkflowTemplate>` | 恢复内置模板到出厂状态（用户自定义模板保留） |
| `save_workflow_ref` | `config: Value\|null` | `()` | 保存项目工作流引用（模板 ID + 版本 + 项目覆盖），随项目文件持久化 |
| `load_workflow_ref` | — | `Value\|null` | 读取项目工作流引用（未配置过返回 null） |

`WorkflowTemplate` 结构：`template_id / name / version / genre / description / builtin / enabled / review_pass_score / stages / bindings`。
`WorkflowRef` 结构：`template_id? / template_version? / overrides`（overrides 形如 `{outline_expand: {model, cards}, chapter_writing: {...}, review: {...}}`）。
模板本体存 `data/workflows/templates.json`（全局共享）；项目只存引用与差异覆盖。

### 2.17 插件（commands/plugin.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `list_plugins` | — | `Vec<PluginConfig>` | 插件列表 |
| `install_plugin` | `yaml_content: String` | `()` | YAML 校验后安装 |
| `remove_plugin` | `plugin_id: String` | `()` | 移除插件 |
| `toggle_plugin` | `plugin_id, enabled: bool` | `()` | 启用/停用 |

### 2.18 杂项（commands/http.rs）

| 命令 | 参数 | 返回 | 说明 |
|---|---|---|---|
| `http_request` | `request: HttpRequest` | `HttpResponse` | 通用 HTTP 转发（前端跨域兜底） |

## 三、关键数据结构（前后端共享，见 `src/types.ts`）

### Chapter

```ts
interface Chapter {
  chapter_id: string;      // 字符串主键 ch-<ts>-<rand>
  chapter_no: number;      // 顺序语义唯一入口（记忆/影响图/一致性索引）
  volume_id: string;
  title: string;
  summary: string;         // 细纲（大纲层信息，与正文分离）
  content: string;
  word_count: number;
  status: string;
  version: number;         // 并发版本（乐观锁）
}
```

### OutlineArc（情节脉络）

```ts
interface OutlineArc {
  arc_id: string;
  title: string;
  plot: string;            // 剧情规划
  chapter_start: number;
  chapter_end: number;
  expanded_until: number;  // 已展开到哪一章（<=0 表示未展开）
}
```

### DiscussionEvent / PipelineEvent（事件类型见第四节）

### WorkflowSkillConfig（工作流技能卡绑定）

```ts
interface StageSkillConfig {
  model?: string | null;
  cards: string[];         // 技法卡 SKILL.md 路径
}
interface WorkflowSkillConfig {
  outline_expand?: StageSkillConfig;
  chapter_writing?: StageSkillConfig;
  review?: StageSkillConfig;
}
```

## 四、事件协议

所有事件由后端 `app.emit` 推送，前端 `listen` 消费；事件结构均为 serde Serialize 的扁平 JSON。

### 4.1 `harness-event`（章节管线）

```ts
interface PipelineEvent {
  seq: number;             // 单调序号（0=未入缓冲，emit 时覆写）
  chapter_id: string;
  chapter_title: string;
  stage: string;           // chapter_writing | chapter_review | state_injection
  kind: string;            // chapter_start | stage_start | llm_output | review_report
                           // | gate | effect | chapter_done | chapter_failed
                           // | paused | resumed | pipeline_done
  status: string;
  content: string;
  score?: number;          // 仅 gate/review_report 带评分
  attempt: number;
}
```

前端订阅：`HarnessConsole.tsx`；切页恢复：`get_pipeline_state()` 的 `events` 字段。

### 4.2 `discussion-event`（概念讨论）

```ts
interface DiscussionEvent {
  agent_id: string;
  agent_name: string;
  round: number;           // 1=立论 2=交锋 3=成果
  status: string;          // running | done | error
  content: string;
}
```

前端订阅：`ConceptView.tsx`；恢复：`get_discussion_state()`。

### 4.3 `distill-phase`（专家蒸馏）

```ts
interface PhaseEvent {
  phase: string;           // 六维调研阶段名
  status: string;
  message: string;
  detail: string;
}
```

前端订阅：`ExpertLibraryView.tsx`。

### 4.4 `book-distill-phase`（书籍蒸馏）

同上 `PhaseEvent` 结构，前端订阅：`BookDistillPanel.tsx`。

## 五、约定与坑

1. **前端 ID 与后端数字章号**：`find_affected_chapters` 的 `chapter_id` 参数目前是 u32 数字章号（与其余命令的字符串 ID 不一致，是历史遗留）；新增代码建议统一走字符串主键 + 后端内部解析 chapter_no。
2. **事件缓冲**：管线 200 条、讨论 100 条，环形丢弃最旧；重放只保证最近进度。
3. **密钥**：`load_api_keys` 的返回值只能进前端内存；任何日志/文档不得打印。
4. **新增命令三步**：后端命令 + main.rs 注册 → `src/ipc.ts` 封装 + `types.ts` 类型 → 更新本文件。
