# PenSoul 架构总览

> 最后更新：2026-08-01 · 版本：v2（对齐 2026-08-01 代码状态）
> 维护规则：见仓库根目录 `agent.md` 与 `docs/PROGRESS.md`

## 一、项目定位与核心理念

PenSoul 是一个 AI 长篇小说创作平台。核心哲学一句话：

> **让引擎管流程，让模型管创作。AI 无权跳步。**

含义：

- **流程是引擎的职责**：章节写作、审查、回灌的编排由 Rust 侧硬编码的 Pipeline 执行，LLM 只负责产出内容片段，无权决定流程是否推进。
- **创作是模型的职责**：正文、细纲、讨论发言、蒸馏成果等一切"写"的动作交给 LLM；是否放行（门控）、何时重试、哪些章节受影响等"判"的动作交给引擎。
- **人机协作**：所有 AI 产物默认是"建议制"，用户确认后才写入正典数据；用户手改章节后系统只给修订建议，不自动改写。

## 二、技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2（Rust 后端 + 系统 WebView） |
| 后端 | Rust 2021 edition，Cargo workspace（11 个 crate） |
| 前端 | React + TypeScript + Vite，无 UI 框架，手写 CSS 变量主题 |
| 编辑器 | TipTap（章节正文编辑） |
| 序列化 | serde + serde_json |
| 校验 | blake3（WAL / 并发校验和） |
| 异步 | tokio（长跑命令、事件通知） |
| 持久化 | 项目内 JSON 文件（`data/projects/<id>/`），原子写（tmp + rename） |
| 文档 | Markdown（docs/ + skills/ 方法论） |

## 三、Workspace 结构与 crate 职责

```
Pensoul/
├── crates/
│   ├── pensoul-core        数据本体：四层世界观 + 章节/卷/设定/大纲/工作流配置
│   ├── pensoul-harness     创作流程引擎：阶段状态机、门控、WAL、崩溃恢复、工具白名单、滚动备忘录
│   ├── pensoul-cda         变更影响分析：影响图构建 + BFS 反向传播 + 严重度分级
│   ├── pensoul-memory      四层记忆：热/温/冷/叙事，8 步更新管道 + 预算分配
│   ├── pensoul-agent       预置 Agent 定义与双通道通信协议（signal/report）
│   ├── pensoul-concurrency 并发控制：版本管理、操作日志、blake3 校验和、冲突解决
│   ├── pensoul-plugin      YAML 插件校验与解析
│   ├── pensoul-consistency 增量一致性检查：5 条规则 + 实体状态管理
│   ├── pensoul-import      文本导入（章节检测/中文数字）、导出、备份恢复
│   ├── pensoul-llm         模型路由（TaskType/冷却/故障转移）—— 目前应用层未使用，见遗留说明
│   └── pensoul-app         Tauri 应用层：命令、Pipeline 编排、集成层、LLM 调用适配
├── src/                    前端 React 应用
├── skills/                 蒸馏方法论技能（pensoul-skill-Experts / pensoul-skill-Books）
├── Experts/                专家蒸馏产物（每个 <名字>-expert/ 一个技能包）
├── data/                   运行时数据（API 密钥、项目），已 gitignore
└── tools/                  开发辅助脚本
```

各 crate 职责详述：

### pensoul-core（本体层）

`NovelOntology` 是唯一的正典数据结构，四层本体：

- **world**（WorldLayer）：世界观设定
- **characters**（CharacterLayer）：人物志
- **narrative**（NarrativeLayer）：情节脉络节点（`outline_arcs`）+ 章节实体（`chapters`）+ 卷（`volumes`）
- **aesthetic**（AestheticLayer）：文风与美学设定

另含项目级设定：`settings`（ProjectSettings：类型/目标章数/每章字数）、`core_concept`（高概念/前提/主角/基调/核心冲突）、`sprout`（萌芽想法与讨论成果）、`workflow_ref`（项目对全局工作流模板的引用 + 项目级覆盖；旧字段 `workflow_skills` 仅做迁移兜底）。

