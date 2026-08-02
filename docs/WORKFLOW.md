# PenSoul 创作全流程手册

> 最后更新：2026-08-02 · 对齐 2026-08-02 代码状态
> 从用户视角描述完整创作流程，并标注每个环节对应的系统实现

## 总览

```
项目创建 → 核心概念 → 世界观/人物志/创作设定
  → 萌芽想法 → 多 Agent 讨论 → 确认成果
  → 情节脉络（大纲规划层）
  → 细纲展开（生成真实章节）
  → 造化工坊连写闭环（章前策划 → 写作 → 审查 → 回灌）
  → 笔耕手改 → 影响图联动修订建议
```

可选增强：专家蒸馏（把人物思维提炼为讨论评审员）、书籍蒸馏（把一本书的写法提炼为工作流技能卡）、方法论蒸馏（把一段写作方法论提炼为技能卡）。
工作流采用「全局模板 + 项目引用」分层：主页「工作流」统一维护模板库、环节技能绑定（模型 + 技法卡）与写作技能库，
项目内只需在「造化工坊」选择要用的模板，造化工坊按模板展开执行。

---

## 一、项目创建与设定

### 用户操作

1. 在项目仪表盘创建项目（标题 + 描述）。
2. 填写**核心概念**：高概念、前提、主角、基调、核心冲突。
3. 填写**创作设定**：类型、目标总章数、每章目标字数。
4. 逐步构建**世界观**（WorldView）与**人物志**（CharacterView）。

### 系统实现

| 动作 | 命令 |
|---|---|
| 创建/列出/打开/删除项目 | `create_project` / `list_projects` / `open_project` / `delete_project` |
| 保存概念/设定/世界观/人物志 | `save_concept` / `save_settings` / `save_world` / `save_characters` |

设定文件落在 `data/projects/<id>/`，原子写落盘；打开项目时自动 backfill 章节序号并重建派生状态。

---

## 二、萌芽想法 → 多 Agent 讨论 → 确认成果

### 用户操作

1. 在概念页写下萌芽想法（构思 + 创作设定上下文），挑选讨论 Agent（可来自专家库）。
2. 点「开始讨论」，实时观看每个 Agent 的发言进度。
3. 讨论结束，系统给出结构化成果（共识总结 + 地点/时间线/设定规则/人物及人物关系），并显式列出讨论中的分歧（已收敛/未收敛）与独立裁判的裁决建议。
4. 确认后成果写入世界观与人物志；大纲页可用它作为情节脉络的起点。

### 系统实现

- 命令：`discuss_concept`（长跑，`discussion-event` 推送进度）、`get_discussion_state`（重连恢复）。
- 三轮流程：立论 → 交锋（互读他人发言）→ 成果提炼（`discussion_synthesis.rs`：五路并行分维度提炼 → 跨维度冲突检查 → 独立裁判裁决，`json_fix` 修复 LLM 产物）。
- 分维度提炼依据：选择聚集优于合成聚集（直接把多 Agent 发言揉成一段共识会损失质量），每个维度都读完整讨论记录并显式输出分歧（各方立场 + 依据，不抹平）；跨维度矛盾显式标记；未收敛分歧由第二个 Agent 模型独立裁决，裁决建议（可直接采用的设定文字）随成果展示，不静默改写已提炼条目。
- 专家库 Agent 自动加载其 `SKILL.md` 作为系统提示词。
- 结果持久化到 `sprout.last_discussion`；讨论期间可切页面，回来用 `get_discussion_state` 重放。

---

## 三、情节脉络（大纲规划层）

### 用户操作

在 OutlineView 建立「情节脉络」节点：每个节点覆盖一个章节范围（如第 1-200 章），写下该故事段的剧情规划。

### 系统实现

- 命令：`list_outline_arcs` / `save_outline_arcs`（新建/编辑/删除都走整体保存）。
- 脉络节点（`OutlineArc`）是规划层，本身不产正文、不参与造化工坊选章。

---

## 四、细纲展开（生成真实章节）

### 用户操作

选中脉络节点点「展开细纲」：每次展开一批（默认 20 章），生成带梗概的真实章节。多次点击逐步展开，按自己的节奏推进，而不是一次吞下几百章。

### 系统实现

- 命令：`expand_outline_arc(arc_id, model?, batch?, skill_cards?)`。
- 章号由后端按批次顺序分配，不信任模型编号；上下文注入核心概念 + 创作设定 + 节点规划 + 前情衔接（本批起点前最近 2 章梗概）。
- 生成的章节正文留空（word_count == 0），等待造化工坊写作；落第一个真实卷或 `_default` 卷。

---

## 五、工作流：全局模板 → 项目选模板 → 造化工坊执行（核心）

### 5.1 主页「工作流」：模板库 + 环节绑定 + 技能库

在侧边栏「工作流」页面统一维护全局配置（存 `data/workflows/templates.json`，所有项目共享）：

