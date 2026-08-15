# PenSoul

> 让引擎管流程，让模型管创作。

PenSoul 是一个面向 **500 万字级长篇小说**的 AI 辅助创作工作台。以 `NovelOntology` 为唯一正典（图谱、约束、记忆均为派生状态），由约束引擎硬性保证一致性，由 LLM Agent 体系驱动「萌芽 → 图谱 → 大纲 → 细纲 → 笔耕 → 审校 → 归档」的工业化创作管线。

## 核心特性

- **约束引擎**：角色状态倒流 / 时间线倒流 / 设定重名 / 伏笔状态机 / 事件悬空引用五条硬规则，写路径事前检查 + 事后验证（失败回滚）
- **创作管线（P0–P6 全部落地）**
  - 萌芽：诘问式对话创作台（高概念 → 主角 → 冲突 → 世界观 → 基调 → 结局）
  - 图谱知识库：人物/组织/设定/事件/伏笔/规则六类档案，全量增删改查
  - 大纲与细纲：脉络/章节 CRUD，细纲分块生成（每 12 章一批）与导入
  - 笔耕：AI 初稿/续写（记忆检索 + 硬约束 + 技巧注入）、审校（本地启发式 + LLM 深度）、批注状态机、消痕改写（diff 确认）、级联同步（仅向后）、批量写作（硬门控 + 检查点制）
  - 蒸馏：txt/md/epub/pdf 语料 → 7+1 维风格画像 → 注入写作/审校提示词（版权红线内置）
  - 归档：操作日志 / 回滚 / 卷摘要压缩 / 成本报告
- **事实提取**：保存章节后自动触发，结构化 Fact 写入档案（冲突告警 + 审计轨迹可回滚）
- **记忆管线**：实体摘要/细节 + 时间/情感上下文，预算按意图参与分配
- **LLM 与 Agent**：writer / reviewer / extractor / outliner / distiller 五角色，每角色可独立绑定 LLM 配置，未绑定回退全局默认
- **安全**：API 密钥只存本地 `data/_config/llm-config.json`（脱敏出接口）；路径白名单防穿越、CORS 收紧、防 SSRF、副作用接口一律 POST/PUT/DELETE

## 技术架构

| 层 | 技术 |
|---|---|
| 后端 | Rust workspace（6 crates：`pensoul-domain / graph / constraints / memory / infra / app`），axum HTTP API，监听 `127.0.0.1:3001` |
| 前端 | React 18 + TypeScript + Vite，端口 `1420`，`/api` 代理到后端 |
| 数据 | `NovelOntology` 正典 JSON（`data/projects/<project_id>/pensoul-project.json`），配置存 `data/_config/` |

## 快速开始

前置：Rust 工具链（`~/.cargo/bin`）、Node.js、一个可用的 LLM API（配置见下）。

```bash
# 1. 启动后端（首次构建较慢）
cargo run --bin pensoul-server        # http://127.0.0.1:3001

# 2. 启动前端（另开终端）
npm install
npm run dev                           # http://127.0.0.1:1420
```

配置 LLM：打开前端「设定 → LLM 配置」新增模型（密钥仅存本机 `data/_config/llm-config.json`），再到「设定 → Agent 模型」为各角色绑定模型（不绑定则回退全局默认）。

## 项目结构

```
crates/
  pensoul-domain        # 领域模型：NovelOntology 正典、实体、ID
  pensoul-graph         # 实体图谱（派生状态）
  pensoul-constraints   # 约束引擎（硬规则 + 状态机门控）
  pensoul-memory        # 记忆管线（检索/评分/摘要）
  pensoul-infra         # 基础设施：LLM 客户端与配置、事件总线
  pensoul-app           # 应用层：HTTP API、状态管理、创作管线命令
src/                    # 前端（React + TS + Vite）
  views/                # 作品库/仪表盘/萌芽/图谱/大纲/笔耕/约束/设定
data/                   # 运行时数据（不入库：密钥、项目正典）
agents.md               # AI Agent 协作规范（提交/推送/实现规则）
```

## 文档

设计稿、调研与会话工作日志**仅存本地 `docs/` 目录，不随仓库推送**（.gitignore 排除）：
- `docs/DESIGN.md`、`docs/REDESIGN.md`：设计体系与 2.0 重设计路线图
- `docs/design/`：知识库化重构设计（P0–P6 冻结稿）
- `docs/session-logs/`：会话工作日志（跨会话进度的唯一事实来源）

## 许可证

MIT
