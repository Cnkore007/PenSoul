# PenSoul 工作流管线设计（V1 对齐版）

> 版本：V1 对齐版 · 2026-08-01（与用户对齐后修订：自动连写为 P0 主流程）
> 依据：`docs/DESIGN-V2.md` 第三部分（Harness 引擎）、第八部分（核心功能设计）+ 当前全部代码核实
> 核心理念：让引擎管流程，让模型管创作。AI 无权跳步。

## 用户流程（对齐确认的终极目标）

```
讨论成果就位（世界观/人物志/大纲梗概）
  → 造化工坊选工作流（写作模型 + 审查模型）
  → 点「开始写作」→ 系统从第 1 章开始自动连写
  → 每章闭环：写作(auto) → 审查(conditional, 异模型) → 回灌(auto)
  → 正文逐章落库，笔耕页面自动出现（只显示 word_count > 0 的章节）
  → 用户随时可暂停 / 继续 / 停止；进度已落库，重启不丢
  → 用户手改某章后 → 系统标记受影响章节 + 给出修订建议（建议制，不自动改写）
```

---

## 一、代码资产盘点（已核实，可直接复用）

| 资产 | 位置 | 现状 |
|------|------|------|
| 阶段状态机 | `pensoul-harness/engine.rs` | 完整：`register_stage / set_start_stage / start_stage / complete_stage(result)`，门控判定、回退、重试、熔断都在 |
| 门控三模式 | `pensoul-harness/gate.rs` | 完整：Auto / Manual / Conditional（默认 `score >= 80`） |
| 滚动备忘录 | `pensoul-harness/memo.rs` | 完整：KV 注入、`to_context_string()` JSON 输出 |
| WAL + 崩溃恢复 | `pensoul-harness/wal.rs / recovery.rs` | 完整：blake3 校验、`.harness/` 目录、状态重放 |
| 四层记忆 | `pensoul-memory` | 完整：`MemoryPipeline.update(chapter_no, text)`、`build_packet()` → 热记忆前文 + 温记忆（卷摘要/活跃伏笔/角色状态）+ 叙事细节 |
| 一致性检查 | `pensoul-consistency` + `integration.rs` | 完整：实体状态随章节保存自动 upsert |
| 影响图 | `pensoul-cda` + `integration.rs` | 完整：章节保存后自动重建，BFS 查受影响章节 |
| 模型路由 | `pensoul-llm/router.rs` | 完整：`TaskType`、冷却 + 故障转移 |
| 事件流模式 | `commands/discussion.rs` | 已验证：后端驱动、`discussion-event` 实时推送、前端纯渲染 |

**结论：零件齐全，缺的是把它们串起来的编排器。**

## 二、已发现的断点（按严重度排序）

### 断点 1（致命）：前端章节 ID 在记忆系统中是隐形的

记忆管道、影响图、一致性实体状态、`build_memory_packet` 全部 keyed by `chapter_id.as_i64()`，而前端章节 ID 是 `ch-<时间戳>-<随机>` 非数字字符串——**前端创建的章节全部跳过四层记忆和影响图**。

**修复方案（P0 前置）**：`Chapter` 新增 `chapter_no: i64`（`#[serde(default)]` 向前兼容）：
- 创建时分配 `max(existing) + 1`；旧项目加载时按数组顺序 backfill
- 记忆管道 / 影响图 / 一致性 / `build_memory_packet` 全部改用 `chapter_no` 索引
- 字符串 `chapter_id` 仍是主键不变，序号只用于顺序语义

### 断点 2：造化工坊是前端模拟器

`HarnessConsole.tsx` 按模板顺序循环调 `execute_harness_step`（单次 LLM、2048 tokens、无上下文注入、无门控判定、产出不落库）；`AppState.harness` 里没有注册任何 Stage。门控只是 UI 暂停。**本轮整体重写为事件流渲染器。**

### 断点 3：委托执行无实现路径

`RunnerType::Delegated` 只是标记。本轮由编排器直接按「写作模型 / 审查模型」两个参数调 `llm_helper`，实现「学生不自己判卷」。

### 断点 4：上下文组装不存在

`MemoryPacket` 能构建了，但没有任何代码把「备忘录 + 本章梗概 + 世界观 + 人物 + 记忆包」组装成写作 prompt。本轮新增 `ContextAssembler`。

### 断点 5：门控没有信号源

`GateEvaluator` 等着消费 `{score}` JSON，但没有审查环节产出它。本轮由审查阶段的 OutputParser 产出。

---

## 三、总体架构：Pipeline 编排器（P0 主流程 = 自动连写）

新增 `crates/pensoul-app/src/pipeline/` 模块，作为唯一编排入口。**执行循环在 Rust 侧，前端只渲染事件 + 发控制指令。**

