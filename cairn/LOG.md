# PenSoul2.0 路标石堆日志

> 由每日 03:00 沟通记录自动化初始化，倒序记录实质进展；结论沉淀进 `cairn/<topic>.md`。

## 2026-08-10 · 中文项目名、萌芽诘问式对话与 AI 写作

- 作品库支持中文名：标题与 ID 分离，ID 自动生成（英文/数字片段，纯中文回退 `project-随机`）；列表接口返回 `{project_id, title}`；日志 0102。
- 萌芽页对话式创作工作台：综合/结构/人物/世界观四视角，LLM 整理生成世界观与大纲提案，建议制确认写入正典；`/api/sprout/*`（会话/对话/生成/应用/拒绝/清空），历史持久化 100 条；日志 0112。
- 诘问式一问一答：一次只问一个问题、提炼后追问，顺序 高概念→主角→核心冲突→世界观→基调→结局；`POST /api/sprout/start`；kimi-k3 实测两轮；日志 0122。
- AI 写作：笔耕页生成初稿/续写（大纲弧+前后章摘要+世界观+角色+伏笔+核心概念上下文；建议制，保存才落盘+一致性评分）；修元叙述与偏离设定问题；日志 0148。
- 遗留：纯中文名 ID 不可读（可接拼音库）；一次抛多问需回复校验；写作上下文未接记忆检索管线。
- 指针：`docs/session-logs/2026-08-10-{0102,0112,0122,0148}-*.md`、`crates/pensoul-app`（sprout 接口）。

## 2026-08-09 · 对抗性审计与全量修复、LLM 全局配置模块

- 对抗性审计：P0 任意目录删除+跨域无鉴权（路径穿越、CORS Any、0.0.0.0:3001）、PUT 契约不一致全线失效、新增实体不落盘；P1 约束引擎空壳、记忆/工作流未接线、LLM 客户端硬伤、派生状态漂移；P2 文档漂移、非 git、无测试。全部修复：路径白名单、CORS 收紧、127.0.0.1、Form 解析、正典+落盘、5 条真实约束+状态机门控、on_chapter_saved、LLM/ApiKeys 接线、记忆检索真实填充、git init（96666c4）、拆 App.tsx、清 target 5.2GB；29→48 测试全绿。
- 会话日志规范固化（agents.md 强制）：docs/session-logs/YYYY-MM-DD-HHMM-主题.md + INDEX，新会话按 agents.md→INDEX→日志读取。
- LLM 全局配置模块（pensoul-infra::llm::config，data/_config/llm-config.json）：增删改测、默认配置、拉取模型列表、密钥脱敏、上下文检测、详细参数（temperature/top_p/stop/json_mode/thinking_budget/timeout）。
- 文档与参数链路：按模型定位官方文档→抓取→优先 llms.txt 索引→Markdown 表格摘取参数（实测 kimi-k3：1M 上下文、思考模式识别）；JS 渲染页回落「未识别+文档线索」。
- 坑：表单空字符串 vs Option<u32> 400（宽松解析+commands/params.rs 统一）；中文文本切片 panic；伏笔 "null" 字符串三态解析；跨页面同型问题一并修。
- 删除内置模型档案（models.rs），建档继承来源配置；界面文案中文化（labels.ts）。
- 遗留：旧后端二进制需重启；旧 api-keys.json 停用需重录；JS 渲染文档深度解析待做。
- 指针：agents.md、docs/session-logs/INDEX.md、data/_config/llm-config.json。