- 内置模板：核心内置仅保留「网文创作流」（webnovel，80 分放行）；标准小说流 / 科幻创作流 / 快速创作流已删除，需要时可「恢复内置模板」找回出厂版本；
- 可新建自定义模板：设置名称/体裁/说明/审查放行阈值，以及执行阶段（默认：章前策划 → 章节写作 → 卖点与质量审查 → 状态回灌）的手册、门控、重试与回退；
- 每个模板可展开「环节技能绑定」：为细纲展开 / 章节写作 / 一致性审查按维度绑定模型与技法卡（WritingCard 技能卡），所有使用该模板的项目自动生效；章前策划的守则在模板「执行阶段」里直接编辑；
- 「写作技能库」在此蒸馏/删除技能包（支持「蒸馏一本书」与「蒸馏方法论」两种入口），删除技能包会同步清理所有模板绑定中的死引用；内置「网文创作方法论」卡组随 webnovel v2 模板自动绑定；
- 自定义模板与除网文外的内置模板可直接删除；核心内置「网文创作流」点删除会改为停用（不可删除），可「恢复内置」出厂；模板带版本号，项目记录引用版本，模板后续更新不污染进行中作品；
- 「清空项目覆盖」按钮可一键清空遗留的项目级覆盖（覆盖层已退役）。

### 5.2 造化工坊页：选模板

项目内不再有独立「工作流」页；每个项目在「造化工坊」页顶部的下拉框选择要用的模板（存 `workflow_ref`：模板 ID + 版本），不复制整套模板：

- 从启用的模板中选择本项目要用的模板（不选也可只绑技能卡）；
- 模板与环节绑定统一在主页「工作流」配置，项目层不再做环节覆盖；
- 解析优先级：显式参数（造化工坊现场选模型/技法卡）→ 模板绑定 → 自动选第一个可用模型。

### 5.3 造化工坊按模板执行

### 用户操作

1. 在「造化工坊」页选好本项目要用的模板（可现场再指定写作/审查模型）。
2. 打开「造化工坊」点「开始写作」：系统按模板的阶段手册与门控逐章连写，实时展示事件流。
3. 随时可**暂停**（当前阶段完成后停住）/ **继续** / **停止**（立即中断，已落库章节保留）。
4. 写完后笔耕页面自动出现新章节（只显示 word_count > 0 的章节）。

### 系统实现

- 命令：`run_chapter_pipeline(chapter_ids?, writing_model?, review_model?, writing_cards?, review_cards?)`（长跑）+ `pause_pipeline` / `resume_pipeline` / `stop_pipeline` / `get_pipeline_state`。
- 模板命令：`list_workflow_templates` / `save_workflow_templates` / `reset_workflow_templates` / `clear_all_project_overrides`（主页「工作流」层面）；
- 项目引用命令：`save_workflow_ref` / `load_workflow_ref`（项目内只存模板引用）。
- 选章：有梗概且正文为空，按 `chapter_no` 升序。
- 每章闭环（网文创作流 v2；默认三阶段模板无「章前策划」）：

```
chapter_planning（auto，写作模型，产出节拍表 JSON 写入滚动备忘录）
  → chapter_writing（auto，写作模型，按节拍表 + 反 AI 味铁律写正文）
  → chapter_review（conditional，异模型，on_fail=writing，max_retries=2，score≥80 放行）
  → state_injection（auto，正文落库 + 增量更新派生状态）
```

- 解析规则：显式参数（造化工坊现场选择）→ 项目覆盖 → 模板绑定 → 自动选模型；模板的 `review_pass_score` 转成审查门控表达式（如 `score >= 85`），阶段手册注入引擎 manual。
- 上下文组装：核心概念 + 创作设定 + 本章梗概 + 章前节拍表 + 记忆包（热/温/冷/叙事）+ 技能卡注入 + 前文衔接；写作阶段注入黄金三章（前 3 章）与反 AI 味语言铁律，审查按七维加权打分（卖点 20 / 开场钩子 10 / 情绪曲线 20 / 场景节奏 10 / 断章钩子 15 / 一致性 15 / 文笔 10）。
- 黄金三章硬门控（模板 review 环节声明 `golden_gate` 时生效）：前 3 章审查 SIGNAL 额外输出 hook/payoff 两个 0-10 子分数，门控表达式升级为 `score >= 阈值 && hook >= 8 && payoff >= 8`，任一不达标即拦截重写（引擎硬门控，非提示词软约束）。
- 事件：`harness-event`（chapter_start / stage_start / llm_output / review_report / gate / effect / chapter_done / chapter_failed / paused / resumed / pipeline_done）。
- 引擎保障：门控、回退重试、WAL 崩溃恢复，进度已落库，重启不丢。

---

## 六、笔耕手改与影响联动

### 用户操作