工作流模板（`pensoul-core::workflow`）：作品库层面定义全局模板（网文/传统/科幻/通用等），存 `data/workflows/templates.json`，AppState 持有 `workflow_templates` 缓存；项目内只存 `workflow_ref`（引用的模板 ID/版本 + 覆盖），造化工坊启动时按「显式参数 → 项目覆盖 → 模板绑定 → 自动选模型」解析实际执行配置。

关键设计决策：

1. **`Chapter.chapter_no` 是顺序语义的唯一入口**。前端字符串 ID（`ch-<时间戳>-<随机>`）仍是主键，用于记忆/影响图/一致性索引时一律转成 `chapter_no` 解析；非数字 ID 显式跳过而不是静默映射为第 0 章。
2. **`outline_arcs` 与 `chapters` 分离**：脉络节点覆盖章节范围（如第 1-200 章）但不产正文；细纲展开（`expand_outline_arc`）按范围分批生成真实章节（正文留空），造化工坊只对真实章节写作。
3. **`SproutData` 容错反序列化**：LLM 产物形态多变，`#[serde(default)]` + 字段级容错保证旧项目与不完整数据可加载。
4. **`migrate_arc_chapters` 迁移**：历史版本把脉络节点误存成"伪章节"，加载时识别并还原为脉络节点，正文不为空的保留为真实章节。

### pensoul-harness（引擎层）

创作流程引擎，独立于具体 LLM：

- **`HarnessEngine`**：阶段注册（register_stage）、启动/完成状态机，支持 Auto / Manual / Conditional 三种门控。
- **`GateEvaluator`**：Conditional 门控默认 `score >= 80` 放行；Manual 门控只认带外人工批准（防 AI 伪造自己的评分）。
- **`CrashRecovery`**：WAL（`.harness/` 目录）记录阶段事件，blake3 校验 + 状态重放，崩溃后可从断点恢复。
- **`ToolWhitelist`**：deny 优先于 allow 的工具白名单。
- **`RollingMemo`**：KV 注入，`to_context_string()` 输出 JSON 进 prompt。

### pensoul-cda（影响分析）

- `build_impact_graph`：章节/角色/伏笔节点 + References 边 + 内容包含判定（章节文本里提到实体名即连边）。
- `bfs_find_affected`：从变更实体反向 BFS 传播，深度 0 直接命中 → Direct，1-2 跳 → Indirect，更深 → Cascading；按章节号 + 严重度排序返回建议。

### pensoul-memory（记忆层）

四层记忆：

| 层 | 内容 | 预算 |
|---|---|---|
| 热记忆 | 热窗口内（默认 2 章）完整正文 | 按编辑模式比例 |
| 温记忆 | 卷摘要、活跃伏笔、角色状态 | 同上 |
| 冷记忆 | 热窗口外的历史章节摘要 | 同上 |
| 叙事记忆 | 叙事细节（带分类与重要度评分） | 同上 |

`MemoryPipeline::update` 是 8 步更新管道：提取摘要 → 提取角色状态 → 提取关键事件 → 提取叙事细节 → 更新热记忆 → 更新温记忆 → 冷记忆同步 → 更新叙事记忆。总预算默认 8000 token，按编辑模式（草稿/修订/审阅）比例分配。

### pensoul-agent（Agent 定义）

- 6 个预置 Agent：一致性审查员、文风校准师、伏笔追踪师、对话打磨师、大纲规划师、世界观构建师。
- 双通道协议：**signal**（结构化 JSON，仅引擎可见）与 **report**（自然语言，仅用户可见）分离。

### pensoul-concurrency（并发控制）

- `ConcurrencyController`：章节版本号 + blake3 内容校验和 + 操作日志（UserEdit/AiGenerate/AiRevision/SystemImport，Pending/Applied/Conflict/Rejected）。
- `ConflictResolver` 与 `VersionManager` 处理并发编辑冲突与版本推进。

### pensoul-consistency（一致性）

`IncrementalChecker` 支持增量检查（只查受影响范围），5 条预置规则：

