# PenSoul 全链路批注系统设计

> 最后更新：2026-08-02
> 定位：设计提案。把笔耕批注泛化为覆盖正文、细纲、脉络、人物志、世界观、萌芽的统一批注系统，并让批注历史成为学习进化的标注集底座。

## 一、现状与目标

**现状**：批注只有正文一套（`ChapterAnnotation` + TipTap 行内高亮 + `AnnotationPanel`），且处理流与"批注重写"绑定。世界观/人物志/大纲/细纲全是表单编辑，没有任何批注能力。

**目标**：
1. 一套批注模型覆盖全部创作对象，状态机与处理流统一；
2. 锚点分级，富文本行内、表单字段、整个条目三种粒度都支持；
3. 批注数据自动沉淀为学习标注集（衔接 EVOLVE-DESIGN 的 L0/L1）；
4. 旧项目零破坏迁移。

## 二、数据模型

### 2.1 通用批注（替换 `ChapterAnnotation`）

```rust
/// 锚点分级：富文本行内锚点（段落+偏移+原文）| 表单字段锚点（字段名）| 实体级（无锚点）
pub enum AnnotationAnchor {
    Inline { paragraph_index: usize, offset: usize, text: String },
    Field(String),   // 如 "description" / "summary"
}

pub struct Annotation {
    pub annotation_id: String,
    /// 稳定定位 key：如 "chapter:ch-001:body" / "world:location:loc-1:description"
    pub target: String,
    /// issue=问题 / suggestion=修改建议 / note=备注
    pub kind: String,
    pub anchor: Option<AnnotationAnchor>,
    pub content: String,
    /// open / accepted / rejected
    pub status: String,
    /// 批注创建时的锚定文本快照（漂移检测与学习数据用）
    #[serde(default)]
    pub anchor_snapshot: Option<String>,
    /// 判决来源：manual=用户直接处理 / rewrite_plan=重写计划（LLM 提案+用户默许）
    #[serde(default)]
    pub resolved_by: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}
```

`target` 用统一字符串约定，前端据此路由跳转，后端据此聚合统计：

| 目标 | target 示例 |
|---|---|
| 章节正文 | `chapter:ch-001:body` |
| 章节细纲 | `chapter:ch-001:summary` |
| 脉络节点 | `outline_arc:arc-1` |
| 人物 | `character:char-1:description` |
| 地点 | `world:location:loc-1:description` |
| 时间线事件 | `world:timeline:evt-1:description` |
| 设定规则 | `world:rule:rule-1:description` |
| 术语 | `world:glossary:term-1:description` |
| 核心概念 | `concept:high_concept` |
| 萌芽 | `sprout:premise` |

### 2.2 挂载方式：分散存储，随实体级联

每个可批注实体加 `#[serde(default)] pub annotations: Vec<Annotation>`：

- `Chapter.annotations`（正文 + 细纲共用，target 区分）；
- `OutlineArc.annotations`；
- `Character.annotations`；
- `WorldLayer` 下 `Location / TimelineEvent / SettingRule / TerminologyEntry .annotations`；
- `SproutData.annotations`（P2）。

理由：删除实体自动级联批注；旧项目 JSON 无该字段时 serde default 兜底；前端 `persistProjectData` 结构自然。聚合查询用后端 `annotations_all` 遍历本体一次即可，PenSoul 项目实体规模下开销可忽略。

## 三、锚点策略

| 粒度 | 适用 | 漂移处理 |
|---|---|---|
| 行内 Inline | 正文富文本 | 保存时按锚定文本重新定位，失败标记 stale（沿用现有逻辑） |
| 字段 Field | textarea/input 字段（细纲、描述、设定） | 批注挂字段整体，内容变化不失效，`anchor_snapshot` 留存对比 |
| 实体级 | 针对整个条目 | 无锚点，随实体存在 |

行内锚点只在正文使用；其余环节先用字段级与实体级，投入产出比最高，也天然抗内容漂移。

## 四、后端命令（泛型化）

在 `pensoul-app/src/commands/annotations.rs` 新增，替代各视图零散逻辑：

```
annotations_add(project_id, target, kind, content, anchor?) → Annotation
annotations_update(project_id, target, annotation_id, patch)
annotations_remove(project_id, target, annotation_id)
annotations_resolve(project_id, target, decisions)     // 逐条 accept/reject，记 resolved_by/resolved_at
annotations_list(project_id, target) → Vec<Annotation>
annotations_all(project_id) → 聚合收件箱（按实体类型分组 + open 计数）
annotations_export(project_id, kind?) → JSONL 标注集
```