在 WritingView 手改章节正文，选中文字可添加行内批注（问题/修改建议/备注），
也可在批注面板添加整章批注；点「按批注重写本章」让 AI 按批注重写（新版本，原稿进版本历史可回滚）。保存后系统自动：

1. 标记受影响章节（影响图 BFS 反向传播，Direct / Indirect / Cascading 三级）。
2. 给出修订建议（建议制，不自动改写）。
3. 更新记忆、一致性检查结果、并发版本。

### 系统实现

- 命令：`save_chapter(chapter_id, content, expected_version, annotations?)`（乐观并发，返回新版本号）、`rewrite_chapter_with_annotations`（修改计划 → 重写正文 → 批注状态流转 → 经验沉淀）、`list_chapter_revisions` / `rollback_chapter`（版本回滚）、`get/save_writing_lessons`、`find_affected(chapter_id, changed_entities)`、`get_impact_graph`、`check_consistency`。
- `on_chapter_saved` 增量更新：记忆 8 步管道 → 影响图重建 → 实体状态 upsert → 版本推进。
- 批注锚点：段落索引 + 段内偏移 + 锚定原文片段；正文被改后按原文片段容错匹配，失败退化为段落级定位。
- 写作经验库：批注重写把已采纳批注归类为经验（措辞/节奏/对话/一致性/反AI味/结构/其他），同类去重累计次数，注入章节审查 prompt 重点检查是否重犯。

---

## 七、专家蒸馏（可选）

### 用户操作

在 ExpertLibraryView 输入一个人物（如某位作家/思想者），选模型点「蒸馏」，实时看六维调研进度；产物出现在专家库，可作为讨论 Agent 参与概念讨论。

### 系统实现

- 命令：`distill_expert(persona, model?)`，进度经 `distill-phase` 推送。
- 产物：`Experts/<名字>-expert/SKILL.md` + `references/research/` 调研存档。
- 方法论：`skills/pensoul-skill-Experts/SKILL.md`（缺文件时用内置简版兜底）。
- 核心红线：提炼思维框架（HOW they think），不写身份卡/生平/谱系；矛盾保留；诚实标注局限。

---

## 八、书籍蒸馏（可选）

### 用户操作

在书籍蒸馏面板输入书名/作者，可选上传书籍文件或粘贴样章，勾选维度（文风 DNA / 叙事结构 / 人物塑造 / 冲突与张力 / 类型范式），点「蒸馏」。产物成为可绑定工作流环节的技能卡。

### 系统实现

- 命令：`distill_book(title, author?, file_path?, sample_text?, dimensions?, model?)`，进度经 `book-distill-phase` 推送；`list_book_packages` / `delete_book_package` 管理产物。
- 产物：`WritingCard/<书名>-book/`，五维各一张 SKILL.md，六段构卡（R/I/A1/A2/E/B）。
- 知识蒸馏模式（无样章）在 B 段标注置信度边界。
- 工作流绑定：在主页「工作流」把技能卡绑定到模板的细纲展开/章节写作/一致性审查环节（写入 `template.bindings`）；管线与细纲展开按 `applicable_stages` 注入对应技能卡。旧 `save_workflow_skills` / `load_workflow_skills` 仅做迁移兜底。

## 八之二、方法论蒸馏（可选）

在主页「工作流」→「写作技能库」点「蒸馏方法论」，粘贴一段写作方法论（经验贴/讲稿/课程摘录，≤2 万字），勾选维度（文风规则 / 结构与编排 / 人物塑造 / 冲突与张力 / 类型范式 / 审查标准），点「蒸馏」。

### 系统实现

- 命令：`distill_methodology(title, methodology_text, dimensions?, model?)`，进度经 `methodology-distill-phase` 推送。
- 产物：`WritingCard/<标题>-methodology/`（六维各一张 SKILL.md + package.json + INDEX.md + references/research/ 存档）。
- 方法论：`skills/pensoul-skill-Methodology/SKILL.md`（缺文件时用内置简版兜底）；与书籍蒸馏同用 RIA++ 六段与三重验证，A1 段改为「方法原文案例」。
- 内置卡组：`WritingCard/网文创作方法论-methodology/` 预置六张卡，webnovel v2 模板的环节绑定直接引用。

---

## 九、常见问题

**Q：造化工坊写了几章想换模型继续？**
直接停掉再开新一轮；`run_chapter_pipeline` 只选未写章节，已写的跳过。

**Q：暂停和停止有什么区别？**
暂停是阶段边界停住，随时可继续；停止是立即中断 LLM 调用，本章已写内容是否落库取决于落库时机，已完成的章节进度保留。

**Q：讨论中断/切页了怎么办？**
讨论事件写入了控制面缓冲，`get_discussion_state` 重放恢复；成果只要提炼过就持久化在 `sprout.last_discussion`。

**Q：手改章节会不会被自动改写？**
不会。系统只标记受影响章节并给建议，所有 AI 产物都是建议制。