```
┌──────────────────────── 造化工坊（前端）────────────────────────┐
│  模型选择（写作/审查）→ run_chapter_pipeline                    │
│  暂停 / 继续 / 停止 → pause/resume/stop_pipeline                │
│  渲染 harness-event 事件流（按章节分组的阶段卡片）              │
└──────────────────────────────┬──────────────────────────────────┘
                               │ IPC
┌──────────────────────────────▼──────────────────────────────────┐
│  ChapterPipeline（pensoul-app/src/pipeline/）                   │
│                                                                 │
│  启动：PipelineControl{running, paused, stop, notify}           │
│       注册三阶段模板（Rust 硬编码）→ engine.register_stage      │
│       inject_memo（核心想法/创作设定/目标字数）                 │
│                                                                 │
│  for chapter in 待写章节（按 chapter_no 升序）:                 │
│    loop:                                                        │
│      ① 检查控制旗标：stop → 中断；paused → 阶段边界自旋等待     │
│      ② inst = engine.start_stage()           # WAL StageStart   │
│      ③ ctx  = ContextAssembler.build(stage, chapter)            │
│      ④ out  = Executor.call_llm(model, ctx)  # select! 可中断   │
│      ⑤ signal = OutputParser.parse(stage, out)                  │
│      ⑥ EffectApplier.apply(stage, out)  # 落库/回灌/发事件      │
│      ⑦ engine.complete_stage(signal)  # 门控：推进/回退/熔断    │
│    章节完成（stages_status 全 Completed）→ 下一章               │
└──────────────────────────────────────────────────────────────────┘
```

### 3.1 三阶段模板（P0 用 Rust 硬编码，不走 YAML）

| 阶段 | gate | runner | on_fail | max_retries | 说明 |
|------|------|--------|---------|-------------|------|
| `chapter_writing` | Auto | Local | — | — | 生成正文，落库后 status=Reviewing |
| `chapter_review` | Conditional(score>=80) | Delegated(审查模型) | chapter_writing | 2 | 双通道产出 score+issues；不过则带 issues 回写作重写；两次不过熔断交人工 |
| `state_injection` | Auto | Local | — | — | 产出 chapter_brief 回灌 RollingMemo（滚动保留最近 3 章纪要） |

状态流转：`Draft → Reviewing →（审查过）→ Reviewed`，进入下一章。

YAML 插件转换器（`PluginStage → Stage`）移至 P1，届时三阶段模板才外置为可配文件。

### 3.2 ContextAssembler —— 工具白名单的落地形态

不做 function calling。注入面就是能力面，「写作阶段不能改设定」因为没有写设定的通道而天然成立。

| 阶段 | 注入内容 |
|------|---------|
| 写作 | 备忘录（核心想法/创作设定/近 3 章纪要）+ 本章标题与梗概 + 世界观压缩（地点/规则各一行，cap ~2000 字）+ 人物（名+特质+心境+关系）+ 记忆包（热记忆前 2 章正文 + 温记忆）+ 目标字数；重写时追加审查 issues |
| 审查 | 本章正文 + 前章纪要 + 人物快照 + 设定规则 |
| 回灌 | 本章正文（截断） |

### 3.3 OutputParser —— 双通道的落地形态

```
===SIGNAL_BEGIN===  {"score": 85, "issues": [...]}
===SIGNAL_END===
===REPORT_BEGIN===   本章一致性评分 85，发现 1 处潜在矛盾……
===REPORT_END===
```

- SIGNAL → `engine.complete_stage(signal)`（引擎判门控）
- REPORT → `harness-event` 推前端（用户看）
- 解析失败 = 阶段执行失败，计入重试
- 写作阶段输出 = 全文正文（剥掉 ```` ``` ```` 围栏）；回灌阶段输出 = JSON `{chapter_brief}`，解析失败用前 150 字兜底

### 3.4 效果落库

- 写作完成 → 直接写 ontology（content / version+1 / word_count 重算 / status=Reviewing）→ `state.save()` → `integration::on_chapter_saved` 增量更新记忆/影响图/一致性
- 审查通过 → status=Reviewed
- 回灌 → `engine.inject_memo("recent_chapters", ...)` 滚动保留最近 3 章纪要

### 3.5 控制面：暂停 / 继续 / 停止 / 续写

- `PipelineControl`（挂 AppState）：`running / paused / stop: AtomicBool` + `tokio::sync::Notify`
- **暂停**：在阶段边界自旋等待（500ms 轮询旗标），不中断进行中的 LLM 调用
- **停止**：LLM 调用用 `tokio::select!` 同时监听请求 future 和 notify，实现立即中断；已落库的章节进度天然保留
- **续写**：再次点「开始写作」即自然续写——选章逻辑只挑 `word_count == 0` 的章节，已写章节自动跳过
- **防重入**：`running == true` 时 `run_chapter_pipeline` 直接报错

### 3.6 选章逻辑

`chapter_ids: Option<Vec<String>>`；缺省 = 所有 `summary 非空 && word_count == 0` 的章节，按 `chapter_no` 升序。单章细写 = 传单个 id 的特例，不再是独立功能。

### 3.7 事件协议（`harness-event`）

```json
{ "chapter_id": "ch-1", "chapter_title": "第一章 …", "stage": "chapter_writing",
  "kind": "stage_start|llm_output|review_report|gate|effect|chapter_done|chapter_failed|paused|resumed|pipeline_done",
  "status": "running|done|error", "content": "用户可见文本", "score": 85, "attempt": 1 }