| 规则 | 检查内容 |
|---|---|
| 角色状态一致性 | 角色属性跨章是否冲突 |
| 事件连续性 | 事件前后是否矛盾/缺环 |
| 伏笔跟踪完整性 | 伏笔是否埋下未收 / 未埋先收 |
| 世界观设定一致性 | 设定跨章是否矛盾 |
| 时间线一致性 | 时间顺序与跨度是否自洽 |

### pensoul-import（导入导出）

- `TextImporter` + `ChapterDetector`：从整本文本按章节标题（阿拉伯/中文数字）切分章节。
- `cn_number::parse_cn_number`：中文数字解析（"二十三"=23、"一百"=100）。
- `exporter`：整书/单章导出 TXT。
- `BackupManager`：项目备份/恢复（`data/backups/`）。

### pensoul-app（应用层）

应用层的核心资产都在这里，是 docs 重点：

- `pipeline/`：章节连写管线编排器（详见下文）。
- `integration.rs`：派生状态集成层（记忆/影响图/一致性/并发版本）。
- `state.rs`：`AppState` 全局状态与持久化。
- `llm_profile.rs`：模型档案自动适配。
- `commands/`：全部 IPC 命令（见 `docs/IPC-CONTRACT.md`）。

## 四、章节连写管线（pipeline）

位置：`crates/pensoul-app/src/pipeline/`，唯一编排入口，执行循环在 Rust 侧，前端只渲染事件 + 发控制指令。

### 4.1 三阶段模板（Rust 硬编码）

| 阶段 | 门控 | 说明 |
|---|---|---|
| `chapter_writing` | auto | 写作：组装上下文 → 调写作模型 → 产出正文 |
| `chapter_review` | conditional | 审查：异模型（默认与写作模型不同）审查，`on_fail=writing`，`max_retries=2` |
| `state_injection` | auto | 回灌：正文落库 + 触发派生状态增量更新 |

### 4.2 选章规则

`chapter_ids` 缺省时自动选择「有梗概且 word_count == 0」的章节，按 `chapter_no` 升序逐个写作。

### 4.3 控制面

- **暂停**：`pause_pipeline` 置 paused 旗标，执行在阶段边界自旋 500ms 等待。
- **停止**：`stop_pipeline` 置 stop 旗标 + `notify.notify_waiters()`，LLM 调用走 `select!` 立即中断（`STOP_ERR` 哨兵），已落库章节保留。
- **重入保护**：`running` 原子旗标防重入；新一轮运行清空事件缓冲与序号。
- **事件缓冲**：环形 200 条，`get_pipeline_state` 可重放，页面切换不丢现场。

### 4.4 执行细节

- 每章内层迭代上限 12（写作失败重试 2 次 + 审查回退重写）。
- 上下文组装（`context.rs`）：核心概念 + 创作设定 + 本章梗概 + 记忆包（热/温/冷/叙事）+ 绑定技法卡（写作/审查环节）+ 前文衔接。
- 事件经 `harness-event` 推送，kind 枚举：
  `chapter_start` / `stage_start` / `llm_output` / `review_report` / `gate` / `effect` / `chapter_done` / `chapter_failed` / `paused` / `resumed` / `pipeline_done`。

## 五、集成层（integration.rs）

所有派生状态统一从本体重建，杜绝手写逻辑绕过：

- `rebuild_derived_state`：项目加载时全量重建 —— 记忆管道、影响图、一致性实体状态、并发版本表。
- `on_chapter_saved`：章节保存后的增量更新 —— 记忆 8 步管道、影响图重建、实体状态 upsert、并发版本推进。
- `entity_states_for_chapter`：从章节文本提取实体状态（角色/世界观/伏笔出现与状态）。

顺序语义统一走 `chapter_no`；`chapters_in_order` 按 `chapter_no` 升序、0 排最后（历史无序号章节兜底）。

## 六、LLM 层

### 6.1 模型档案（llm_profile.rs）

按模型前缀自动适配请求体，避免各家 API 差异：

