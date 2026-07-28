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
| 工作流 | 写死在代码里 | 声明式配置，零代码可换 |

## 核心功能

- **Harness 引擎** — 确定性流程引擎，用 Stage/Gate/WAL 驱动创作流程，支持自动/人工/条件三种门控模式
- **四层记忆系统** — 热/温/冷/叙事四级记忆，效果优先，无 token 预算上限
- **CDA 一致性驱动架构** — 基于影响图的 BFS 传播，1000 章构建 < 50ms，修改后自动标记受影响章节
- **Agent 通信系统** — 双通道 signal/report 路由，Agent 间零耦合通信
- **并发控制** — 乐观锁 + 操作队列 + 冲突检测，用户编辑与 AI 生成互不干扰
- **增量一致性检查** — 按范围（章节/卷/全书）增量扫描，实时报告不一致项
- **LLM 模型路由** — 任务偏好、冷却与故障转移，支持多 provider 自动切换
- **插件系统** — YAML/JSON 声明式插件，无需编译即可扩展

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 后端语言 | Rust (edition 2024) |
| 前端 | React + TypeScript + TipTap |
| 异步运行时 | tokio |
| 序列化 | serde + serde_json |
| 向量数据库 | LanceDB |
| 关系数据库 | SQLite (rusqlite) |

## 项目结构

```
├── Cargo.toml                 # 工作空间定义
├── src/                       # 前端源码（React + TypeScript）
│   ├── components/            # 通用组件
│   ├── views/                 # 页面视图
│   └── store.ts               # 状态管理
├── crates/                    # Rust crate 集合
│   ├── pensoul-core/          # 领域模型：四层本体、错误类型
│   ├── pensoul-harness/       # 确定性流程引擎
│   ├── pensoul-cda/           # 一致性驱动架构
│   ├── pensoul-memory/        # 四层记忆系统
│   ├── pensoul-agent/         # Agent 通信系统
│   ├── pensoul-concurrency/   # 并发控制
│   ├── pensoul-consistency/   # 增量一致性检查
│   ├── pensoul-llm/           # LLM 模型路由
│   ├── pensoul-plugin/        # 插件系统
│   ├── pensoul-import/        # 数据导入导出
│   └── pensoul-app/           # Tauri 桌面应用入口
├── docs/                      # 设计文档
│   ├── DESIGN-V2.md           # 完整设计文档
│   ├── DEVELOPMENT-MANUAL-V2.md # 开发手册
│   └── FEASIBILITY-REPORT.md  # 可行性验证报告
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

## 性能指标（来自可行性验证）

| 测试项 | 结果 |
|--------|------|
| 1000 章影响图构建 | 5029 节点 / 0.03ms |
| 500 章记忆包构建 | 平均 < 50ms |
| 200 章影响图构建 | 1020 节点 / 1800 边，构建 < 50ms |

## 设计文档

详细的架构设计、开发指南和可行性验证报告存放在 `docs/` 目录：

- **DESIGN-V2.md** — 设计哲学、市场调研、架构设计、核心功能设计
- **DEVELOPMENT-MANUAL-V2.md** — Rust 开发指南、各 crate 详细说明、验收清单
- **FEASIBILITY-REPORT.md** — 核心架构可行性验证结果

## 许可证

本项目基于 MIT 许可证开源。