```

### 3.8 修改联动（建议制，P1，本轮不做）

用户手改章节后：CDA 影响图 BFS 找出受影响章节 → 标记「可能受影响」+ 生成修订建议列表 → 用户在笔耕/大纲查看并决定是否重写。**不自动改写任何正文**。依赖本章落库链路（`on_chapter_saved` 已重建影响图），连写跑通后单独做。

---

## 四、IPC 接口

| 命令 | 参数 | 说明 |
|------|------|------|
| `run_chapter_pipeline` | `chapter_ids?, writing_model?, review_model?` | async 长跑；模型缺省取第一个可用；审查模型尽量与写作不同 |
| `pause_pipeline` / `resume_pipeline` / `stop_pipeline` | — | 改旗标 + notify |
| `get_pipeline_state` | — | `{running, paused, current_chapter}` |

## 五、造化工坊 UI（整体重写 HarnessConsole.tsx）

- **顶部**：工作流状态 + 写作模型 / 审查模型两个下拉（`listModels`，默认「自动」）
- **章节队列**：列出有梗概的章节（已完成/待写状态图标）
- **控制按钮**：开始写作 / 暂停 / 继续 / 停止
- **事件流**：复用 DiscussionPanel 模式——按章节分组的阶段卡片、实时 LLM 输出、审查报告卡、门控标记（✓放行/↺重写/✗熔断）
- **空态**：提示先去大纲建章节梗概

---

## 六、分期路线图

| 期 | 内容 |
|----|------|
| **P0（本轮）** | chapter_no 修复；三阶段模板（Rust 硬编码）；自动连写编排器；暂停/停止/续写；harness-event 事件流；造化工坊 UI 重写 |
| **P1** | 修改联动（建议制：影响图标受影响章节+修订建议）；YAML 模板插件化（PluginStage→Stage 转换器，WorkflowView 迁移）；阶段级模型配置 |
| **P2** | 伏笔全生命周期（写作注入活跃伏笔/回灌推进状态/超期告警）；反 AI 规则注入写作与审查；完整状态回灌（角色状态机） |
| **P3** | 设定协调（改设定→CDA 影响图→标注矛盾章节）；全书优化流水线；墨韵文风校准阶段 |

---

## 七、P0 验收标准

1. 讨论成果就位后点「开始写作」→ 从第 1 章开始自动连写，事件流实时看到 写作 → 审查 → 回灌 三阶段逐章推进
2. 正文逐章落库，笔耕自动出现该章，字数正确，**重启客户端不丢**
3. 审查 `score < 80` → 带 issues 自动重写；两次不过 → Failed + 报告可见，不死循环
4. 暂停 → 当前阶段完成后停住；继续 → 接着写；停止 → LLM 调用立即中断；再点开始 → 从未写章节继续
5. `.harness/` WAL 有完整 StageStart / StageComplete / GatePass / GateFail 记录
6. 写作 prompt 可验证包含：备忘录、本章梗概、世界观、人物、前章正文（热记忆）
7. 旧项目（ch-xxx 章节）加载后 backfill chapter_no，记忆包可正常构建
8. `cargo test` / `clippy -D warnings` / `fmt` 全过；新增核心逻辑有单元测试（解析器、序号分配）

## 八、明确不做的（本期）

- 不做修改联动的自动改写（P1 做建议制标记，正文永远由人/显式重写决定）
- 不做 function calling 工具循环（国产模型能力参差，prompt 注入更可控）
- 不做 YAML 工作流模板转换器（P1；P0 三阶段 Rust 硬编码）
- 不写通用门控表达式引擎（复用 GateEvaluator 现有能力）
- 不做开书定盘/全书规划阶段（灵魂萌芽的讨论+导入已覆盖该职能）
- 不做向量检索冷记忆（记忆系统关键词检索已够用）