| 模型前缀 | reasoning | 预算字段 | 采样 |
|---|---|---|---|
| kimi-k3 | 永远思考，reasoning_effort 可调（low/high/max） | `max_completion_tokens` | 固定（Kimi 显式传 temperature 会被拒） |
| kimi-k2.x / moonshot | thinking 可开关 | `max_tokens` | Kimi 系列固定 |
| glm-5.x | thinking 开关 + reasoning_effort | `max_tokens`（上限 65536） | 可调 |
| deepseek-v4 | thinking 开关 + reasoning_effort | `max_tokens` | 思考模式下采样被忽略 |
| 其余 | 无 | `max_tokens` | 可调 |

任务分 Light / Deep 两档：Light（评审、纪要等结构任务）注入低档或关闭思考；Deep 用完整思考。

### 6.2 统一调用（commands/llm_helper.rs）

应用层所有 LLM 调用统一走 `llm_helper`：

- `resolve_provider`：模型 → 供应商 → API Key/Base 解析。
- SSE 流式调用，600s 超时。
- 三类重试策略：瞬断/5xx/429 重试一次；TokenExhausted 预算翻倍重试；4xx 降级请求体（剔除 thinking/reasoning_effort/max_completion_tokens 等扩展参数）重试。
- `ensure_api_keys_loaded`：调用前保证密钥已加载。

## 七、多 Agent 讨论（commands/discussion.rs）

概念讨论三阶段：

1. **立论**：每个 Agent（可来自专家库，加载 SKILL.md 作为系统提示词）独立分析。
2. **交锋**：每个 Agent 完整阅读他人第 1 轮发言，回应/质疑/补强。
3. **成果**：单次综合调用提炼结构化成果（共识总结 + 地点/时间线/设定规则/人物及关系）。

进度经 `discussion-event` 实时推送，同时写入控制面环形缓冲（上限 100），结果持久化到 `sprout.last_discussion`，前端切页后 `get_discussion_state` 重连恢复。

## 八、专家蒸馏与书籍蒸馏

### 专家蒸馏（commands/expert_distill.rs）

加载 `skills/pensoul-skill-Experts/SKILL.md` 方法论（找不到时用内置简版兜底），把人物思维方式提炼为专家技能卡：

- 产物目录：`Experts/<名字>-expert/`（含 SKILL.md + references/research/ 调研存档）。
- 模板：角色规则 / 创作讨论工作流 / 核心心智模型 / 创作决策启发式 / 表达 DNA / 价值观与反模式 / 诚实边界。
- 调研六维度：著作、对话、表达、他者、决策、演变；心智模型三重验证（跨域复现/生成力/排他性）。
- 红线：提炼思维框架而非人物生平，不写身份卡/年表/谱系；矛盾保留而非调和；诚实标注局限。

### 书籍蒸馏（commands/book_distill.rs）

加载 `skills/pensoul-skill-Books/SKILL.md` 方法论，把一本书的写作方法提炼为五维技能卡：

- 产物目录：`WritingCard/<书名>-book/`（style / structure / character / tension / genre 各一张 SKILL.md）。
- 卡片六段：R 手法出处 / I 技法骨架 / A1 书中案例 / A2 适用场景 / E 执行步骤 / B 边界。
- 技法三重验证：跨章复现 / 生成力 / 独特性；每维 1-3 个，宁少勿多。
- 知识蒸馏模式（无样章）必须在 B 段标注"基于模型知识储备，非逐字文本核对"。
- `load_writing_cards`：蒸馏产物可供管线写作/审查与细纲展开注入（按 `applicable_stages` 匹配）。

## 九、两层大纲模型

1. **情节脉络**（`outline_arcs`）：讨论成果的剧情规划，覆盖章节范围，不可写正文。
2. **章节细纲**（`Chapter.summary`）：`expand_outline_arc` 每批展开 20 章（LLM 产出量控制，防截断），生成真实章节实体（正文留空）。

展开时上下文 = 核心概念 + 创作设定 + 节点规划 + 前情衔接（本批起点前最近 2 章梗概）；章号由后端按批次顺序分配，不信任模型编号；章节落入第一个真实卷或 `_default` 卷。

