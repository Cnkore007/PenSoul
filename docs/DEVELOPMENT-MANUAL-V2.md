# PenSoul v2 — Rust 开发手册

**版本**：v2.0
**日期**：2026-07-26
**技术栈**：Rust (edition 2024) + Tauri 2
**文档依据**：DESIGN-V2.md、FEASIBILITY-REPORT.md、Python 原型代码

---

## 目录

- [第一章：项目总览与 Workspace 架构](#第一章项目总览与-workspace-架构)
- [第二章：pensoul-core — 核心类型与本体论](#第二章pensoul-core--核心类型与本体论)
- [第三章：pensoul-harness — 确定性流程引擎](#第三章pensoul-harness--确定性流程引擎)
- [第四章：pensoul-cda — 一致性驱动架构](#第四章pensoul-cda--一致性驱动架构)
- [第五章：pensoul-memory — 四层记忆系统](#第五章pensoul-memory--四层记忆系统)
- [第六章：pensoul-agent — 智能体通信](#第六章pensoul-agent--智能体通信)
- [第七章：pensoul-concurrency — 并发控制](#第七章pensoul-concurrency--并发控制)
- [第八章：pensoul-plugin — 插件系统](#第八章pensoul-plugin--插件系统)
- [第九章：pensoul-consistency — 增量一致性检查](#第九章pensoul-consistency--增量一致性检查)
- [第十章：pensoul-import — 数据导入导出](#第十章pensoul-import--数据导入导出)
- [第十一章：pensoul-llm — LLM 模型管理](#第十一章pensoul-llm--llm-模型管理)
- [第十二章：pensoul-app — Tauri 桌面应用](#第十二章pensoul-app--tauri-桌面应用)
- [第十三章：模块间集成与验收](#第十三章模块间集成与验收)
- [附录 A：Crate 依赖关系图](#附录-acrate-依赖关系图)
- [附录 B：总验收清单](#附录-b总验收清单)

---

# 第一章：项目总览与 Workspace 架构

## 1.1 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri | 2.x |
| 后端语言 | Rust | edition 2024 |
| 前端 | React + TipTap | — |
| 向量数据库 | LanceDB | — |
| 关系数据库 | SQLite（rusqlite） | — |
| 图数据库 | petgraph | 6.x |
| 序列化 | serde + serde_json | — |
| YAML 解析 | serde_yaml | — |
| 异步运行时 | tokio | 1.x |
| 错误处理 | thiserror + anyhow | — |
| UUID | uuid | 1.x |
| 日志 | tracing + tracing-subscriber | — |
| 哑光哈希 | blake3 | — |

## 1.2 Workspace 目录结构

```
pensoul/
├── Cargo.toml                    # Workspace 根配置
├── crates/
│   ├── pensoul-core/             # 核心类型与本体论
│   ├── pensoul-harness/          # 确定性流程引擎
│   ├── pensoul-cda/              # 一致性驱动架构
│   ├── pensoul-memory/           # 四层记忆系统
│   ├── pensoul-agent/            # 智能体通信
│   ├── pensoul-concurrency/      # 并发控制
│   ├── pensoul-plugin/           # 插件系统
│   ├── pensoul-consistency/      # 增量一致性检查
│   ├── pensoul-import/           # 数据导入导出
│   ├── pensoul-llm/              # LLM 模型管理
│   └── pensoul-app/              # Tauri 桌面应用
├── src/                          # (Tauri) 前端
└── docs/
    ├── DESIGN-V2.md
    ├── DEVELOPMENT-MANUAL.md
    └── DEVELOPMENT-MANUAL-V2.md
```

## 1.3 根 Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/pensoul-core",
    "crates/pensoul-harness",
    "crates/pensoul-cda",
    "crates/pensoul-memory",
    "crates/pensoul-agent",
    "crates/pensoul-concurrency",
    "crates/pensoul-plugin",
    "crates/pensoul-consistency",
    "crates/pensoul-import",
    "crates/pensoul-llm",
    "crates/pensoul-app",
]
```

---

# 第二章：pensoul-core — 核心类型与本体论

## 2.1 职责说明

定义全项目共享的领域类型：四层世界模型（世界/角色/叙事/审美）、章节结构、错误类型、通用 ID 类型。所有其他 crate 依赖此 crate，不允许反向依赖。

## 2.2 文件结构

```
crates/pensoul-core/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # 模块入口
│   ├── id.rs                     # 新类型 ID
│   ├── error.rs                  # 错误类型
│   ├── world.rs                  # Layer 1 世界层
│   ├── character.rs              # Layer 2 角色层
│   ├── narrative.rs              # Layer 3 叙事层
│   ├── aesthetic.rs              # Layer 4 审美层
│   ├── chapter.rs                # 章节结构
│   ├── ontology.rs               # 四层本体协调
│   └── prelude.rs                # 常用导出
```

## 2.3 核心数据类型

```rust
// ─── id.rs ────────────────────────────────────────

use std::fmt;

/// 新类型 ID 模式，类型安全的字符串标识
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(WorldId);
define_id!(CharacterId);
define_id!(EventId);
define_id!(ForeshadowId);
define_id!(SettingId);
define_id!(ChapterId);
define_id!(VolumeId);
define_id!(NodeId);         // 影响图节点
define_id!(EdgeId);         // 影响图边
define_id!(EntityId);       // 通用实体
define_id!(SkillId);        // 写法资产
define_id!(RuleId);         // 规则
define_id!(AgentId);
define_id!(StageName);
define_id!(PluginId);
define_id!(ProjectId);
```

```rust
// ─── error.rs ─────────────────────────────────────

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PensoulError {
    #[error("阶段未注册: {0}")]
    StageNotFound(String),

    #[error("工具访问被拒绝: {tool} 在阶段 {stage} 中不被允许")]
    ToolAccessDenied { tool: String, stage: String },

    #[error("门控条件不满足: {reason}")]
    GateConditionFailed { reason: String },

    #[error("版本冲突: 章节 {chapter_id} 期望版本 {expected}，实际版本 {actual}")]
    VersionConflict {
        chapter_id: i64,
        expected: i32,
        actual: i32,
    },

    #[error("操作被拒绝: {0}")]
    OperationRejected(String),

    #[error("插件验证失败: {errors:?}")]
    PluginValidationFailed { errors: Vec<String> },

    #[error("一致性违反: {entity_id} 在第 {chapter_a} 章和第 {chapter_b} 章之间不一致")]
    ConsistencyViolation {
        entity_id: String,
        chapter_a: i64,
        chapter_b: i64,
        description: String,
    },

    #[error("WAL 校验失败: 条目 {index} checksum 不匹配")]
    WalChecksumFailed { index: usize },

    #[error("LLM 调用失败: 所有模型均不可用，尝试链: {chain:?}")]
    LlmAllModelsFailed { chain: Vec<String> },

    #[error("导入失败: {0}")]
    ImportError(String),

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PensoulError>;
```

```rust
// ─── world.rs ─────────────────────────────────────

/// Layer 1: 世界层 — 空间、时间、设定、术语、物品
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldLayer {
    pub world_id: WorldId,
    pub name: String,
    pub spatial_model: SpatialModel,
    pub timeline: Timeline,
    pub setting_rules: Vec<SettingRule>,
    pub glossary: Vec<TerminologyEntry>,
    pub item_graph: Vec<ItemNode>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpatialModel {
    pub locations: Vec<Location>,
    pub hierarchy: Vec<(LocationId, LocationId)>, // (parent, child)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub name: String,
    pub description: String,
    pub spatial_tags: Vec<String>,
}

define_id!(LocationId);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Timeline {
    pub events: Vec<TimelineEvent>,
    pub epoch_markers: Vec<EpochMarker>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    pub event_id: EventId,
    pub story_time: String,           // 故事内时间戳
    pub chapter_id: Option<ChapterId>,
    pub description: String,
    pub participants: Vec<CharacterId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EpochMarker {
    pub name: String,
    pub story_time: String,
    pub description: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingRule {
    pub rule_id: RuleId,
    pub category: String,
    pub title: String,
    pub description: String,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologyEntry {
    pub term: String,
    pub definition: String,
    pub aliases: Vec<String>,
    pub category: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ItemNode {
    pub item_id: EntityId,
    pub name: String,
    pub description: String,
    pub properties: std::collections::HashMap<String, String>,
    pub owner: Option<CharacterId>,
}
```

```rust
// ─── character.rs ─────────────────────────────────

/// Layer 2: 角色层 — 角色状态机、关系拓扑、成长曲线、对话风格、知识库
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterLayer {
    pub characters: Vec<Character>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub core_personality: PersonalityVector,
    pub current_mood: Emotion,
    pub current_location: Option<LocationId>,
    pub current_knowledge: KnowledgeSet,
    pub state_history: Vec<StateTransition>,
    pub transition_rules: Vec<TransitionRule>,
    pub dialogue_style: DialogueStyle,
    pub growth_curve: Vec<GrowthPoint>,
    pub knowledge_base: CharacterKnowledgeBase,
}

/// 核心人格向量 — 基本不变
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersonalityVector {
    pub traits: Vec<(String, f32)>,  // (特质名, 强度 0.0-1.0)
}

/// 当前情绪
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Emotion {
    pub primary: String,
    pub intensity: f32,
    pub secondary: Option<String>,
}

/// 角色当前知识集
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct KnowledgeSet {
    pub facts: Vec<KnowledgeItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeItem {
    pub fact_id: String,
    pub content: String,
    pub source: KnowledgeSource,
    pub reliability: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum KnowledgeSource {
    Observed,
    Told { from: CharacterId },
    Inferred,
    Remembered,
}

/// 状态转换记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: EventId,
    pub chapter_id: ChapterId,
    pub story_time: String,
    pub causality: String,
}

/// 状态转换规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransitionRule {
    pub event_pattern: String,
    pub from_state: String,
    pub to_state: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueStyle {
    pub patterns: Vec<String>,
    pub vocabulary_level: String,
    pub sentence_length_avg: f32,
    pub catchphrases: Vec<String>,
}

/// 成长曲线数据点
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrowthPoint {
    pub chapter_id: ChapterId,
    pub dimension: String,
    pub value: f32,
    pub note: String,
}

/// 角色知识库 — 防信息穿越
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterKnowledgeBase {
    pub known_facts: Vec<KnowledgeItem>,
    pub knowledge_sources: std::collections::HashMap<String, KnowledgeSourceRecord>,
    pub decay_model: DecayModel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeSourceRecord {
    pub source: KnowledgeSource,
    pub obtained_at: ChapterId,
    pub reliability: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecayModel {
    pub half_life_chapters: i32,
    pub min_reliability: f32,
}

/// 关系拓扑
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relationship {
    pub from: CharacterId,
    pub to: CharacterId,
    pub relation_type: String,
    pub strength: f32,
    pub history: Vec<RelationshipChange>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipChange {
    pub chapter_id: ChapterId,
    pub old_type: String,
    pub new_type: String,
    pub reason: String,
}
```

```rust
// ─── narrative.rs ─────────────────────────────────

/// Layer 3: 叙事层 — 情节图谱、伏笔追踪、冲突矩阵、情感弧线
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeLayer {
    pub plot_graph: Vec<PlotNode>,
    pub foreshadows: Vec<Foreshadow>,
    pub conflicts: Vec<Conflict>,
    pub emotional_arcs: Vec<EmotionalArc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlotNode {
    pub event_id: EventId,
    pub chapter_id: ChapterId,
    pub title: String,
    pub description: String,
    pub causality_from: Vec<EventId>,
    pub causality_to: Vec<EventId>,
}

/// 伏笔全生命周期
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Foreshadow {
    pub id: ForeshadowId,
    pub name: String,
    pub description: String,
    pub status: ForeshadowStatus,
    pub planted_chapter: ChapterId,
    pub expected_resolve_chapter: Option<ChapterId>,
    pub actual_resolve_chapter: Option<ChapterId>,
    pub related_characters: Vec<CharacterId>,
    pub related_items: Vec<EntityId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ForeshadowStatus {
    Planned,
    Planted,
    Progressing,
    Resolved,
    Abandoned,
    Overdue,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conflict {
    pub conflict_id: String,
    pub parties: Vec<CharacterId>,
    pub chapter_id: ChapterId,
    pub description: String,
    pub resolution: Option<String>,
    pub resolution_chapter: Option<ChapterId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmotionalArc {
    pub character_id: CharacterId,
    pub data_points: Vec<(ChapterId, f32)>, // (chapter, emotion_value)
}
```

```rust
// ─── aesthetic.rs ──────────────────────────────────

/// Layer 4: 审美层 — 文风指纹、叙事节奏、反AI规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AestheticLayer {
    pub style_fingerprint: StyleFingerprint,
    pub pacing_model: PacingModel,
    pub anti_ai_rules: Vec<AntiAiRule>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StyleFingerprint {
    pub sentence_length_avg: f32,
    pub vocabulary_richness: f32,
    pub rhetorical_frequency: f32,
    pub dialogue_ratio: f32,
    pub paragraph_length_avg: f32,
    pub sample_texts: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacingModel {
    pub tension_curve: Vec<(ChapterId, f32)>,
    pub scene_length_avg: f32,
    pub action_ratio: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntiAiRule {
    pub rule_id: RuleId,
    pub pattern: String,
    pub action: AntiAiAction,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AntiAiAction {
    Rewrite,
    Flag,
    Remove,
}
```

```rust
// ─── chapter.rs ────────────────────────────────────

/// 章节结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chapter {
    pub chapter_id: ChapterId,
    pub volume_id: Option<VolumeId>,
    pub title: String,
    pub content: String,
    pub word_count: usize,
    pub version: i32,
    pub status: ChapterStatus,
    pub consistency_score: Option<f32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ChapterStatus {
    Draft,
    Reviewing,
    Reviewed,
    Polished,
    Published,
}

/// 卷结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Volume {
    pub volume_id: VolumeId,
    pub title: String,
    pub chapter_ids: Vec<ChapterId>,
    pub summary: Option<String>,
}
```

```rust
// ─── ontology.rs ───────────────────────────────────

/// 四层本体 — 整合所有层
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NovelOntology {
    pub project_id: ProjectId,
    pub title: String,
    pub world: WorldLayer,
    pub characters: CharacterLayer,
    pub narrative: NarrativeLayer,
    pub aesthetic: AestheticLayer,
    pub chapters: Vec<Chapter>,
    pub volumes: Vec<Volume>,
}

impl NovelOntology {
    /// 创建空白项目
    pub fn new(project_id: ProjectId, title: String) -> Self {
        todo!("实现创建空白本体")
    }

    /// 获取指定章节
    pub fn get_chapter(&self, chapter_id: &ChapterId) -> Option<&Chapter> {
        self.chapters.iter().find(|c| &c.chapter_id == chapter_id)
    }

    /// 获取指定角色
    pub fn get_character(&self, character_id: &CharacterId) -> Option<&Character> {
        self.characters.characters.iter().find(|c| &c.id == character_id)
    }

    /// 获取所有活跃伏笔
    pub fn active_foreshadows(&self) -> Vec<&Foreshadow> {
        self.narrative
            .foreshadows
            .iter()
            .filter(|f| matches!(f.status, ForeshadowStatus::Planted | ForeshadowStatus::Progressing))
            .collect()
    }
}
```

## 2.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 1 | 新类型 ID 类型安全 | 编译测试 | 不同 ID 类型之间不能隐式转换 |
| 2 | 四层本体序列化/反序列化 | serde round-trip | JSON 往返完全一致 |
| 3 | PensoulError 覆盖所有错误场景 | 代码审查 | 每个 crate 的错误都能映射到 PensoulError |
| 4 | 角色知识库防穿越 | 单元测试 | 角色 A 不能访问角色 B 的专属知识 |
| 5 | 伏笔状态机 5 态流转 | 状态转换测试 | Planned→Planted→Progressing→Resolved/Abandoned 路径全部合法 |
| 6 | NovelOntology 创建空白项目 | 集成测试 | 返回非空实例，默认四层为空 |

---

# 第三章：pensoul-harness — 确定性流程引擎

## 3.1 职责说明

实现核心创新：确定性流程引擎。负责阶段状态机、门控三模式、工具白名单、WAL 崩溃恢复、滚动备忘录。所有流程决策由引擎代码做出，AI 无权干预。

## 3.2 文件结构

```
crates/pensoul-harness/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs               # 引擎核心
│   ├── stage.rs                # 阶段定义与实例
│   ├── gate.rs                 # 门控三模式
│   ├── wal.rs                  # Write-Ahead Log
│   ├── memo.rs                 # 滚动备忘录
│   ├── tools.rs                # 工具白名单
│   ├── runner.rs               # 执行者矩阵
│   └── recovery.rs             # 崩溃恢复
```

## 3.3 核心数据类型

```rust
// ─── stage.rs ─────────────────────────────────────

use pensoul_core::{StageName, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 门控类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    /// 自动放行 — 做完就过（ETC）
    Auto,
    /// 人工放行 — 必须等用户确认（收费站窗口）
    Manual,
    /// 条件放行 — 根据检查结果判定（检查站）
    Conditional,
}

/// 执行者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerType {
    /// 本机执行 — 同一个 AI 换手册
    Local,
    /// 委托执行 — 独立专家
    Delegated,
}

/// 阶段状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pending,
    Running,
    WaitingGate,
    WaitingHuman,
    Completed,
    Failed,
    Blocked,
}

/// 阶段定义 — 一张"任务卡"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: StageName,
    pub display_name: String,
    pub manual: String,
    pub tools_allowed: Vec<String>,
    pub tools_denied: Vec<String>,
    pub gate_type: GateType,
    pub gate_condition: Option<String>,
    pub next_stage: Option<StageName>,
    pub on_fail: Option<StageName>,
    pub runner: RunnerType,
    pub max_retries: u32,
    pub timeout_secs: Option<u64>,
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            name: StageName::new("unnamed"),
            display_name: String::new(),
            manual: String::new(),
            tools_allowed: Vec::new(),
            tools_denied: Vec::new(),
            gate_type: GateType::Auto,
            gate_condition: None,
            next_stage: None,
            on_fail: None,
            runner: RunnerType::Local,
            max_retries: 3,
            timeout_secs: Some(300),
        }
    }
}

/// 阶段运行实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInstance {
    pub stage_name: StageName,
    pub status: StageStatus,
    pub attempt: u32,
    pub started_at: Option<f64>,
    pub completed_at: Option<f64>,
    pub result: Option<serde_json::Value>,
    pub gate_result: Option<bool>,
    pub error: Option<String>,
}

impl StageInstance {
    pub fn new(stage_name: StageName) -> Self {
        Self {
            stage_name,
            status: StageStatus::Pending,
            attempt: 0,
            started_at: None,
            completed_at: None,
            result: None,
            gate_result: None,
            error: None,
        }
    }
}
```

```rust
// ─── wal.rs ────────────────────────────────────────

/// WAL 操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalAction {
    EngineInit,
    MemoInject,
    StageStart,
    StageComplete,
    GatePass,
    GateFail,
    Advance,
    ToolBlocked,
    HarnessComplete,
    StateSync,
}

/// WAL 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub timestamp: f64,
    pub action: WalAction,
    pub stage: String,
    pub data: serde_json::Value,
    pub checksum: String,
}

impl WalEntry {
    pub fn compute_checksum(&self) -> String {
        use blake3::Hasher;
        let content = format!(
            "{}:{}:{}:{}",
            self.timestamp,
            serde_json::to_string(&self.action).unwrap_or_default(),
            self.stage,
            self.data,
        );
        let mut hasher = Hasher::new();
        hasher.update(content.as_bytes());
        hasher.finalize().to_hex().to_string()[..16].to_string()
    }
}

/// WAL 管理器
pub struct WalManager {
    entries: Vec<WalEntry>,
    wal_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

impl WalManager {
    pub fn new(project_dir: &std::path::Path) -> Self {
        Self {
            entries: Vec::new(),
            wal_path: project_dir.join("harness.wal.json"),
            state_path: project_dir.join("harness.state.json"),
        }
    }

    /// 写入一条 WAL 条目并刷盘
    pub fn write(&mut self, action: WalAction, stage: &str, data: serde_json::Value) -> Result<()> {
        let mut entry = WalEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            action,
            stage: stage.to_string(),
            data,
            checksum: String::new(),
        };
        entry.checksum = entry.compute_checksum();
        self.entries.push(entry);
        self.flush()
    }

    /// 将 WAL 刷盘
    fn flush(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))?;
        std::fs::write(&self.wal_path, json)
            .map_err(|e| PensoulError::IoError(e.to_string()))?;
        Ok(())
    }

    /// 保存引擎状态快照
    pub fn save_state(&self, state: &EngineState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))?;
        std::fs::write(&self.state_path, json)
            .map_err(|e| PensoulError::IoError(e.to_string()))?;
        Ok(())
    }

    /// 从 WAL 文件加载所有条目
    pub fn load_entries(&self) -> Result<Vec<WalEntry>> {
        if !self.wal_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.wal_path)
            .map_err(|e| PensoulError::IoError(e.to_string()))?;
        let entries: Vec<WalEntry> = serde_json::from_str(&content)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))?;
        Ok(entries)
    }

    /// 验证 WAL 完整性
    pub fn verify_integrity(entries: &[WalEntry]) -> Result<Vec<WalEntry>> {
        let mut valid = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            if entry.checksum != entry.compute_checksum() {
                return Err(PensoulError::WalChecksumFailed { index: i });
            }
            valid.push(entry.clone());
        }
        Ok(valid)
    }
}
```

```rust
// ─── memo.rs ───────────────────────────────────────

/// 滚动备忘录 — 规划确认后，后续每个阶段都注入
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RollingMemo {
    entries: std::collections::HashMap<String, serde_json::Value>,
}

impl RollingMemo {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入一条备忘录
    pub fn inject(&mut self, key: &str, value: serde_json::Value) {
        self.entries.insert(key.to_string(), value);
    }

    /// 获取所有条目
    pub fn entries(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.entries
    }

    /// 生成注入给 AI 的文本上下文
    pub fn to_context_string(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut lines = vec!["## 创作方向备忘（已确认，不可偏离）".to_string()];
        for (k, v) in &self.entries {
            lines.push(format!("- {}: {}", k, v));
        }
        lines.join("\n")
    }
}
```

```rust
// ─── tools.rs ──────────────────────────────────────

/// 工具白名单检查器 — 硬性隔离
pub struct ToolWhitelist;

impl ToolWhitelist {
    /// 检查工具是否在当前阶段被允许
    pub fn check_access(
        stage: &Stage,
        tool_name: &str,
        wal: Option<&mut WalManager>,
        current_stage: &str,
    ) -> bool {
        // 被明确禁止 → 拒绝
        if stage.tools_denied.iter().any(|t| t == tool_name) {
            if let Some(wal) = wal {
                let _ = wal.write(
                    WalAction::ToolBlocked,
                    current_stage,
                    serde_json::json!({
                        "tool": tool_name,
                        "reason": "在工具白名单中被禁止"
                    }),
                );
            }
            return false;
        }
        // 如果有允许列表但工具不在其中 → 拒绝
        if !stage.tools_allowed.is_empty()
            && !stage.tools_allowed.iter().any(|t| t == tool_name)
        {
            if let Some(wal) = wal {
                let _ = wal.write(
                    WalAction::ToolBlocked,
                    current_stage,
                    serde_json::json!({
                        "tool": tool_name,
                        "reason": "不在允许列表中"
                    }),
                );
            }
            return false;
        }
        true
    }
}
```

```rust
// ─── gate.rs ───────────────────────────────────────

/// 门控判定器 — 确定性逻辑，AI 无权干预
pub struct GateEvaluator;

impl GateEvaluator {
    /// 执行门控判定
    pub fn evaluate(
        stage: &Stage,
        result: &serde_json::Value,
    ) -> bool {
        match stage.gate_type {
            GateType::Auto => true,

            GateType::Manual => {
                result
                    .get("human_approved")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }

            GateType::Conditional => {
                if let Some(ref condition) = stage.gate_condition {
                    Self::eval_condition(condition, result)
                } else {
                    // 默认：score >= 80 通过
                    result
                        .get("score")
                        .and_then(|v| v.as_f64())
                        .map(|s| s >= 80.0)
                        .unwrap_or(false)
                }
            }
        }
    }

    /// 安全的条件评估（沙箱）
    fn eval_condition(condition: &str, result: &serde_json::Value) -> bool {
        // 简单表达式解析：支持 "result.score >= 80" 模式
        if let Some(threshold) = condition
            .split(">=")
            .nth(1)
            .and_then(|s| s.trim().parse::<f64>().ok())
        {
            if let Some(score) = result.get("score").and_then(|v| v.as_f64()) {
                return score >= threshold;
            }
        }
        false
    }
}
```

```rust
// ─── engine.rs ─────────────────────────────────────

/// 引擎状态快照 — 用于持久化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub current_stage: Option<String>,
    pub memo: RollingMemo,
    pub stages_status: std::collections::HashMap<String, StageInstance>,
}

/// Harness 引擎核心
pub struct HarnessEngine {
    stages: std::collections::HashMap<String, Stage>,
    pub current_stage: Option<String>,
    stage_instances: std::collections::HashMap<String, StageInstance>,
    pub memo: RollingMemo,
    wal: WalManager,
}

impl HarnessEngine {
    /// 创建引擎实例
    pub fn new(project_dir: &std::path::Path) -> Result<Self> {
        std::fs::create_dir_all(project_dir)
            .map_err(|e| PensoulError::IoError(e.to_string()))?;
        Ok(Self {
            stages: std::collections::HashMap::new(),
            current_stage: None,
            stage_instances: std::collections::HashMap::new(),
            memo: RollingMemo::new(),
            wal: WalManager::new(project_dir),
        })
    }

    /// 注册一个阶段
    pub fn register_stage(&mut self, stage: Stage) {
        let name = stage.name.clone();
        self.stage_instances
            .insert(name.as_str().to_string(), StageInstance::new(name.clone()));
        self.stages.insert(name.as_str().to_string(), stage);
    }

    /// 设置起始阶段
    pub fn set_start_stage(&mut self, stage_name: &str) -> Result<()> {
        if !self.stages.contains_key(stage_name) {
            return Err(PensoulError::StageNotFound(stage_name.to_string()));
        }
        self.current_stage = Some(stage_name.to_string());
        self.wal.write(
            WalAction::EngineInit,
            stage_name,
            serde_json::json!({"start": true}),
        )
    }

    /// 注入滚动备忘录
    pub fn inject_memo(&mut self, key: &str, value: serde_json::Value) -> Result<()> {
        self.memo.inject(key, value.clone());
        let stage = self.current_stage.as_deref().unwrap_or("init");
        self.wal.write(
            WalAction::MemoInject,
            stage,
            serde_json::json!({ key: value }),
        )
    }

    /// 检查工具访问权限
    pub fn check_tool_access(&mut self, tool_name: &str) -> bool {
        let stage_name = match &self.current_stage {
            Some(name) => name.clone(),
            None => return false,
        };
        let stage = match self.stages.get(&stage_name) {
            Some(s) => s.clone(),
            None => return false,
        };
        ToolWhitelist::check_access(&stage, tool_name, Some(&mut self.wal), &stage_name)
    }

    /// 启动当前阶段
    pub fn start_stage(&mut self) -> Result<StageInstance> {
        let stage_name = self.current_stage.clone()
            .ok_or_else(|| PensoulError::Internal("未设置起始阶段".into()))?;
        let inst = self.stage_instances.get_mut(&stage_name)
            .ok_or_else(|| PensoulError::StageNotFound(stage_name.clone()))?;

        inst.status = StageStatus::Running;
        inst.attempt += 1;
        inst.started_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        );

        self.wal.write(
            WalAction::StageStart,
            &stage_name,
            serde_json::json!({"attempt": inst.attempt}),
        )?;

        Ok(inst.clone())
    }

    /// 阶段执行完成，进入门控判定
    pub fn complete_stage(&mut self, result: serde_json::Value) -> Result<()> {
        let stage_name = self.current_stage.clone()
            .ok_or_else(|| PensoulError::Internal("当前阶段为空".into()))?;
        let stage = self.stages.get(&stage_name)
            .cloned()
            .ok_or_else(|| PensoulError::StageNotFound(stage_name.clone()))?;
        let inst = self.stage_instances.get_mut(&stage_name)
            .ok_or_else(|| PensoulError::StageNotFound(stage_name.clone()))?;

        inst.result = Some(result.clone());
        inst.status = StageStatus::WaitingGate;

        self.wal.write(
            WalAction::StageComplete,
            &stage_name,
            serde_json::json!({"result_summary": result}),
        )?;

        // 引擎确定性地执行门控判定
        let gate_passed = GateEvaluator::evaluate(&stage, &result);
        inst.gate_result = Some(gate_passed);

        if gate_passed {
            self.wal.write(WalAction::GatePass, &stage_name, serde_json::json!({}))?;
            inst.status = StageStatus::Completed;
            inst.completed_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64(),
            );
            self.advance_to_next(&stage)?;
        } else {
            self.wal.write(
                WalAction::GateFail,
                &stage_name,
                serde_json::json!({"retries_remaining": stage.max_retries - inst.attempt}),
            )?;
            if inst.attempt >= stage.max_retries {
                inst.status = StageStatus::Failed;
                inst.error = Some(format!("超过最大重试次数 {}", stage.max_retries));
            } else if let Some(ref on_fail) = stage.on_fail {
                inst.status = StageStatus::Pending;
                self.current_stage = Some(on_fail.as_str().to_string());
            } else {
                inst.status = StageStatus::Blocked;
            }
        }

        // 保存状态
        let state = self.build_state();
        self.wal.save_state(&state)?;
        Ok(())
    }

    fn advance_to_next(&mut self, stage: &Stage) -> Result<()> {
        if let Some(ref next) = stage.next_stage {
            self.current_stage = Some(next.as_str().to_string());
            self.wal.write(
                WalAction::Advance,
                stage.name.as_str(),
                serde_json::json!({"next": next.as_str()}),
            )
        } else {
            self.current_stage = None;
            self.wal.write(
                WalAction::HarnessComplete,
                stage.name.as_str(),
                serde_json::json!({"done": true}),
            )
        }
    }

    fn build_state(&self) -> EngineState {
        EngineState {
            current_stage: self.current_stage.clone(),
            memo: self.memo.clone(),
            stages_status: self.stage_instances.clone(),
        }
    }

    /// 崩溃恢复 — 从 WAL 重建状态
    pub fn recover_from_crash(&mut self) -> Result<bool> {
        let entries = self.wal.load_entries()?;
        if entries.is_empty() {
            return Ok(false);
        }

        // 验证 WAL 完整性
        let entries = WalManager::verify_integrity(&entries)?;

        let mut recovered_stage: Option<String> = None;
        let mut recovered_memo = RollingMemo::new();

        for entry in &entries {
            match entry.action {
                WalAction::EngineInit => {
                    recovered_stage = Some(entry.stage.clone());
                }
                WalAction::MemoInject => {
                    if let Some(obj) = entry.data.as_object() {
                        for (k, v) in obj {
                            recovered_memo.inject(k, v.clone());
                        }
                    }
                }
                WalAction::StageComplete => {
                    if let Some(inst) = self.stage_instances.get_mut(&entry.stage) {
                        inst.status = StageStatus::Completed;
                    }
                }
                WalAction::GateFail => {
                    if let Some(inst) = self.stage_instances.get_mut(&entry.stage) {
                        inst.status = StageStatus::Pending;
                        recovered_stage = entry.data.get("next_stage")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or(Some(entry.stage.clone()));
                    }
                }
                _ => {}
            }
        }

        self.current_stage = recovered_stage;
        self.memo = recovered_memo;
        self.wal = WalManager::new(
            self.wal.wal_path.parent().unwrap_or(std::path::Path::new(".")),
        );

        Ok(true)
    }
}
```

## 3.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 7 | 阶段状态机确定性流转 | 单元测试 | AI 无法跳步，引擎唯一控制推进 |
| 8 | 自动放行（GateType::Auto） | 单元测试 | 完成后直接进入 next_stage |
| 9 | 人工放行（GateType::Manual） | 单元测试 | human_approved=false 时阻塞，true 时放行 |
| 10 | 条件放行（GateType::Conditional） | 单元测试 | score >= 80 通过，< 80 回退到 on_fail |
| 11 | 工具白名单 — 显式禁止 | 单元测试 | denied 列表中的工具返回 false |
| 12 | 工具白名单 — 不在允许列表 | 单元测试 | 非 allowed 列表中的工具返回 false |
| 13 | WAL 写入与刷盘 | 文件系统测试 | WAL 文件存在且内容正确 |
| 14 | WAL 校验和验证 | 单元测试 | 篡改条目后 checksum 检测失败 |
| 15 | 崩溃恢复 — WAL 重放 | 集成测试 | 恢复后 current_stage 和 memo 与崩溃前一致 |
| 16 | 滚动备忘录跨阶段注入 | 集成测试 | 后续阶段通过 get_memo_context 读取到注入内容 |
| 17 | 最大重试次数拦截 | 单元测试 | attempt >= max_retries 时状态变为 Failed |
| 18 | 引擎状态快照持久化 | 文件测试 | state.json 文件包含完整的 stages_status |

---

# 第四章：pensoul-cda — 一致性驱动架构

## 4.1 职责说明

实现影响图（基于 petgraph）、变更传播、联动修改建议。核心功能：当用户修改第 N 章时，自动找出所有受影响的章节和实体，按直接/间接/级联分级。

## 4.2 文件结构

```
crates/pensoul-cda/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── graph.rs                # 影响图核心
│   ├── node.rs                 # 节点类型
│   ├── edge.rs                 # 边类型
│   ├── propagation.rs          # 变更传播算法
│   ├── suggestion.rs           # 联动建议生成
│   └── stats.rs                # 图统计
```

## 4.3 核心数据类型

```rust
// ─── node.rs ───────────────────────────────────────

use pensoul_core::{ChapterId, EntityId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 影响图节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Entity,
    Event,
    Setting,
    Foreshadow,
    Relationship,
    Knowledge,
}

/// 影响严重度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    /// 直接影响 — 同章节、紧邻章节（2章以内）
    Direct,
    /// 间接影响 — 通过引用链传播
    Indirect,
    /// 级联影响 — 触发重新评估
    Cascading,
}

/// 影响图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    pub id: String,
    pub node_type: NodeType,
    pub chapter_id: i64,
    pub content_hash: String,
    pub severity: ImpactSeverity,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
```

```rust
// ─── edge.rs ───────────────────────────────────────

/// 边关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    References,
    Contradicts,
    DependsOn,
    Causes,
    Modifies,
}

/// 影响图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEdge {
    pub from_id: String,
    pub to_id: String,
    pub relation: EdgeRelation,
    pub weight: f32, // 影响强度 0.0-1.0
}
```

```rust
// ─── graph.rs ──────────────────────────────────────

/// 受影响项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedItem {
    pub node_id: String,
    pub chapter_id: i64,
    pub severity: ImpactSeverity,
    pub reason: String,
    pub suggested_action: String,
}

/// 影响图 — CDA 架构核心，基于 petgraph
pub struct ImpactGraph {
    graph: petgraph::graph::DiGraph<ImpactNode, ImpactEdge>,
    node_indices: std::collections::HashMap<String, petgraph::graph::NodeIndex>,
}

impl ImpactGraph {
    pub fn new() -> Self {
        Self {
            graph: petgraph::graph::DiGraph::new(),
            node_indices: std::collections::HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: ImpactNode) {
        let idx = self.graph.add_node(node.clone());
        self.node_indices.insert(node.id.clone(), idx);
    }

    /// 添加边
    pub fn add_edge(&mut self, edge: ImpactEdge) -> Result<()> {
        let from_idx = self.node_indices.get(&edge.from_id)
            .ok_or_else(|| PensoulError::Internal(format!("源节点 {} 不存在", edge.from_id)))?;
        let to_idx = self.node_indices.get(&edge.to_id)
            .ok_or_else(|| PensoulError::Internal(format!("目标节点 {} 不存在", edge.to_id)))?;
        self.graph.add_edge(*from_idx, *to_idx, edge);
        Ok(())
    }

    /// 查找受变更影响的所有节点（BFS + 深度限制 + 环检测）
    pub fn find_affected(
        &self,
        source_chapter: i64,
        changed_entity_ids: &[String],
        max_depth: u32,
    ) -> Vec<AffectedItem> {
        propagation::bfs_find_affected(
            &self.graph,
            &self.node_indices,
            source_chapter,
            changed_entity_ids,
            max_depth,
        )
    }

    /// 获取图统计信息
    pub fn stats(&self) -> Stats {
        stats::compute_stats(&self.graph)
    }
}
```

```rust
// ─── propagation.rs ────────────────────────────────

use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet, VecDeque};
use super::{ImpactNode, ImpactEdge, AffectedItem, ImpactSeverity};

pub(crate) fn bfs_find_affected(
    graph: &DiGraph<ImpactNode, ImpactEdge>,
    node_indices: &HashMap<String, NodeIndex>,
    source_chapter: i64,
    changed_entity_ids: &[String],
    max_depth: u32,
) -> Vec<AffectedItem> {
    let mut affected = Vec::new();
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<(u32, NodeIndex, ImpactSeverity, String)> = VecDeque::new();

    // 第一步：找到所有直接关联节点
    for entity_id in changed_entity_ids {
        if let Some(&idx) = node_indices.get(entity_id) {
            // 反向遍历：谁引用了这个实体
            for neighbor in graph.neighbors_directed(idx, petgraph::Direction::Incoming) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let node = &graph[neighbor];
                    let sev = if (node.chapter_id - source_chapter).abs() <= 2 {
                        ImpactSeverity::Direct
                    } else {
                        ImpactSeverity::Indirect
                    };
                    queue.push_back((0, neighbor, sev, format!("通过引用关联到 {}", entity_id)));
                }
            }
        }
    }

    // 第二步：BFS 传播
    while let Some((depth, idx, base_severity, reason)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let node = &graph[idx];
        let item_severity = if depth == 0 {
            base_severity
        } else if depth <= 2 {
            ImpactSeverity::Indirect
        } else {
            ImpactSeverity::Cascading
        };

        affected.push(AffectedItem {
            node_id: node.id.clone(),
            chapter_id: node.chapter_id,
            severity: item_severity,
            reason: reason.clone(),
            suggested_action: suggest_action(node.node_type, item_severity),
        });

        for neighbor in graph.neighbors_directed(idx, petgraph::Direction::Outgoing) {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                let edge = graph.find_edge(idx, neighbor);
                let edge_weight = edge
                    .and_then(|e| graph.edge_weight(e))
                    .map(|e| e.weight)
                    .unwrap_or(0.5);
                queue.push_back((
                    depth + 1,
                    neighbor,
                    item_severity,
                    format!("通过 {} → {} 传播", node.id, graph[neighbor].id),
                ));
            }
        }
    }

    // 按章节和严重度排序
    affected.sort_by(|a, b| {
        a.chapter_id
            .cmp(&b.chapter_id)
            .then_with(|| severity_rank(a.severity).cmp(&severity_rank(b.severity)))
    });

    affected
}

fn severity_rank(s: ImpactSeverity) -> u8 {
    match s {
        ImpactSeverity::Direct => 0,
        ImpactSeverity::Indirect => 1,
        ImpactSeverity::Cascading => 2,
    }
}

fn suggest_action(node_type: super::NodeType, severity: ImpactSeverity) -> String {
    let base = match node_type {
        super::NodeType::Entity => "检查该实体的状态描述是否与修改后一致",
        super::NodeType::Event => "检查事件的时间线和因果链是否受影响",
        super::NodeType::Setting => "检查世界观设定文档是否需要更新",
        super::NodeType::Foreshadow => "检查伏笔的埋设/推进/回收状态",
        super::NodeType::Relationship => "检查角色关系描述是否需要调整",
        super::NodeType::Knowledge => "检查角色知识库是否需要更新",
    };
    match severity {
        ImpactSeverity::Direct => format!("🔴 必须修改: {}", base),
        ImpactSeverity::Indirect => format!("🟡 建议检查: {}", base),
        ImpactSeverity::Cascading => format!("🟢 可能受影响: {}", base),
    }
}
```

```rust
// ─── stats.rs ──────────────────────────────────────

use petgraph::graph::DiGraph;
use super::{ImpactNode, ImpactEdge};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub chapters: usize,
    pub avg_edges_per_node: f32,
}

pub(crate) fn compute_stats(graph: &DiGraph<ImpactNode, ImpactEdge>) -> Stats {
    let total_nodes = graph.node_count();
    let total_edges = graph.edge_count();
    let mut nodes_by_type: HashMap<String, usize> = HashMap::new();
    let mut chapter_set: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for node in graph.node_indices() {
        let n = &graph[node];
        *nodes_by_type.entry(format!("{:?}", n.node_type)).or_insert(0) += 1;
        chapter_set.insert(n.chapter_id);
    }

    Stats {
        total_nodes,
        total_edges,
        nodes_by_type,
        chapters: chapter_set.len(),
        avg_edges_per_node: if total_nodes > 0 {
            total_edges as f32 / total_nodes as f32
        } else {
            0.0
        },
    }
}
```

## 4.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 19 | 200 章影响图构建性能 | 基准测试 | 1020 节点 / 1800 边，构建 < 50ms |
| 20 | 变更传播查询性能 | 基准测试 | 1000 章规模，单次查询 < 5ms |
| 21 | 影响分级 — 直接影响 | 单元测试 | 紧邻章节（2章内）标记为 Direct |
| 22 | 影响分级 — 间接影响 | 单元测试 | 通过引用链传播的标记为 Indirect |
| 23 | 影响分级 — 级联影响 | 单元测试 | 深度 > 2 的传播标记为 Cascading |
| 24 | BFS 环检测 | 单元测试 | 含环图不会无限循环 |
| 25 | 联动建议生成 | 单元测试 | 每个 AffectedItem 含非空 suggested_action |

---

# 第五章：pensoul-memory — 四层记忆系统

## 5.1 职责说明

实现四层记忆架构（热/温/冷/冰），记忆包构建，叙事记忆，三种编辑模式（drafting/revising/reviewing）的预算分配。设计原则：效果优先，不计 token 预算上限。

## 5.2 文件结构

```
crates/pensoul-memory/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── hot.rs                  # 热记忆
│   ├── warm.rs                 # 温记忆
│   ├── cold.rs                 # 冷记忆（向量检索）
│   ├── archive.rs              # 冰记忆
│   ├── packet.rs               # 记忆包构建
│   ├── narrative.rs            # 叙事记忆
│   └── pipeline.rs             # 记忆更新管道
```

## 5.3 核心数据类型

```rust
// ─── packet.rs ─────────────────────────────────────

use pensoul_core::{ChapterId, CharacterId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 编辑模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditingMode {
    Drafting,
    Revising,
    Reviewing,
}

/// 记忆层预算分配比例
#[derive(Debug, Clone)]
struct BudgetRatio {
    hot: f32,
    warm: f32,
    cold: f32,
    narrative: f32,
}

fn get_budget_ratio(mode: EditingMode) -> BudgetRatio {
    match mode {
        EditingMode::Drafting => BudgetRatio { hot: 0.50, warm: 0.25, cold: 0.20, narrative: 0.05 },
        EditingMode::Revising => BudgetRatio { hot: 0.60, warm: 0.20, cold: 0.15, narrative: 0.05 },
        EditingMode::Reviewing => BudgetRatio { hot: 0.30, warm: 0.20, cold: 0.40, narrative: 0.10 },
    }
}

/// 章节摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub chapter_id: ChapterId,
    pub title: String,
    pub summary: String,
    pub key_events: Vec<String>,
    pub character_states: std::collections::HashMap<CharacterId, serde_json::Value>,
    pub word_count: usize,
    pub consistency_score: f32,
    #[serde(default)]
    pub semantic_embedding: Vec<f32>,
}

/// 叙事记忆 — 大纲之外的关键细节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeDetail {
    pub detail_id: String,
    pub chapter_id: ChapterId,
    pub category: NarrativeCategory,
    pub content: String,
    pub importance: f32,
    pub last_referenced: ChapterId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeCategory {
    Habit,
    Promise,
    Prop,
    Sensory,
    Subplot,
}

/// 注入给 LLM 的记忆包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPacket {
    /// 热记忆：完整文本
    pub hot: Vec<String>,
    /// 温记忆：结构化摘要
    pub warm: WarmMemoryData,
    /// 冷记忆：检索到的相关内容
    pub cold: Vec<String>,
    /// 叙事记忆
    pub narrative: Vec<NarrativeDetail>,
    /// 总 token 估算
    pub total_tokens: usize,
    /// 预算使用率
    pub budget_used: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WarmMemoryData {
    pub volume_summary: Option<String>,
    pub active_foreshadows: Option<String>,
    pub character_states: Option<String>,
}
```

```rust
// ─── hot.rs ────────────────────────────────────────

/// 热记忆管理器 — 当前章节 ± 前后 N 章，完整文本
pub struct HotMemory {
    full_texts: std::collections::HashMap<i64, String>,
    window_size: i64,
}

impl HotMemory {
    pub fn new(window_size: i64) -> Self {
        Self {
            full_texts: std::collections::HashMap::new(),
            window_size,
        }
    }

    pub fn insert(&mut self, chapter_id: i64, text: String) {
        self.full_texts.insert(chapter_id, text);
    }

    /// 构建热记忆文本列表
    pub fn build(&self, current_chapter: i64, budget: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut used = 0;
        for offset in (-self.window_size..=0).rev() {
            let ch_id = current_chapter + offset;
            if let Some(text) = self.full_texts.get(&ch_id) {
                let est_tokens = text.len() / 2; // 中文约 2 字 = 1 token
                if used + est_tokens > budget {
                    break;
                }
                let label = if offset == 0 {
                    "[当前章]"
                } else if offset == -1 {
                    "[前一章]"
                } else {
                    "[前前章]"
                };
                result.push(format!("{}\n{}", label, text));
                used += est_tokens;
            }
        }
        result
    }
}
```

```rust
// ─── warm.rs ───────────────────────────────────────

use super::{ChapterSummary, WarmMemoryData, NarrativeDetail};

/// 温记忆管理器 — 当前卷摘要 + 角色状态 + 活跃伏笔
pub struct WarmMemory {
    chapters: std::collections::HashMap<i64, ChapterSummary>,
    active_foreshadows: Vec<serde_json::Value>,
    character_states: std::collections::HashMap<String, serde_json::Value>,
}

impl WarmMemory {
    pub fn new() -> Self {
        Self {
            chapters: std::collections::HashMap::new(),
            active_foreshadows: Vec::new(),
            character_states: std::collections::HashMap::new(),
        }
    }

    pub fn insert_chapter(&mut self, chapter: ChapterSummary) {
        self.chapters.insert(chapter.chapter_id.inner(), chapter);
    }

    pub fn set_foreshadows(&mut self, foreshadows: Vec<serde_json::Value>) {
        self.active_foreshadows = foreshadows;
    }

    pub fn set_character_states(&mut self, states: std::collections::HashMap<String, serde_json::Value>) {
        self.character_states = states;
    }

    /// 构建温记忆数据
    pub fn build(&self, current_chapter: i64, budget: usize) -> WarmMemoryData {
        let mut used = 0;
        let mut data = WarmMemoryData::default();

        // 卷摘要
        let vol_summary = self.build_volume_summary(current_chapter);
        let tokens = vol_summary.len() / 2;
        if used + tokens <= budget {
            data.volume_summary = Some(vol_summary);
            used += tokens;
        }

        // 活跃伏笔
        let fs_text = self.build_foreshadow_text();
        let tokens = fs_text.len() / 2;
        if used + tokens <= budget {
            data.active_foreshadows = Some(fs_text);
            used += tokens;
        }

        // 角色状态
        let cs_text = serde_json::to_string_pretty(&self.character_states)
            .unwrap_or_default();
        let tokens = cs_text.len() / 2;
        if used + tokens <= budget {
            data.character_states = Some(cs_text);
        }

        data
    }

    fn build_volume_summary(&self, current_chapter: i64) -> String {
        let mut recent: Vec<_> = self.chapters.values()
            .filter(|c| c.chapter_id.inner() >= current_chapter - 20 && c.chapter_id.inner() < current_chapter)
            .collect();
        recent.sort_by_key(|c| c.chapter_id.inner());
        recent.iter().rev().take(10)
            .map(|c| format!("第{}章: {}...", c.chapter_id, &c.summary[..c.summary.len().min(80)]))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn build_foreshadow_text(&self) -> String {
        self.active_foreshadows.iter()
            .filter(|f| f.get("status").and_then(|s| s.as_str()) != Some("resolved"))
            .take(8)
            .map(|f| {
                let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let desc = f.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let ch = f.get("planted_chapter").and_then(|v| v.as_i64()).unwrap_or(0);
                format!("- {}: {} (埋设于第{}章)", name, desc, ch)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

```rust
// ─── cold.rs ───────────────────────────────────────

/// 冷记忆管理器 — 向量数据库检索（LanceDB）
pub struct ColdMemory {
    // 实际产品中持有 LanceDB 连接
    // 原型阶段使用简单关键词匹配
    chapters: std::collections::HashMap<i64, super::ChapterSummary>,
}

impl ColdMemory {
    pub fn new() -> Self {
        Self {
            chapters: std::collections::HashMap::new(),
        }
    }

    pub fn insert_chapter(&mut self, chapter: super::ChapterSummary) {
        self.chapters.insert(chapter.chapter_id.inner(), chapter);
    }

    /// 检索相关章节摘要
    pub fn retrieve(&self, current_chapter: i64, budget: usize) -> Vec<String> {
        let mut results = Vec::new();
        let mut used = 0;
        for (ch_id, chapter) in &self.chapters {
            if *ch_id == current_chapter || (*ch_id - current_chapter).abs() <= 2 {
                continue; // 已在热记忆中
            }
            let text = format!("[第{}章摘要] {}", ch_id, chapter.summary);
            let tokens = text.len() / 2;
            if used + tokens <= budget {
                results.push(text);
                used += tokens;
            }
        }
        results
    }
}
```

```rust
// ─── narrative.rs ──────────────────────────────────

use super::{NarrativeDetail, NarrativeCategory};

/// 叙事记忆管理器
pub struct NarrativeMemory {
    details: Vec<NarrativeDetail>,
}

impl NarrativeMemory {
    pub fn new() -> Self {
        Self { details: Vec::new() }
    }

    pub fn add_detail(&mut self, detail: NarrativeDetail) {
        self.details.push(detail);
    }

    /// 检索相关的叙事记忆
    pub fn retrieve(&self, current_chapter: i64, budget: usize) -> Vec<NarrativeDetail> {
        let mut relevant: Vec<_> = self.details.iter()
            .filter(|d| d.chapter_id.inner() < current_chapter && d.importance > 0.5)
            .cloned()
            .collect();
        relevant.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));

        let mut result = Vec::new();
        let mut used = 0;
        for detail in relevant {
            let tokens = detail.content.len() / 2;
            if used + tokens <= budget {
                used += tokens;
                result.push(detail);
            }
        }
        result
    }
}
```

```rust
// ─── pipeline.rs ───────────────────────────────────

/// 记忆更新管道 — 每完成一个章节自动触发
pub struct MemoryPipeline;

impl MemoryPipeline {
    /// 执行 8 步记忆更新
    pub fn update(
        chapter_id: i64,
        chapter_text: &str,
        // ... 其他参数
    ) -> Result<()> {
        // 1. 提取新叙事要素（实体、事件、关系变化）
        // 2. 更新角色状态机
        // 3. 更新时间线
        // 4. 更新伏笔状态（planted → progressing → resolved）
        // 5. 重新生成章节摘要
        // 6. 更新一致性向量
        // 7. 更新影响图
        // 8. 更新叙事记忆
        todo!("实现记忆更新管道")
    }
}
```

## 5.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 26 | 热记忆窗口 ± 2 章完整文本 | 单元测试 | 返回当前章和前2章的完整文本 |
| 27 | 温记忆全量注入（无裁剪） | 单元测试 | 角色状态、伏笔、卷摘要全部包含 |
| 28 | 冷记忆向量检索 Top-K | 单元测试 | 排除热记忆窗口内的章节 |
| 29 | 叙事记忆按重要性排序 | 单元测试 | importance > 0.5 的记录被检索 |
| 30 | 三种编辑模式预算分配 | 单元测试 | drafting/warm=0.25, reviewing/cold=0.40 |
| 31 | 500 章大规模记忆包构建性能 | 基准测试 | 平均构建耗时 < 50ms |
| 32 | Token 预算不超限 | 单元测试 | total_tokens <= budget * 1.1 |
| 33 | 记忆更新管道完整性 | 集成测试 | 8 步更新全部执行，无遗漏 |

---

# 第六章：pensoul-agent — 智能体通信

## 6.1 职责说明

实现双通道通信协议（signal/report）、6 个预置 Agent 定义、通道路由器。信号通道仅引擎可见（结构化 JSON），文本通道仅用户可见（自然语言报告）。

## 6.2 文件结构

```
crates/pensoul-agent/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── message.rs              # 消息类型
│   ├── channel.rs              # 双通道
│   ├── router.rs               # 通道路由器
│   ├── agents.rs               # 6 个预置 Agent
│   └── protocol.rs             # JSON Schema
```

## 6.3 核心数据类型

```rust
// ─── message.rs ────────────────────────────────────

use pensoul_core::{AgentId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 通道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Signal,
    Report,
}

/// Agent 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub msg_id: String,
    pub channel: ChannelType,
    pub from_agent: AgentId,
    pub to_agent: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    #[serde(default)]
    pub metadata: MessageMetadata,
    pub timestamp: f64,
}

/// 信号通道载荷 — 仅引擎读取
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPayload {
    pub pass: bool,
    pub score: Option<f32>,
    pub severity_levels: Option<SeverityLevels>,
    pub retry: bool,
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeverityLevels {
    pub critical: u32,
    pub warning: u32,
    pub info: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub model_used: Option<String>,
    pub duration_ms: Option<u64>,
}
```

```rust
// ─── router.rs ─────────────────────────────────────

type Handler = Box<dyn Fn(&AgentMessage) -> Result<()> + Send + Sync>;

/// 通道路由器 — signal 和 report 分离
pub struct ChannelRouter {
    signal_handlers: std::collections::HashMap<String, Handler>,
    report_handlers: std::collections::HashMap<String, Handler>,
    message_log: Vec<AgentMessage>,
}

impl ChannelRouter {
    pub fn new() -> Self {
        Self {
            signal_handlers: std::collections::HashMap::new(),
            report_handlers: std::collections::HashMap::new(),
            message_log: Vec::new(),
        }
    }

    /// 注册信号处理器（引擎端）
    pub fn register_signal_handler<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.signal_handlers.insert(agent_id.to_string(), Box::new(handler));
    }

    /// 注册报告处理器（UI 端）
    pub fn register_report_handler<F>(&mut self, agent_id: &str, handler: F)
    where
        F: Fn(&AgentMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.report_handlers.insert(agent_id.to_string(), Box::new(handler));
    }

    /// 发送消息 — 根据 channel 类型路由到对应处理器
    pub fn send(&mut self, msg: AgentMessage) -> Result<()> {
        self.message_log.push(msg.clone());
        match msg.channel {
            ChannelType::Signal => {
                if let Some(handler) = self.signal_handlers.get(msg.to_agent.as_str()) {
                    handler(&msg)
                } else {
                    Err(PensoulError::Internal(
                        format!("信号处理器未注册: {}", msg.to_agent)
                    ))
                }
            }
            ChannelType::Report => {
                if let Some(handler) = self.report_handlers.get(msg.to_agent.as_str()) {
                    handler(&msg)
                } else {
                    Err(PensoulError::Internal(
                        format!("报告处理器未注册: {}", msg.to_agent)
                    ))
                }
            }
        }
    }

    pub fn get_signal_messages(&self) -> Vec<&AgentMessage> {
        self.message_log.iter()
            .filter(|m| m.channel == ChannelType::Signal)
            .collect()
    }

    pub fn get_report_messages(&self) -> Vec<&AgentMessage> {
        self.message_log.iter()
            .filter(|m| m.channel == ChannelType::Report)
            .collect()
    }
}
```

```rust
// ─── agents.rs ─────────────────────────────────────

/// 预置 Agent 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// 一致性审查
    ConsistencyAuditor,
    /// 文风校准
    StyleAnalyzer,
    /// 伏笔追踪
    ForeshadowTracker,
    /// 对话打磨
    DialoguePolisher,
    /// 大纲规划
    PlotArchitect,
    /// 世界观构建
    WorldBuilder,
}

/// Agent 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub agent_type: AgentType,
    pub agent_id: AgentId,
    pub display_name: String,
    pub description: String,
    pub model_preference: String,
    pub tools_allowed: Vec<String>,
    pub signal_fields: Vec<String>,
    pub system_prompt: String,
}

impl AgentDefinition {
    pub fn preset(agent_type: AgentType) -> Self {
        match agent_type {
            AgentType::ConsistencyAuditor => Self {
                agent_type,
                agent_id: AgentId::new("consistency_auditor"),
                display_name: "一致性审查员".into(),
                description: "独立审查实体状态、时间线、角色行为一致性".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec![
                    "read_chapter".into(),
                    "read_character_state".into(),
                    "read_consistency_vector".into(),
                    "run_consistency_check".into(),
                ],
                signal_fields: vec!["pass".into(), "score".into(), "issues".into()],
                system_prompt: "你是一个严格的一致性审查专家。".into(),
            },
            AgentType::StyleAnalyzer => Self {
                agent_type,
                agent_id: AgentId::new("style_analyzer"),
                display_name: "文风分析师".into(),
                description: "校准文风、反AI味检查".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_style_guide".into()],
                signal_fields: vec!["pass".into(), "style_score".into()],
                system_prompt: "你是一个专业的文风分析专家。".into(),
            },
            AgentType::ForeshadowTracker => Self {
                agent_type,
                agent_id: AgentId::new("foreshadow_tracker"),
                display_name: "伏笔追踪员".into(),
                description: "追踪伏笔的埋设、推进和回收".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_foreshadows".into()],
                signal_fields: vec!["status_update".into(), "alerts".into()],
                system_prompt: "你是一个伏笔管理专家。".into(),
            },
            AgentType::DialoguePolisher => Self {
                agent_type,
                agent_id: AgentId::new("dialogue_polisher"),
                display_name: "对话打磨师".into(),
                description: "打磨对话质量，确保角色语言个性化".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_character_style".into()],
                signal_fields: vec!["pass".into(), "quality_score".into()],
                system_prompt: "你是一个对话写作专家。".into(),
            },
            AgentType::PlotArchitect => Self {
                agent_type,
                agent_id: AgentId::new("plot_architect"),
                display_name: "大纲架构师".into(),
                description: "规划多层大纲、伏笔地图、角色弧线".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_memory".into(), "generate_outline".into()],
                signal_fields: vec!["outline_proposal".into()],
                system_prompt: "你是一个专业的小说大纲架构师。".into(),
            },
            AgentType::WorldBuilder => Self {
                agent_type,
                agent_id: AgentId::new("world_builder"),
                display_name: "世界观构建师".into(),
                description: "构建一致的世界观设定".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_world".into(), "generate_world_spec".into()],
                signal_fields: vec!["world_spec".into()],
                system_prompt: "你是一个专业的小说世界观构建师。".into(),
            },
        }
    }
}
```

## 6.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 34 | signal 通道路由到引擎 | 单元测试 | 审查信号正确到达 signal_handlers |
| 35 | report 通道路由到 UI | 单元测试 | 用户报告正确到达 report_handlers |
| 36 | JSON 序列化/反序列化 | round-trip 测试 | 枚举类型 ChannelType 正确转换 |
| 37 | 多消息路由 | 集成测试 | 6 条信号正确路由到目标处理器 |
| 38 | 6 个预置 Agent 定义完整 | 代码审查 | 每个 Agent 有 agent_id、tools_allowed、signal_fields |
| 39 | 信号与报告通道隔离 | 审查测试 | signal 消息不会被 report_handler 接收 |

---

# 第七章：pensoul-concurrency — 并发控制

## 7.1 职责说明

实现乐观锁、操作队列、冲突检测与合并。解决用户手动编辑与 AI 后台生成同时修改共享数据的问题。

## 7.2 文件结构

```
crates/pensoul-concurrency/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── lock.rs                 # 章节级锁
│   ├── queue.rs                # 操作队列
│   ├── conflict.rs             # 冲突检测与合并
│   └── version.rs              # 版本管理
```

## 7.3 核心数据类型

```rust
// ─── lock.rs ───────────────────────────────────────

use pensoul_core::{ChapterId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    UserEdit,
    AiGenerate,
    AiRevision,
    SystemImport,
}

/// 章节版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    pub chapter_id: i64,
    pub version: i32,
    pub checksum: String,
    pub last_modified_by: String,
    pub last_modified_at: f64,
}

/// 操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op_id: String,
    pub op_type: OperationType,
    pub chapter_id: i64,
    pub content: String,
    pub expected_version: i32,
    pub timestamp: f64,
    pub status: OperationStatus,
    pub actual_version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Applied,
    Conflict,
    Rejected,
}

/// 并发控制器
pub struct ConcurrencyController {
    versions: std::collections::HashMap<i64, ChapterVersion>,
    operation_log: Vec<Operation>,
    lock: std::sync::Mutex<()>,
}

impl ConcurrencyController {
    pub fn new() -> Self {
        Self {
            versions: std::collections::HashMap::new(),
            operation_log: Vec::new(),
            lock: std::sync::Mutex::new(()),
        }
    }

    /// 注册章节
    pub fn register_chapter(&mut self, chapter_id: i64, initial_content: &str) {
        let checksum = blake3::hash(initial_content.as_bytes()).to_hex().to_string()[..8].to_string();
        self.versions.insert(chapter_id, ChapterVersion {
            chapter_id,
            version: 1,
            checksum,
            last_modified_by: "system".into(),
            last_modified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        });
    }

    /// 获取章节当前版本号
    pub fn get_version(&self, chapter_id: i64) -> i32 {
        self.versions.get(&chapter_id).map(|v| v.version).unwrap_or(0)
    }

    /// 提交操作 — 乐观锁检测
    pub fn submit_operation(&mut self, mut op: Operation) -> Operation {
        let _guard = self.lock.lock().unwrap();

        let current = match self.versions.get(&op.chapter_id) {
            Some(v) => v.clone(),
            None => {
                op.status = OperationStatus::Rejected;
                self.operation_log.push(op.clone());
                return op;
            }
        };

        // 乐观锁：版本号不匹配 → 冲突
        if current.version != op.expected_version {
            op.status = OperationStatus::Conflict;
            op.actual_version = current.version;
            self.operation_log.push(op.clone());
            return op;
        }

        // 版本号匹配 → 应用
        let new_version = current.version + 1;
        let checksum = blake3::hash(op.content.as_bytes()).to_hex().to_string()[..8].to_string();
        self.versions.insert(op.chapter_id, ChapterVersion {
            chapter_id: op.chapter_id,
            version: new_version,
            checksum,
            last_modified_by: op.op_type.to_string(),
            last_modified_at: op.timestamp,
        });

        op.status = OperationStatus::Applied;
        op.actual_version = new_version;
        self.operation_log.push(op.clone());
        op
    }

    /// 获取章节锁状态
    pub fn get_chapter_lock(&self, chapter_id: i64) -> ChapterVersion {
        self.versions.get(&chapter_id).cloned().unwrap_or(ChapterVersion {
            chapter_id,
            version: 0,
            checksum: String::new(),
            last_modified_by: String::new(),
            last_modified_at: 0.0,
        })
    }

    /// 获取待处理操作
    pub fn get_pending_ops(&self) -> Vec<&Operation> {
        self.operation_log.iter()
            .filter(|op| op.status == OperationStatus::Pending)
            .collect()
    }
}
```

```rust
// ─── conflict.rs ───────────────────────────────────

/// 冲突检测与合并
pub struct ConflictResolver;

impl ConflictResolver {
    /// 检测两个操作是否冲突（修改区域是否重叠）
    pub fn detect_conflict(op_a: &Operation, op_b: &Operation) -> bool {
        op_a.chapter_id == op_b.chapter_id
    }

    /// 合并两个不重叠的修改
    pub fn merge(content_a: &str, content_b: &str) -> Result<String> {
        // 简化实现：直接拼接
        Ok(format!("{}\n\n{}", content_a, content_b))
    }
}
```

## 7.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 40 | 章节级乐观锁 — 版本匹配 | 单元测试 | expected_version 匹配时操作成功 |
| 41 | 章节级乐观锁 — 版本冲突 | 单元测试 | expected_version 不匹配时状态为 Conflict |
| 42 | 操作队列日志完整性 | 单元测试 | 3 条操作全部记录在 operation_log 中 |
| 43 | 版本号递增 | 单元测试 | 每次成功操作版本号 +1 |
| 44 | 并发操作冲突检测 | 单元测试 | 两个并发操作中只有一个成功 |
| 45 | 未注册章节拒绝 | 单元测试 | 未 register_chapter 的章节返回 Rejected |

---

# 第八章：pensoul-plugin — 插件系统

## 8.1 职责说明

YAML 声明式工作流、插件注册中心、验证器。让用户零代码扩展创作工作流。

## 8.2 文件结构

```
crates/pensoul-plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs               # 插件配置解析
│   ├── validator.rs            # 验证器
│   ├── registry.rs             # 注册中心
│   └── loader.rs               # 文件加载
```

## 8.3 核心数据类型

```rust
// ─── config.rs ─────────────────────────────────────

use pensoul_core::{PluginId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 插件阶段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStage {
    pub name: String,
    pub tool: String,
    #[serde(default)]
    pub gate: String,
    #[serde(default = "default_runner")]
    pub runner: String,
    #[serde(default)]
    pub prompt_template: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i32,
    #[serde(default = "default_retries")]
    pub max_retries: i32,
}

fn default_runner() -> String { "local".into() }
fn default_timeout() -> i32 { 300 }
fn default_retries() -> i32 { 3 }

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stages: Vec<PluginStage>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
```

```rust
// ─── validator.rs ──────────────────────────────────

const REQUIRED_FIELDS: &[&str] = &["plugin_id", "name", "version"];
const VALID_GATE_TYPES: &[&str] = &["auto", "manual", "conditional"];
const VALID_RUNNER_TYPES: &[&str] = &["local", "delegated"];

/// 插件验证器
pub struct PluginValidator;

impl PluginValidator {
    /// 验证插件配置
    pub fn validate(config: &PluginConfig) -> Result<()> {
        let mut errors = Vec::new();

        // 必填字段
        if config.plugin_id.is_empty() {
            errors.push("缺少必填字段: plugin_id".into());
        }
        if config.name.is_empty() {
            errors.push("缺少必填字段: name".into());
        }
        if config.version.is_empty() {
            errors.push("缺少必填字段: version".into());
        }

        // 阶段验证
        let mut stage_names = std::collections::HashSet::new();
        for stage in &config.stages {
            if stage.name.is_empty() {
                errors.push("阶段 name 不能为空".into());
            }
            if !stage_names.insert(&stage.name) {
                errors.push(format!("阶段名称重复: {}", stage.name));
            }
            if !VALID_GATE_TYPES.contains(&stage.gate.as_str()) {
                errors.push(format!("阶段 {}: 无效的 gate 类型 '{}'", stage.name, stage.gate));
            }
            if !VALID_RUNNER_TYPES.contains(&stage.runner.as_str()) {
                errors.push(format!("阶段 {}: 无效的 runner 类型 '{}'", stage.name, stage.runner));
            }
            if stage.timeout_seconds <= 0 {
                errors.push(format!("阶段 {}: timeout 必须为正数", stage.name));
            }
            if stage.max_retries < 0 {
                errors.push(format!("阶段 {}: max_retries 不能为负", stage.name));
            }
            if stage.runner == "local" && stage.allowed_tools.iter().any(|t| t == "delegate_to_expert") {
                errors.push(format!("阶段 {}: local runner 不能使用 delegate_to_expert", stage.name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(PensoulError::PluginValidationFailed { errors })
        }
    }
}
```

```rust
// ─── registry.rs ───────────────────────────────────

/// 插件注册中心
pub struct PluginRegistry {
    plugins: std::collections::HashMap<String, PluginConfig>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    /// 注册插件（带验证）
    pub fn register(&mut self, config: PluginConfig) -> Result<()> {
        PluginValidator::validate(&config)?;
        self.plugins.insert(config.plugin_id.clone(), config);
        Ok(())
    }

    pub fn get(&self, plugin_id: &str) -> Option<&PluginConfig> {
        self.plugins.get(plugin_id)
    }

    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    pub fn export_plugin(&self, plugin_id: &str) -> Result<String> {
        let config = self.plugins.get(plugin_id)
            .ok_or_else(|| PensoulError::Internal(format!("插件 {} 不存在", plugin_id)))?;
        serde_json::to_string_pretty(config)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))
    }

    pub fn import_plugin(&mut self, json_str: &str) -> Result<()> {
        let config: PluginConfig = serde_json::from_str(json_str)
            .map_err(|e| PensoulError::SerializationError(e.to_string()))?;
        self.register(config)
    }
}
```

## 8.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 46 | 合法插件注册成功 | 单元测试 | 返回 Ok，list_plugins 包含该 ID |
| 47 | 非法插件正确拒绝 | 单元测试 | 返回 Err，errors 包含 ≥ 4 个错误 |
| 48 | 重复阶段名检测 | 单元测试 | 验证失败，错误包含"重复" |
| 49 | 工具白名单一致性 | 单元测试 | local runner + delegate_to_expert → 失败 |
| 50 | 插件导出/导入 round-trip | 单元测试 | JSON 往返后 stages 数量一致 |

---

# 第九章：pensoul-consistency — 增量一致性检查

## 9.1 职责说明

基于影响图做增量一致性检查：分层策略（角色增量、设定全量、伏笔相邻）、5 条预置规则、检查报告生成。

## 9.2 文件结构

```
crates/pensoul-consistency/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── checker.rs              # 检查器
│   ├── rules.rs                # 5 条预置规则
│   ├── scope.rs                # 检查范围界定
│   ├── entity_state.rs         # 实体状态管理
│   └── report.rs               # 检查报告
```

## 9.3 核心数据类型

```rust
// ─── scope.rs ──────────────────────────────────────

use pensoul_core::{ChapterId, EntityId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Character,
    Setting,
    Timeline,
    Event,
    Plot,
    Foreshadow,
}

/// 检查范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyCheckScope {
    ChapterOnly,
    ChapterPlusNeighbors,
    FullBook,
}

/// 根据实体类型确定检查范围
pub fn determine_scope(entity_type: EntityType) -> ConsistencyCheckScope {
    match entity_type {
        EntityType::Setting => ConsistencyCheckScope::FullBook,
        EntityType::Timeline | EntityType::Foreshadow => ConsistencyCheckScope::ChapterPlusNeighbors,
        _ => ConsistencyCheckScope::ChapterOnly,
    }
}
```

```rust
// ─── entity_state.rs ───────────────────────────────

/// 实体状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: String,
    pub entity_type: EntityType,
    pub chapter_id: i64,
    pub state_data: serde_json::Value,
    pub version: i32,
}

/// 实体状态管理器
pub struct EntityStateManager {
    states: std::collections::HashMap<String, Vec<EntityState>>,
}

impl EntityStateManager {
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
        }
    }

    pub fn add_state(&mut self, state: EntityState) {
        let entry = self.states.entry(state.entity_id.clone()).or_insert_with(Vec::new);
        entry.push(state);
        entry.sort_by_key(|s| s.chapter_id);
    }

    pub fn get_states(&self, entity_id: &str) -> &[EntityState] {
        self.states.get(entity_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
```

```rust
// ─── checker.rs ────────────────────────────────────

/// 一致性违反记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyViolation {
    pub violation_id: String,
    pub entity_id: String,
    pub entity_type: EntityType,
    pub chapter_a: i64,
    pub chapter_b: i64,
    pub description: String,
    pub severity: String,
}

/// 检查报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub violations: Vec<ConsistencyViolation>,
    pub total_entities_checked: usize,
    pub total_violations: usize,
    pub check_duration_ms: u64,
}

/// 增量一致性检查器
pub struct IncrementalChecker {
    entity_manager: EntityStateManager,
    violations: Vec<ConsistencyViolation>,
}

impl IncrementalChecker {
    pub fn new() -> Self {
        Self {
            entity_manager: EntityStateManager::new(),
            violations: Vec::new(),
        }
    }

    pub fn add_entity_state(&mut self, state: EntityState) {
        self.entity_manager.add_state(state);
    }

    /// 增量检查指定章节
    pub fn check_incremental(
        &mut self,
        chapter_id: i64,
        entity_type: EntityType,
    ) -> Vec<ConsistencyViolation> {
        let scope = determine_scope(entity_type);
        let violations = match scope {
            ConsistencyCheckScope::ChapterOnly => self.check_chapter(chapter_id, entity_type),
            ConsistencyCheckScope::ChapterPlusNeighbors => {
                self.check_range(chapter_id - 1, chapter_id + 1, entity_type)
            }
            ConsistencyCheckScope::FullBook => self.check_range(1, i64::MAX, entity_type),
        };
        self.violations.extend(violations.clone());
        violations
    }

    fn check_chapter(&self, chapter_id: i64, entity_type: EntityType) -> Vec<ConsistencyViolation> {
        let mut violations = Vec::new();
        for (entity_id, states) in &self.entity_manager.states {
            let chapter_states: Vec<_> = states.iter()
                .filter(|s| s.chapter_id == chapter_id && s.entity_type == entity_type)
                .collect();
            // 检查同一章节内是否存在矛盾状态
            let mut seen = std::collections::HashSet::new();
            for s in chapter_states {
                let key = serde_json::to_string(&s.state_data).unwrap_or_default();
                if !seen.insert(key) {
                    violations.push(ConsistencyViolation {
                        violation_id: uuid::Uuid::new_v4().to_string(),
                        entity_id: entity_id.clone(),
                        entity_type,
                        chapter_a: chapter_id,
                        chapter_b: chapter_id,
                        description: format!("章节{}内实体'{}'存在矛盾状态", chapter_id, entity_id),
                        severity: "high".into(),
                    });
                }
            }
        }
        violations
    }

    fn check_range(&self, start: i64, end: i64, entity_type: EntityType) -> Vec<ConsistencyViolation> {
        let mut violations = Vec::new();
        for (entity_id, states) in &self.entity_manager.states {
            let relevant: Vec<_> = states.iter()
                .filter(|s| s.chapter_id >= start && s.chapter_id <= end && s.entity_type == entity_type)
                .collect();
            if relevant.len() < 2 { continue; }
            for i in 0..relevant.len() - 1 {
                let diff = compute_state_diff(&relevant[i].state_data, &relevant[i + 1].state_data);
                if !diff.is_empty() {
                    violations.push(ConsistencyViolation {
                        violation_id: uuid::Uuid::new_v4().to_string(),
                        entity_id: entity_id.clone(),
                        entity_type,
                        chapter_a: relevant[i].chapter_id,
                        chapter_b: relevant[i + 1].chapter_id,
                        description: format!("实体'{}'在第{}章和第{}章之间状态不一致: {}",
                            entity_id, relevant[i].chapter_id, relevant[i + 1].chapter_id, diff),
                        severity: "medium".into(),
                    });
                }
            }
        }
        violations
    }

    pub fn get_all_violations(&self) -> &[ConsistencyViolation] {
        &self.violations
    }
}

fn compute_state_diff(a: &serde_json::Value, b: &serde_json::Value) -> String {
    let mut diffs = Vec::new();
    let a_obj = a.as_object();
    let b_obj = b.as_object();
    if let (Some(ao), Some(bo)) = (a_obj, b_obj) {
        let all_keys: std::collections::HashSet<_> = ao.keys().chain(bo.keys()).collect();
        for key in all_keys {
            if ao.get(key) != bo.get(key) {
                diffs.push(format!("{}: {:?} -> {:?}", key, ao.get(key), bo.get(key)));
            }
        }
    }
    diffs.join("; ")
}
```

## 9.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 51 | 角色检查范围 = 增量 | 单元测试 | determine_scope(Character) == ChapterOnly |
| 52 | 设定检查范围 = 全量 | 单元测试 | determine_scope(Setting) == FullBook |
| 53 | 伏笔检查范围 = 相邻 | 单元测试 | determine_scope(Foreshadow) == ChapterPlusNeighbors |
| 54 | 跨章状态对比 | 单元测试 | 检测到设定交换率变化，返回 ≥ 1 个违反 |
| 55 | 违反记录完整 | 单元测试 | 每个 violation 含 violation_id、severity |

---

# 第十章：pensoul-import — 数据导入导出

## 10.1 职责说明

TXT/DOCX/EPUB/Markdown 章节自动检测（含中文数字解析）、备份恢复、导出多格式。

## 10.2 文件结构

```
crates/pensoul-import/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── text_importer.rs        # TXT 导入
│   ├── chapter_detect.rs       # 章节检测
│   ├── cn_number.rs            # 中文数字解析
│   ├── exporter.rs             # 导出
│   └── backup.rs               # 备份恢复
```

## 10.3 核心数据类型

```rust
// ─── chapter_detect.rs ─────────────────────────────

use pensoul_core::{ChapterId, PensoulError, Result};
use serde::{Deserialize, Serialize};

/// 检测到的章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedChapter {
    pub chapter_number: i64,
    pub title: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub word_count: usize,
    pub confidence: f32,
}

/// 章节检测器
pub struct ChapterDetector {
    patterns: Vec<String>,
}

impl ChapterDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                r"^第[一二三四五六七八九十百千\d]+章[：:\s]*(.+)?$".into(),
                r"^Chapter\s+(\d+)[：:\s]*(.+)?$".into(),
                r"^第\s*(\d+)\s*章[：:\s]*(.+)?$".into(),
                r"^(\d+)\.\s*(.+)$".into(),
                r"^【第(\d+)章[】\s]*(.+)?$".into(),
            ],
        }
    }

    /// 检测文本中的章节
    pub fn detect(&self, text: &str, min_words: usize) -> Vec<DetectedChapter> {
        let lines: Vec<&str> = text.lines().collect();
        let mut markers: Vec<(usize, String, i64)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            for pattern in &self.patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(trimmed) {
                        let num = extract_chapter_number(trimmed);
                        markers.push((i, trimmed.to_string(), num));
                        break;
                    }
                }
            }
        }

        if markers.is_empty() {
            let content: String = lines.join("\n");
            let wc = content.replace('\n', "").replace(' ', "").len();
            return vec![DetectedChapter {
                chapter_number: 1,
                title: "全文".into(),
                content,
                start_line: 0,
                end_line: lines.len().saturating_sub(1),
                word_count: wc,
                confidence: 0.5,
            }];
        }

        let mut chapters = Vec::new();
        for (idx, (line_idx, title, chapter_num)) in markers.iter().enumerate() {
            let start = *line_idx;
            let end = markers.get(idx + 1).map(|m| m.0).unwrap_or(lines.len());
            let content: String = lines[start..end].join("\n");
            let wc = content.replace('\n', "").replace(' ', "").len();
            let confidence = estimate_confidence(
                *chapter_num, wc, min_words,
                chapters.last().map(|c: &DetectedChapter| c.chapter_number),
            );
            chapters.push(DetectedChapter {
                chapter_number: *chapter_num,
                title: title.clone(),
                content,
                start_line: start,
                end_line: end.saturating_sub(1),
                word_count: wc,
                confidence,
            });
        }
        chapters
    }
}
```

```rust
// ─── cn_number.rs ──────────────────────────────────

/// 中文数字解析 — 支持组合如 二十三、一百二十
pub fn parse_cn_number(cn: &str) -> i64 {
    let cn_num_map: std::collections::HashMap<char, i64> = [
        ('零', 0), ('一', 1), ('二', 2), ('三', 3), ('四', 4),
        ('五', 5), ('六', 6), ('七', 7), ('八', 8), ('九', 9),
        ('十', 10), ('百', 100), ('千', 1000), ('万', 10000),
    ].into_iter().collect();

    let mut result: i64 = 0;
    let mut current: i64 = 0;
    for ch in cn.chars() {
        let val = cn_num_map.get(&ch).copied().unwrap_or(0);
        if val >= 10000 {
            result = (result + current) * val;
            current = 0;
        } else if val >= 100 {
            result = (result + current) * val;
            current = 0;
        } else if val >= 10 {
            if current == 0 { current = 1; }
            result += current * val;
            current = 0;
        } else {
            current = val;
        }
    }
    result + current
}

/// 从标题中提取章节号
pub fn extract_chapter_number(title: &str) -> i64 {
    // 先尝试数字
    if let Some(num_str) = title.split_whitespace()
        .find(|s| s.chars().all(|c| c.is_ascii_digit()))
    {
        if let Ok(n) = num_str.parse::<i64>() {
            return n;
        }
    }
    // 再尝试中文数字
    let cn_text: String = title.chars()
        .take_while(|c| "零一二三四五六七八九十百千万".contains(*c))
        .collect();
    if !cn_text.is_empty() {
        return parse_cn_number(&cn_text);
    }
    0
}
```

## 10.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 56 | 章节自动检测 — "第X章" 格式 | 单元测试 | 3 章正确识别，平均置信度 ≥ 0.7 |
| 57 | 中文章节号 "十一" "十二" | 单元测试 | parse_cn_number("十一") == 11 |
| 58 | 中文组合数字 "二十三" | 单元测试 | parse_cn_number("二十三") == 23 |
| 59 | 纯文本无标记处理 | 单元测试 | 识别为 1 章，confidence = 0.5 |
| 60 | 置信度估算 — 连续章节号 | 单元测试 | 连续章节号 confidence 提升 0.2 |

---

# 第十一章：pensoul-llm — LLM 模型管理

## 11.1 职责说明

用户手动选择模型、容灾降级、多模型对比。系统提供智能推荐但不做自动决策。

## 11.2 文件结构

```
crates/pensoul-llm/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── model.rs                # 模型配置
│   ├── router.rs               # 模型路由（容灾降级）
│   ├── provider.rs             # LLM 提供商抽象
│   └── comparison.rs           # 多模型对比
```

## 11.3 核心数据类型

```rust
// ─── model.rs ──────────────────────────────────────

use serde::{Deserialize, Serialize};

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Outline,
    Drafting,
    Revision,
    Consistency,
    Style,
    General,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub provider: String,
    pub display_name: String,
    pub max_tokens: u32,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub cost_per_1k_tokens: f64,
    pub avg_quality_score: f32,
    pub avg_latency_ms: u32,
    pub is_available: bool,
    pub failure_count: u32,
    pub last_failure_time: f64,
    pub cooldown_seconds: u64,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
}

/// 路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    pub chosen_model: ModelConfig,
    pub fallback_used: bool,
    pub fallback_reason: String,
    pub attempt_chain: Vec<String>,
    pub routing_time_ms: u64,
}
```

```rust
// ─── router.rs ─────────────────────────────────────

/// 模型路由器 — 用户手动选择 + 容灾降级
pub struct ModelRouter {
    models: std::collections::HashMap<String, ModelConfig>,
    task_preferences: std::collections::HashMap<TaskType, Vec<String>>,
    routing_log: Vec<RoutingResult>,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
            task_preferences: std::collections::HashMap::new(),
            routing_log: Vec::new(),
        }
    }

    pub fn register_model(&mut self, model: ModelConfig) {
        self.models.insert(model.model_id.clone(), model);
    }

    /// 设置任务偏好（用户手动选择的模型列表）
    pub fn set_task_preference(&mut self, task_type: TaskType, model_ids: Vec<String>) {
        self.task_preferences.insert(task_type, model_ids);
    }

    /// 路由到可用模型
    pub fn route(&mut self, task_type: TaskType) -> Result<RoutingResult, PensoulError> {
        let start = std::time::Instant::now();
        let mut attempt_chain = Vec::new();

        let preferred = self.task_preferences.get(&task_type).cloned().unwrap_or_default();
        let other_ids: Vec<String> = self.models.keys()
            .filter(|id| !preferred.contains(id))
            .cloned()
            .collect();
        let candidate_ids: Vec<_> = preferred.into_iter().chain(other_ids).collect();

        for model_id in &candidate_ids {
            let model = match self.models.get(model_id) {
                Some(m) => m.clone(),
                None => continue,
            };
            attempt_chain.push(model_id.clone());
            if !model.is_available { continue; }
            if model.failure_count > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();
                let elapsed = now - model.last_failure_time;
                if elapsed < model.cooldown_seconds as f64 { continue; }
            }
            let result = RoutingResult {
                chosen_model: model,
                fallback_used: attempt_chain.len() > 1,
                fallback_reason: if attempt_chain.len() > 1 {
                    format!("前{}个模型不可用", attempt_chain.len() - 1)
                } else {
                    String::new()
                },
                attempt_chain: attempt_chain.clone(),
                routing_time_ms: start.elapsed().as_millis() as u64,
            };
            self.routing_log.push(result.clone());
            return Ok(result);
        }
        Err(PensoulError::LlmAllModelsFailed { chain: attempt_chain })
    }

    pub fn report_failure(&mut self, model_id: &str) {
        if let Some(model) = self.models.get_mut(model_id) {
            model.failure_count += 1;
            model.last_failure_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
        }
    }

    pub fn get_recommendation(&self, task_type: TaskType) -> Vec<&ModelConfig> {
        self.task_preferences.get(&task_type)
            .map(|prefs| prefs.iter().filter_map(|id| self.models.get(id)).collect())
            .unwrap_or_default()
    }
}
```

## 11.4 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 61 | 用户手动选择 — 首选模型 | 单元测试 | route 返回 task_preferences 中第一个可用模型 |
| 62 | 容灾降级 — 主模型失败 | 单元测试 | report_failure 3 次后 route 跳过该模型 |
| 63 | 全链路降级 | 单元测试 | 所有模型失败返回 LlmAllModelsFailed |
| 64 | 冷却恢复 | 单元测试 | 超过 cooldown_seconds 后模型恢复可用 |
| 65 | 智能推荐列表 | 单元测试 | get_recommendation 返回偏好模型列表 |
| 66 | 路由日志记录 | 单元测试 | 每次路由都记录在 routing_log 中 |

---

# 第十二章：pensoul-app — Tauri 桌面应用

## 12.1 职责说明

7 个核心视图（写作、大纲、角色管理、世界观编辑器、一致性仪表盘、Harness 控制台、写法工坊）、IPC 命令、全局状态管理。

## 12.2 文件结构

```
crates/pensoul-app/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Tauri 入口
│   ├── lib.rs
│   ├── state.rs                # 全局状态
│   ├── commands/               # IPC 命令
│   │   ├── mod.rs
│   │   ├── project.rs
│   │   ├── chapter.rs
│   │   ├── harness.rs
│   │   ├── cda.rs
│   │   ├── memory.rs
│   │   ├── model.rs
│   │   └── plugin.rs
│   └── views/                  # 视图状态
│       ├── mod.rs
│       ├── writing.rs
│       ├── outline.rs
│       ├── character.rs
│       ├── world.rs
│       ├── consistency.rs
│       ├── harness_console.rs
│       └── style_workshop.rs
```

## 12.3 核心数据类型

```rust
// ─── state.rs ──────────────────────────────────────

use pensoul_core::NovelOntology;
use pensoul_harness::HarnessEngine;
use pensoul_cda::ImpactGraph;
use pensoul_memory::{HotMemory, WarmMemory, ColdMemory, NarrativeMemory};
use pensoul_concurrency::ConcurrencyController;
use pensoul_llm::ModelRouter;
use pensoul_plugin::PluginRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 全局应用状态
pub struct AppState {
    pub project_dir: std::path::PathBuf,
    pub ontology: Arc<RwLock<NovelOntology>>,
    pub harness: Arc<RwLock<HarnessEngine>>,
    pub impact_graph: Arc<RwLock<ImpactGraph>>,
    pub hot_memory: Arc<RwLock<HotMemory>>,
    pub warm_memory: Arc<RwLock<WarmMemory>>,
    pub cold_memory: Arc<RwLock<ColdMemory>>,
    pub narrative_memory: Arc<RwLock<NarrativeMemory>>,
    pub concurrency: Arc<RwLock<ConcurrencyController>>,
    pub model_router: Arc<RwLock<ModelRouter>>,
    pub plugin_registry: Arc<RwLock<PluginRegistry>>,
}
```

```rust
// ─── commands/project.rs ───────────────────────────

/// 创建新项目
#[tauri::command]
pub async fn create_project(
    state: tauri::State<'_, Arc<AppState>>,
    title: String,
    template: Option<String>,
) -> Result<String, String> {
    todo!("实现创建项目命令")
}

/// 打开项目
#[tauri::command]
pub async fn open_project(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    todo!("实现打开项目命令")
}
```

```rust
// ─── commands/chapter.rs ───────────────────────────

/// 获取章节内容
#[tauri::command]
pub async fn get_chapter(
    state: tauri::State<'_, Arc<AppState>>,
    chapter_id: i64,
) -> Result<String, String> {
    todo!("实现获取章节命令")
}

/// 保存章节（带乐观锁）
#[tauri::command]
pub async fn save_chapter(
    state: tauri::State<'_, Arc<AppState>>,
    chapter_id: i64,
    content: String,
    expected_version: i32,
) -> Result<i32, String> {
    todo!("实现保存章节命令（返回新版本号）")
}
```

```rust
// ─── commands/harness.rs ───────────────────────────

/// 启动当前阶段
#[tauri::command]
pub async fn start_harness_stage(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    todo!("实现启动阶段命令")
}

/// 完成阶段（带结果）
#[tauri::command]
pub async fn complete_harness_stage(
    state: tauri::State<'_, Arc<AppState>>,
    result: serde_json::Value,
) -> Result<(), String> {
    todo!("实现完成阶段命令")
}

/// 注入滚动备忘录
#[tauri::command]
pub async fn inject_memo(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    todo!("实现注入备忘录命令")
}

/// 获取当前引擎状态
#[tauri::command]
pub async fn get_harness_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    todo!("实现获取引擎状态命令")
}
```

```rust
// ─── commands/cda.rs ───────────────────────────────

/// 查询受影响章节
#[tauri::command]
pub async fn find_affected_chapters(
    state: tauri::State<'_, Arc<AppState>>,
    chapter_id: i64,
    changed_entities: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    todo!("实现查询受影响章节命令")
}
```

```rust
// ─── commands/model.rs ─────────────────────────────

/// 获取模型列表
#[tauri::command]
pub async fn get_models(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    todo!("实现获取模型列表命令")
}

/// 设置任务模型分配
#[tauri::command]
pub async fn set_task_model(
    state: tauri::State<'_, Arc<AppState>>,
    task_type: String,
    model_id: String,
) -> Result<(), String> {
    todo!("实现设置任务模型命令")
}
```

## 12.4 Tauri 视图

| 视图 | 功能 | 关键 IPC 命令 |
|------|------|-------------|
| 写作视图 | 沉浸式编辑器 + AI 续写 + 一致性侧边栏 | `get_chapter`, `save_chapter`, `find_affected_chapters` |
| 大纲视图 | 可折叠树状大纲 + 拖拽重排 + 伏笔地图 | `get_outline`, `update_outline` |
| 角色管理 | 角色卡片 + 关系网络图 + 知识库可视化 | `get_characters`, `update_character` |
| 世界观编辑器 | 设定卡片 + 时间线 + 术语表 + 规则引擎 | `get_world`, `update_world` |
| 一致性仪表盘 | 全书一致性热力图 + 冲突列表 | `run_consistency_check`, `get_violations` |
| Harness 控制台 | 阶段状态机可视化 + 门控状态 + 执行日志 | `get_harness_status`, `start_harness_stage` |
| 写法工坊 | 写法资产管理 + 特征池 + 反 AI 规则 + 试写对比 | `get_styles`, `add_style` |

## 12.5 验收标准

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 67 | IPC 命令 — 创建项目 | 集成测试 | 返回项目路径，ontology 非空 |
| 68 | IPC 命令 — 保存章节（乐观锁） | 集成测试 | 版本冲突时返回错误 |
| 69 | IPC 命令 — Harness 状态查询 | 集成测试 | 返回 current_stage 和 memo |
| 70 | 全局状态线程安全 | 并发测试 | 多线程读写无 panic |
| 71 | 7 个核心视图可加载 | UI 测试 | 每个视图渲染无错误 |

---

# 第十三章：模块间集成与验收

## 13.1 模块间集成场景

### 场景一：章节创作完整流程

```
用户点击"开始写作"
  → pensoul-harness: start_stage("chapter_write")
  → pensoul-memory: build_memory_packet(drafting)
  → pensoul-llm: route(Drafting) → 选择模型
  → pensoul-agent: 调用 SceneWriter Agent
  → pensoul-harness: complete_stage(result)
  → pensoul-harness: 自动门控(auto) → 进入审查
  → pensoul-agent: 调用 ConsistencyAuditor
  → pensoul-consistency: check_incremental()
  → pensoul-agent: signal 通道 → pass/fail
  → pensoul-harness: 条件门控(score >= 80) → 放行或回退
```

### 场景二：用户修改后联动传播

```
用户修改第 10 章
  → pensoul-concurrency: submit_operation(乐观锁)
  → pensoul-cda: find_affected_chapters(10, changed_entities)
  → pensoul-consistency: 生成受影响章节的检查报告
  → pensoul-app: 显示联动建议 UI
  → 用户选择"全部应用"
  → pensoul-concurrency: 批量操作
```

### 场景三：崩溃恢复

```
应用启动
  → pensoul-harness: recover_from_crash()
  → 读取 WAL → 校验 checksum → 重放操作
  → 恢复 current_stage + memo + stage_instances
  → pensoul-app: 显示恢复状态
```

## 13.2 端到端验收

| # | 验收项 | 验证方式 | 通过标准 |
|---|--------|---------|---------|
| 72 | 端到端 — 章节创作流程 | 端到端测试 | 开书→大纲→写作→审查→润色完整走通 |
| 73 | 端到端 — 联动传播 | 端到端测试 | 修改第 N 章后正确找出受影响章节 |
| 74 | 端到端 — 崩溃恢复 | 端到端测试 | 模拟崩溃后恢复，memo 和 stage 完整 |
| 75 | 端到端 — 记忆注入 | 端到端测试 | 写作阶段的 LLM prompt 包含正确的记忆上下文 |

## 13.3 性能目标

| 指标 | 目标 | 测试条件 |
|------|------|---------|
| 影响图构建 | < 50ms | 200 章 / 1020 节点 |
| 变更传播查询 | < 5ms | 1000 章 / 5029 节点 |
| 记忆包构建 | < 50ms | 500 章 |
| WAL 崩溃恢复 | < 100ms | 100 条 WAL 条目 |
| 增量一致性检查 | < 200ms | 1000 章 / 50 个实体 |
| 章节保存（含乐观锁） | < 10ms | 单章写入 |
| IPC 命令响应 | < 50ms | 所有只读查询 |

## 13.4 安全边界

| 边界 | 措施 |
|------|------|
| LLM API Key 存储 | 系统钥匙串（macOS Keychain / Windows Credential Manager） |
| WAL 文件权限 | 仅当前用户可读写（0600） |
| 插件沙箱 | 插件不能执行任意系统命令 |
| 导入文件验证 | 文件大小上限、编码检测、内容消毒 |
| 内存限制 | 向量检索结果集上限、热记忆窗口限制 |

---

# 附录 A：Crate 依赖关系图

```
pensoul-core          ← 所有 crate 的基础
    ↑
    ├── pensoul-harness
    ├── pensoul-cda
    ├── pensoul-memory
    ├── pensoul-agent
    ├── pensoul-concurrency
    ├── pensoul-plugin
    ├── pensoul-consistency
    ├── pensoul-import
    ├── pensoul-llm
    │
    └── pensoul-app       ← 依赖所有其他 crate

依赖关系：
  pensoul-app ──→ pensoul-harness
  pensoul-app ──→ pensoul-cda
  pensoul-app ──→ pensoul-memory
  pensoul-app ──→ pensoul-agent
  pensoul-app ──→ pensoul-concurrency
  pensoul-app ──→ pensoul-plugin
  pensoul-app ──→ pensoul-consistency
  pensoul-app ──→ pensoul-import
  pensoul-app ──→ pensoul-llm
  pensoul-app ──→ pensoul-core

  pensoul-harness  ──→ pensoul-core
  pensoul-cda      ──→ pensoul-core
  pensoul-memory   ──→ pensoul-core
  pensoul-agent    ──→ pensoul-core
  pensoul-concurrency ──→ pensoul-core
  pensoul-plugin   ──→ pensoul-core
  pensoul-consistency ──→ pensoul-core
  pensoul-import   ──→ pensoul-core
  pensoul-llm      ──→ pensoul-core
```

```
可视化：

              ┌────────────┐
              │ pensoul-app│ (Tauri)
              └─────┬──────┘
     ┌──────┬───────┼───────┬──────┬──────┬──────┬──────┬──────┐
     ▼      ▼       ▼       ▼      ▼      ▼      ▼      ▼      ▼
  harness  cda   memory  agent  conc. plugin cons. import  llm
     └──────┴───────┴───────┴──────┴──────┴──────┴──────┴──────┘
                            │
                     ┌──────▼──────┐
                     │pensoul-core │
                     └─────────────┘
```

---

# 附录 B：总验收清单

| # | 模块 | 验收项 |
|---|------|--------|
| 1 | core | 新类型 ID 类型安全 |
| 2 | core | 四层本体序列化/反序列化 |
| 3 | core | PensoulError 覆盖所有错误场景 |
| 4 | core | 角色知识库防穿越 |
| 5 | core | 伏笔状态机 5 态流转 |
| 6 | core | NovelOntology 创建空白项目 |
| 7 | harness | 阶段状态机确定性流转 |
| 8 | harness | 自动放行 |
| 9 | harness | 人工放行 |
| 10 | harness | 条件放行 |
| 11 | harness | 工具白名单 — 显式禁止 |
| 12 | harness | 工具白名单 — 不在允许列表 |
| 13 | harness | WAL 写入与刷盘 |
| 14 | harness | WAL 校验和验证 |
| 15 | harness | 崩溃恢复 — WAL 重放 |
| 16 | harness | 滚动备忘录跨阶段注入 |
| 17 | harness | 最大重试次数拦截 |
| 18 | harness | 引擎状态快照持久化 |
| 19 | cda | 200 章影响图构建性能 |
| 20 | cda | 变更传播查询性能 |
| 21 | cda | 影响分级 — 直接影响 |
| 22 | cda | 影响分级 — 间接影响 |
| 23 | cda | 影响分级 — 级联影响 |
| 24 | cda | BFS 环检测 |
| 25 | cda | 联动建议生成 |
| 26 | memory | 热记忆窗口 ± 2 章完整文本 |
| 27 | memory | 温记忆全量注入 |
| 28 | memory | 冷记忆向量检索 Top-K |
| 29 | memory | 叙事记忆按重要性排序 |
| 30 | memory | 三种编辑模式预算分配 |
| 31 | memory | 500 章大规模记忆包构建性能 |
| 32 | memory | Token 预算不超限 |
| 33 | memory | 记忆更新管道完整性 |
| 34 | agent | signal 通道路由到引擎 |
| 35 | agent | report 通道路由到 UI |
| 36 | agent | JSON 序列化/反序列化 |
| 37 | agent | 多消息路由 |
| 38 | agent | 6 个预置 Agent 定义完整 |
| 39 | agent | 信号与报告通道隔离 |
| 40 | concurrency | 乐观锁 — 版本匹配 |
| 41 | concurrency | 乐观锁 — 版本冲突 |
| 42 | concurrency | 操作队列日志完整性 |
| 43 | concurrency | 版本号递增 |
| 44 | concurrency | 并发操作冲突检测 |
| 45 | concurrency | 未注册章节拒绝 |
| 46 | plugin | 合法插件注册成功 |
| 47 | plugin | 非法插件正确拒绝 |
| 48 | plugin | 重复阶段名检测 |
| 49 | plugin | 工具白名单一致性 |
| 50 | plugin | 插件导出/导入 round-trip |
| 51 | consistency | 角色检查范围 = 增量 |
| 52 | consistency | 设定检查范围 = 全量 |
| 53 | consistency | 伏笔检查范围 = 相邻 |
| 54 | consistency | 跨章状态对比 |
| 55 | consistency | 违反记录完整 |
| 56 | import | 章节自动检测 — "第X章" 格式 |
| 57 | import | 中文章节号 "十一" "十二" |
| 58 | import | 中文组合数字 "二十三" |
| 59 | import | 纯文本无标记处理 |
| 60 | import | 置信度估算 — 连续章节号 |
| 61 | llm | 用户手动选择 — 首选模型 |
| 62 | llm | 容灾降级 — 主模型失败 |
| 63 | llm | 全链路降级 |
| 64 | llm | 冷却恢复 |
| 65 | llm | 智能推荐列表 |
| 66 | llm | 路由日志记录 |
| 67 | app | IPC 命令 — 创建项目 |
| 68 | app | IPC 命令 — 保存章节（乐观锁） |
| 69 | app | IPC 命令 — Harness 状态查询 |
| 70 | app | 全局状态线程安全 |
| 71 | app | 7 个核心视图可加载 |
| 72 | E2E | 端到端 — 章节创作流程 |
| 73 | E2E | 端到端 — 联动传播 |
| 74 | E2E | 端到端 — 崩溃恢复 |
| 75 | E2E | 端到端 — 记忆注入 |

---

*本文档由 PenSoul 架构团队生成，基于 DESIGN-V2.md、FEASIBILITY-REPORT.md 和 Python 原型代码。*
*验收标准总数：75 项。*