**处理流解耦**：正文保留"批注重写"整合流程（rewrite plan → 蒸馏经验），但 accept/reject 不再只发生在重写时——`annotations_resolve` 支持任何实体逐条处理，正文批注也可先在面板手动处理再重写。

**蒸馏泛化**：`distill_lessons` 从"只吃正文 accepted 批注"扩为 `distill_lessons_from(project_id, scope)`：

| scope | 批注来源 | 经验去向 |
|---|---|---|
| chapter_review | 正文 | 现有 WritingLesson → 审查 prompt |
| outline_expand | 细纲/脉络 | 细纲展开提示词经验 |
| consistency | 设定/人物 | 一致性规则案例库 |

## 五、前端 UI

### 5.1 组件改造

- **`AnnotationPanel` 泛型化**：props 改为 `annotations + target + onResolve + onLocate`，去掉章节专用逻辑；每条批注支持逐条 accept/reject/编辑/删除/定位。
- **`AnnotatableField`（新）**：包装 input/textarea，右侧批注角标（open 计数），点击展开批注抽屉，自动生成字段级 target。细纲、描述、设定字段统一用它。
- **实体卡片批注按钮**：WorldView/CharacterView 每条目头部加批注图标，展开实体级批注。
- **行内批注**：TipTap 现有交互保留，底层类型切换为通用 `Annotation`。

### 5.2 批注中心（新入口）

侧边栏新增"批注"聚合视图：按实体类型分组展示全部 open 批注，点击经 target 路由跳转到对应视图并定位（行内锚点滚动到段落，字段级滚动到字段容器）。这是"批注即标注"工作流的总入口，也是用户处理批注的主界面。

### 5.3 视图接入优先级

| 优先级 | 视图 | 锚点粒度 |
|---|---|---|
| P0 | 笔耕（现有） | 行内 + 实体 |
| P0 | 大纲（细纲 textarea + 脉络卡片） | 字段 + 实体 |
| P1 | 世界观（地点/时间线/设定/术语） | 字段 + 实体 |
| P1 | 人物志 | 字段 + 实体 |
| P2 | 核心概念 / 萌芽 | 字段 |

## 六、学习进化衔接（核心价值）

批注全面化后，标注集从"仅正文"扩展为五大类，正好喂给 EVOLVE-DESIGN 的 L0/L1：

| 批注来源 | 进化评估信号 |
|---|---|
| 正文 accepted/rejected | 审查量规命中率/误报率（M1 量规进化评估器） |
| 细纲/脉络 | 细纲展开一次通过率校准 |
| 设定/人物 | 一致性检查规则案例（用户纠正的矛盾 = 真值） |
| 全部批注 | L0 校准集 `data/evolve/calibration/annotations.jsonl` |

`annotations_export` 输出的每条样本 = `(target_type, anchor_snapshot, content, decision, resolved_by)`。导出规则：

1. 只导 `accepted` / `rejected`，过滤 `open` 与 `note`；
2. `resolved_by` 区分"用户直接判决"（强标签）与"重写计划默许"（弱标签），学习时分层加权；
3. 标注集只含批注文本与锚定快照，不含完整私密正文，符合 EVOLVE-DESIGN 红线 2。

## 七、迁移与兼容

- `ChapterAnnotation` 保留为兼容别名/转换函数，旧项目 JSON 无需迁移；
- 现有 `save_chapter` 的 annotations 参数、TipTap mark、批注重写流程不动，仅内部换类型；
- 所有新字段 `#[serde(default)]`，加载零破坏；
- IPC 封装收敛在 `src/ipc.ts`，前端类型同步 `src/types.ts`。

## 八、实施步骤

1. **M1 数据层**：通用 `Annotation` + target 约定 + 各实体 `annotations` 字段（pensoul-core）；
2. **M2 命令层**：泛型批注命令 + 聚合 + JSONL 导出（pensoul-app）；
3. **M3 UI 层**：AnnotationPanel 泛型化 + AnnotatableField + 细纲/世界观/人物志接入；
4. **M4 批注中心**：聚合视图 + target 路由定位；
5. **M5 学习衔接**：`distill_lessons_from` 泛化 + 标注集接入 EVOLVE-DESIGN L0（与进化方案合并推进）。

## 九、红线

1. 批注只做"建议与标注"，不自动改写任何实体（维持建议制哲学）；
2. 标注集导出不含私密正文全文，只含快照片段；
3. 判决标签必须保留 `resolved_by`，弱标签不得冒充人工标注；
4. 不引入新存储（不建数据库、不建独立批注文件），全部随本体 JSON 持久化。
