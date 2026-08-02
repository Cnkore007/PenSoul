# PenSoul — AI 长篇小说创作平台

让引擎管流程，让模型管创作。

PenSoul 是一个面向网络文学作者和严肃小说创作者的 AI 写作工具，基于 Rust + Tauri 构建，解决 500 万字级别超长篇小说的一致性、长期记忆和协同编辑问题。

## 核心理念

主流 AI 写作工具把流程写在提示词里，建议 AI 按步骤来——但建议终究只是建议，AI 会跳步、会绕过一致性检查、会忘记伏笔。PenSoul 换了一条路：

| 维度 | 主流做法 | PenSoul |
|------|---------|---------|
| 流程控制 | 写在提示词里（建议） | 写在引擎的确定性逻辑里（约束） |
| AI 行为 | AI 自觉遵守，可跳步 | 引擎硬性驱动，不可绕过 |
| 质量保障 | 可选步骤 | 必经之路（门控放行） |
| 工作流 | 写死在代码里 | P0 硬编码编排，P1 声明式工作流模板 |

## 核心功能

- **Harness 引擎** — 确定性流程引擎，用 Stage/Gate/WAL 驱动创作流程，支持自动/人工/条件三种门控模式
- **章节连写管线** — 写作 → 审查（异模型门控）→ 回灌三阶段自动连写，支持暂停/继续/停止
- **四层记忆系统** — 热/温/冷/叙事四级记忆，8 步更新管道，预算按编辑模式分配（默认 8000 token）
- **CDA 一致性驱动架构** — 基于影响图的 BFS 传播，1000 章构建 < 50ms，修改后自动标记受影响章节
- **多 Agent 讨论** — 立论/交锋/成果三轮讨论，专家库 Agent 自动加载技能，进度实时推送
- **并发控制** — 乐观锁 + 操作队列 + 冲突检测，用户编辑与 AI 生成互不干扰
- **增量一致性检查** — 按范围（章节/卷/全书）增量扫描，实时报告不一致项
- **LLM 模型适配** — 按模型自动匹配请求参数（推理开关/预算字段），统一调用、重试与降级
- **专家/书籍蒸馏** — 把人物思维与书籍写法提炼为可复用技能卡

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 后端语言 | Rust (edition 2024) |
| 前端 | React + TypeScript + TipTap |
| 异步运行时 | tokio |
| 序列化 | serde + serde_json |
| 持久化 | 项目 JSON 文件，原子写（tmp + rename） |

## 项目结构

```
├── Cargo.toml                 # 工作空间定义
├── agent.md                   # Agent 强制工作约定（文档同步规则）
├── src/                       # 前端源码（React + TypeScript）
│   ├── components/            # 通用组件
│   ├── views/                 # 页面视图
│   ├── ipc.ts                 # IPC 封装（命令唯一入口）
│   ├── types.ts               # 前后端共享类型
│   └── store.ts               # 状态管理
├── crates/                    # Rust crate 集合（11 个）
│   ├── pensoul-core/          # 领域模型：四层本体、错误类型
│   ├── pensoul-harness/       # 确定性流程引擎（门控/WAL/崩溃恢复）
│   ├── pensoul-cda/           # 影响图与 BFS 变更传播
│   ├── pensoul-memory/        # 四层记忆系统
│   ├── pensoul-agent/         # Agent 双通道协议与预置定义
│   ├── pensoul-concurrency/   # 并发控制（版本/校验和/冲突）
│   ├── pensoul-consistency/   # 增量一致性检查（5 条规则）
│   ├── pensoul-llm/           # 模型路由（应用层未接入，见架构文档）
│   ├── pensoul-import/        # 数据导入导出/备份
│   └── pensoul-app/           # Tauri 应用层：命令/管线/集成
├── skills/                    # 蒸馏方法论技能（Experts/Books）
├── Experts/                   # 专家蒸馏产物
├── tools/                     # 图标生成工具脚本
├── docs/                      # 设计文档
│   ├── ARCHITECTURE.md        # 架构总览
│   ├── WORKFLOW.md            # 创作全流程手册
│   ├── DEVELOPMENT.md         # 开发指南
│   ├── IPC-CONTRACT.md        # IPC 契约（前后端事实源）
│   └── PROGRESS.md            # 工作进度日志（Agent 强制同步）
├── index.html                 # 前端入口
└── vite.config.ts             # Vite 配置
```

## 快速开始

### 前提条件

- Rust 2024 或更新版本
- Node.js 18+
- macOS（Tauri 开发环境）

### 构建与运行

```bash
# 构建所有 crate
cargo build

# 启动 Vite 前端开发服务器
npx vite dev

# 启动 Tauri 桌面应用
cargo tauri dev

# 运行全部测试
cargo test

# 代码检查
cargo clippy --all -- -D warnings
cargo fmt --all --check
```

### 前端构建

```bash
npx vite build
```

构建产物在 `dist/` 目录，Tauri 生产模式会使用该目录。

## 性能指标（历史实测）

| 测试项 | 结果 |
|--------|------|
| 1000 章影响图构建 | 5029 节点 / 0.03ms |
| 500 章记忆包构建 | 平均 < 50ms |
| 200 章影响图构建 | 1020 节点 / 1800 边，构建 < 50ms |

## 设计文档

文档存放在 `docs/` 目录，均对齐当前代码：

- **ARCHITECTURE.md** — 架构总览：crate 职责、管线、集成层、LLM 层、事件流
- **WORKFLOW.md** — 创作全流程手册：从萌芽讨论到造化工坊连写的完整链路
- **DEVELOPMENT.md** — 开发指南：构建/测试/规范/新增 IPC 三步
- **IPC-CONTRACT.md** — IPC 契约：全部命令与事件协议
- **PROGRESS.md** — 工作进度日志：每个 Agent 每轮工作必须追加记录

仓库根目录的 **agent.md** 是强制约定：无论换什么 LLM/Agent 工具，开工前必读
`docs/PROGRESS.md`，每轮工作后必须把进度、问题、设计思考写入其中，代码变更后
同步对应文档，防止文档与代码再次脱节。

## 许可证

本项目基于 MIT 许可证开源。
