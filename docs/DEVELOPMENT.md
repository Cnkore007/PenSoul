# PenSoul 开发指南

> 最后更新：2026-08-01 · 对齐 2026-08-01 代码状态 · 面向在本仓库工作的开发者与 AI Agent

## 一、环境要求

- macOS ARM64（Apple Silicon）
- Rust 稳定版 + cargo（构建工具优先 brew 安装版本）
- Node.js + npm（前端目前用 npm 管理，见 `package-lock.json`；如引入 pnpm 需保持 lock 文件一致）
- Tauri 2 系统依赖（WebView 等，按 Tauri 官方 macOS 准备）

## 二、构建与运行

```bash
# 安装前端依赖
npm install

# 开发模式启动 Tauri（同时构建 Rust 后端与前端）
npm run tauri dev

# 仅构建 Rust（检查编译）
cargo build

# 仅构建前端
npm run build
```

开发模式下 `data/` 位于工作区根（`AppState::new` 传入 base_dir），生产打包后数据目录由 Tauri 资源路径决定。

## 三、测试

```bash
# 全部 crate 测试
cargo test

# 应用层关键测试
cargo test -p pensoul-app
```

重点测试资产：

| 测试 | 位置 | 覆盖 |
|---|---|---|
| 验收场景（#72-75） | `crates/pensoul-app/tests/e2e_scenarios.rs` | 讨论→脉络→展开→连写的端到端流程 |
| 领域测试 | `crates/pensoul-app/tests/domain.rs` | 本体/领域行为 |
| 引擎流程测试 | `crates/pensoul-app/tests/engine_flow.rs` | Harness 阶段/门控/WAL 流程 |
| 各 crate 单测 | 各 `src/*.rs` 内 `#[cfg(test)]` | 记忆预算、CDA BFS、一致性规则、并发冲突等 |

测试后主动清理产物（`target/debug/deps` 等），不保留临时 fixture。

## 四、代码规范（全局 AGENTS.md 摘要）

1. **语言**：注释、提交信息、文档一律中文；标识符保持英文。
2. **提交信息**：`<类型>: <中文描述>`，如 `feat: 新增用户导出功能`、`fix: 修复分页计算溢出`。
3. **Rust**：rustfmt 默认格式 + clippy 检查；不手写 unsafe（除非必要且注释说明）；优先 `Result` + `?`，`unwrap()` 仅限"失败则程序不该继续"场景。
4. **文件组织**：单文件不超过 500 行，超了拆模块；`src/` 按功能划分，不把代码全塞 main.rs。
5. **依赖**：先想标准库；`[dev-dependencies]` 只放测试真需要的；定期 `cargo audit`。
6. **清理**：临时文件、测试产物、日志用完即删；`target/`、`node_modules/` 不入 git。
7. **安全**：不硬编码密钥；API key 只在 `data/_config/api-keys.json`，绝不写进代码/文档/提交。

## 五、如何新增一条 IPC 命令

三步走，缺一不可：

### 第 1 步：后端命令

在 `crates/pensoul-app/src/commands/` 下（或既有模块）新增：

```rust
#[tauri::command]
pub async fn my_new_command(state: tauri::State<'_, AppState>, foo: String) -> Result<serde_json::Value, String> {
    // 逻辑
    Ok(serde_json::json!({ "bar": 1 }))
}
```

在 `crates/pensoul-app/src/main.rs` 的 `invoke_handler![...]` 中注册，同时在 `commands/mod.rs` 导出。

### 第 2 步：前端封装

在 `src/ipc.ts` 新增封装（camelCase 参数，内部转 snake_case 传给后端；返回统一转 camelCase 供视图消费），并在 `src/types.ts` 补类型。

### 第 3 步：更新契约文档

同步更新 `docs/IPC-CONTRACT.md`（命令表 + 事件协议）。这是 `agent.md` 的强制要求，防止文档再次失同步。

## 六、如何新增事件协议

长跑命令的进度推送统一走「后端驱动 + 实时 emit + 环形缓冲」模式：

1. 定义事件结构（serde::Serialize），如 `DiscussionEvent`、`DistillPhaseEvent`。
2. 挂在 `AppState` 的控制面上（如 `DiscussionControl`），事件先入缓冲再 `app.emit("<事件名>", ev)`。
3. 前端 `listen` 渲染；提供 `get_*_state` 命令供切页后重放。
4. 更新 `docs/IPC-CONTRACT.md` 的事件协议章节。

事件名命名惯例：`<领域>-event`（harness-event / discussion-event）或 `<领域>-phase`（distill-phase / book-distill-phase）。

## 七、架构注意事项（防踩坑）

1. **不要绕过集成层**：章节保存后必须走 `on_chapter_saved`，不要手写"更新记忆/影响图"逻辑。
2. **顺序语义用 `chapter_no`**：记忆/影响图/一致性索引一律用序号，前端字符串 ID 需先解析；解析不了显式跳过，禁止静默当第 0 章。
3. **不要用旧模拟器**：新功能走 `pipeline/`，`harness_exec.rs` 是遗留兼容入口。
4. **不要用 ModelRouter**：应用层 LLM 调用统一走 `llm_helper` + `llm_profile`；如要启用 ModelRouter 需先接入 llm_helper。
5. **LLM 产物要防"形态多变"**：反序列化用 `#[serde(default)]` 容错；需要修 JSON 用 `json_fix`。
6. **不要动 `data/` 与密钥**：`data/_config/api-keys.json` 含真实密钥，任何文档/提交不得包含。
7. **工作区是脏的**：用户有大量未提交改动（pipeline、book_distill、前端视图等），编辑前先 `git status`，不要覆盖、不要擅自提交。

## 八、修改文档的强制约定

见仓库根目录 `agent.md`。核心：

- 工作前先读 `docs/PROGRESS.md` 与 `agent.md`；
- 每轮工作后把进度/问题/设计思考追加到 `docs/PROGRESS.md`；
- 代码变更后同步 `docs/IPC-CONTRACT.md` 及相关架构文档；
- 文档必须与代码事实一致，禁止凭记忆写过期内容。
