# PenSoul 创作全流程手册

> 最后更新：2026-08-01 · 对齐 2026-08-01 代码状态
> 从用户视角描述完整创作流程，并标注每个环节对应的系统实现

## 总览

```
项目创建 → 核心概念 → 世界观/人物志/创作设定
  → 萌芽想法 → 多 Agent 讨论 → 确认成果
  → 情节脉络（大纲规划层）
  → 细纲展开（生成真实章节）
  → 造化工坊连写闭环（写作 → 审查 → 回灌）
  → 笔耕手改 → 影响图联动修订建议
```

可选增强：专家蒸馏（把人物思维提炼为讨论 Agent）、书籍蒸馏（把一本书的写法提炼为工作流技能卡）。
工作流采用「全局模板 + 项目引用」分层：作品库「工作流模板库」定义网文/传统/科幻/通用等模板，
项目内只选择要用的模板并做环节覆盖，造化工坊按模板展开执行。

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
3. 讨论结束，系统给出结构化成果（共识总结 + 地点/时间线/设定规则/人物及人物关系）。
4. 确认后成果写入世界观与人物志；大纲页可用它作为情节脉络的起点。

### 系统实现

- 命令：`discuss_concept`（长跑，`discussion-event` 推送进度）、`get_discussion_state`（重连恢复）。
- 三轮流程：立论 → 交锋（互读他人发言）→ 成果提炼（单次综合调用，`json_fix` 修复 LLM 产物）。
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

## 五、工作流：全局模板 → 项目引用 → 造化工坊执行（核心）

### 5.1 作品库「工作流模板库」

在作品库侧边栏的「工作流」页面维护全局模板（存 `data/workflows/templates.json`，所有项目共享）：

- 内置模板：网文创作流（webnovel，80 分放行）、标准小说流（standard-novel，85 分）、科幻创作流（scifi，85 分）、快速创作流（quick-novel，70 分）；
- 可新建自定义模板：设置名称/体裁/说明/审查放行阈值，以及三个执行阶段（章节写作 → 一致性审查 → 状态回灌）的手册、门控、重试与回退；
- 内置模板不可删除，可「恢复内置」出厂；模板带版本号，项目记录引用版本，模板后续更新不污染进行中作品。

### 5.2 项目内「工作流」页：选模板 + 项目覆盖

每个项目只保存一个引用（`workflow_ref`：模板 ID + 版本 + 覆盖），不复制整套模板：

- 从启用的模板中选择本项目要用的模板（不选也可只绑技能卡）；
- 三个执行环节（细纲展开 / 章节写作 / 一致性审查）可按维度绑定模型与技法卡（WritingCard 技能卡）；
- 项目覆盖优先，模板绑定兜底，两者都缺时自动选第一个可用模型。

### 5.3 造化工坊按模板执行

### 用户操作

1. 在项目「工作流」页选好模板并做环节覆盖（可现场再指定写作/审查模型）。
2. 打开「造化工坊」点「开始写作」：系统按模板的阶段手册与门控逐章连写，实时展示事件流。
3. 随时可**暂停**（当前阶段完成后停住）/ **继续** / **停止**（立即中断，已落库章节保留）。
4. 写完后笔耕页面自动出现新章节（只显示 word_count > 0 的章节）。

### 系统实现

- 命令：`run_chapter_pipeline(chapter_ids?, writing_model?, review_model?, writing_cards?, review_cards?)`（长跑）+ `pause_pipeline` / `resume_pipeline` / `stop_pipeline` / `get_pipeline_state`。
- 模板命令：`list_workflow_templates` / `save_workflow_templates` / `reset_workflow_templates`（作品库层面）；
- 项目引用命令：`save_workflow_ref` / `load_workflow_ref`（项目内只存引用 + 覆盖）。
- 选章：有梗概且正文为空，按 `chapter_no` 升序。
- 每章闭环：

```
chapter_writing（auto，写作模型）
  → chapter_review（conditional，异模型，on_fail=writing，max_retries=2，score≥80 放行）
  → state_injection（auto，正文落库 + 增量更新派生状态）
```

- 解析规则：显式参数（造化工坊现场选择）→ 项目覆盖 → 模板绑定 → 自动选模型；模板的 `review_pass_score` 转成审查门控表达式（如 `score >= 85`），阶段手册注入引擎 manual。
- 上下文组装：核心概念 + 创作设定 + 本章梗概 + 记忆包（热/温/冷/叙事）+ 技能卡注入 + 前文衔接。
- 事件：`harness-event`（chapter_start / stage_start / llm_output / review_report / gate / effect / chapter_done / chapter_failed / paused / resumed / pipeline_done）。
- 引擎保障：门控、回退重试、WAL 崩溃恢复，进度已落库，重启不丢。

---

## 六、笔耕手改与影响联动

### 用户操作

在 WritingView 手改章节正文，保存后系统自动：

1. 标记受影响章节（影响图 BFS 反向传播，Direct / Indirect / Cascading 三级）。
2. 给出修订建议（建议制，不自动改写）。
3. 更新记忆、一致性检查结果、并发版本。

### 系统实现

- 命令：`save_chapter(chapter_id, content, expected_version)`（乐观并发，返回新版本号）、`find_affected(chapter_id, changed_entities)`、`get_impact_graph`、`check_consistency`。
- `on_chapter_saved` 增量更新：记忆 8 步管道 → 影响图重建 → 实体状态 upsert → 版本推进。

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
- 工作流绑定：在项目「工作流」页把技能卡绑定到细纲展开/章节写作/一致性审查环节（写入 `workflow_ref.overrides`，模板绑定兜底）；管线与细纲展开按 `applicable_stages` 注入对应技能卡。旧 `save_workflow_skills` / `load_workflow_skills` 仅做迁移兜底。

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