## 十、持久化与数据迁移

- 项目根：`data/projects/<project_id>/`（开发模式 data 在工作区根，见 `AppState::new`）。
- 原子写：tmp 文件 + rename，杜绝半写损坏。
- 加载时迁移：backfill `chapter_no`（按数组顺序）→ `migrate_arc_chapters`（伪章节还原为脉络）→ 全量重建派生状态 → 立即落盘。
- API 密钥：`data/_config/api-keys.json`，不落项目目录、不进 git、不进任何文档。

## 十一、事件流架构

| 事件名 | 来源 | 用途 |
|---|---|---|
| `harness-event` | 章节管线 | 阶段/章节/门控/暂停恢复/完成 |
| `discussion-event` | 概念讨论 | Agent 立论/交锋/成果进度 |
| `distill-phase` | 专家蒸馏 | 六维调研分阶段进度 |
| `book-distill-phase` | 书籍蒸馏 | 整书理解/五维提炼分阶段进度 |

所有事件模式一致：后端驱动 → 实时推送 + 环形缓冲 → 前端切页后通过状态命令重放恢复。

## 十二、前端架构

- `src/ipc.ts`：`invoke` 封装，所有命令的唯一前端入口，camelCase ↔ snake_case 转换在此收敛。
- `src/types.ts`：前后端共享类型（与后端 serde 结构对齐）。
- `src/store.ts`：项目数据状态管理，独立容错保存（各区块分开持久化），`open`（打开项目全量加载）与 `refresh`（重拉派生状态）语义区分。
- 视图：ProjectDashboard（项目仪表盘）/ WorkflowLibraryView（作品库工作流模板库）/ ConceptView（萌芽与讨论）/ WorkflowView（项目工作流：选模板 + 项目覆盖）/ HarnessConsole（造化工坊管线事件控制台）/ OutlineView（情节脉络与细纲）/ WritingView（笔耕）/ WorldView / CharacterView / ConsistencyView / ExpertLibraryView / StyleWorkshop / PluginView / LlmSettingsView / CreationSettings / ProjectManager。

## 十三、数据流总览

```
用户操作（前端视图）
  └─ IPC（src/ipc.ts 封装）
       └─ 后端命令（commands/*）
            ├─ 本体读写（ontology）→ 原子落盘
            ├─ LLM 调用（llm_helper + llm_profile）
            ├─ Pipeline 编排（pipeline/）→ harness-event
            └─ 派生状态（integration.rs）
                 ├─ 记忆（memory）
                 ├─ 影响图（cda）
                 ├─ 一致性（consistency）
                 └─ 并发版本（concurrency）
```

## 十四、遗留与兼容入口（重要）

以下代码仍存在但**不是**当前主流程，文档与开发时勿混淆：

1. **`commands/harness_exec.rs`**：旧模拟器执行入口（单次 LLM、无门控、产出不落库），仅兼容保留，新功能一律走 `pipeline/`。
2. **`pensoul-llm` 的 ModelRouter**：应用层实际直接用 `llm_helper` + `llm_profile`，ModelRouter 的 TaskType/冷却/故障转移未接入应用层。如需启用需在 `llm_helper` 里接入。
3. **`pensoul-agent` 的预置 Agent 与双通道协议**：定义与协议已实现，但概念讨论（discussion.rs）目前直接按前端传入的 Agent 配置执行，预置 Agent 定义未直接驱动讨论流程。
4. **`crates/pensoul-app/src/views/mod.rs`**：旧视图状态枚举，前端实际视图在 `src/views/*.tsx`。

## 十五、性能指标（历史实测）

以下数据来自早期可行性验证（原 `FEASIBILITY-REPORT.md` 归档前的实测结果），仅作规模参考，随代码演进可能变化：

| 测试项 | 结果 |
|---|---|
| 1000 章影响图构建 | 5029 节点 / 0.03ms |
| 500 章记忆包构建 | 平均 < 50ms |
| 200 章影响图构建 | 1020 节点 / 1800 边，构建 < 50ms |
