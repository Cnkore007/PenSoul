// distill.rs — 书籍蒸馏：风格配方管道（P3）
// 语料摄取(txt/md/epub/pdf) → 多维度风格分析(Distiller Agent) → 风格画像
// → 落盘作品级 StyleRecipe(_config/style-recipe.json) → 注入写作/审校提示词
//
// 版权红线（设计已确认）：
// - 配方只含抽象特征/规则/统计，绝不保留原书句子（无风格锚）；
// - 提示词强制禁止复制原书表达；多书混合降低单一依赖。

use axum::extract::{Form, State};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::agent::AgentRole;
use crate::commands::llm::{build_llm_request, llm_client};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_infra::llm::LlmMessage;

// ---- 数据模型 ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSource {
    pub id: String,
    pub title: String,
    pub format: String,
    pub chars: usize,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimFeature {
    pub dimension: String,
    pub features: Vec<String>,
}

/// 作品级风格配方（落盘 _config/style-recipe.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StyleRecipe {
    pub books: Vec<BookSource>,
    /// 风格强度 0.3 ~ 1.0（防过拟合）
    pub strength: f32,
    pub dimensions: Vec<DimFeature>,
    /// 跨维度「写作基因」
    pub genes: Vec<String>,
    /// 禁用清单（该书绝不会出现的写法，兼作反 AI 味）
    pub bans: Vec<String>,
    pub generated_at: String,
    pub model: String,
}

// ---- 文件系统 ----
// 2026-08-13 用户决策：语料/配方按项目隔离（作品级），不再全局共享

/// 项目配置目录：data/projects/<project_id>/_config（作品级）
fn project_config_dir(base_dir: &str, project_id: &str) -> std::path::PathBuf {
    std::path::Path::new(base_dir)
        .join("projects")
        .join(project_id)
        .join("_config")
}

fn corpus_dir(base_dir: &str, project_id: &str) -> std::path::PathBuf {
    project_config_dir(base_dir, project_id).join("distill-corpus")
}

fn corpus_meta_path(base_dir: &str, project_id: &str) -> std::path::PathBuf {
    project_config_dir(base_dir, project_id).join("distill-corpus.json")
}

fn recipe_path(base_dir: &str, project_id: &str) -> std::path::PathBuf {
    project_config_dir(base_dir, project_id).join("style-recipe.json")
}

/// 从状态取出 (base_dir, 当前项目 id)；未打开项目时报错
fn current_project(state: &AppState) -> Result<(String, String), ApiError> {
    let project_id = state
        .ontology
        .as_ref()
        .ok_or(ApiError::bad_request("没有打开的项目"))?
        .project_id
        .as_str()
        .to_string();
    Ok((state.base_dir.clone(), project_id))
}

fn load_corpus_meta(base_dir: &str, project_id: &str) -> Vec<BookSource> {
    std::fs::read_to_string(corpus_meta_path(base_dir, project_id))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_corpus_meta(base_dir: &str, project_id: &str, books: &[BookSource]) -> Result<(), ApiError> {
    std::fs::create_dir_all(project_config_dir(base_dir, project_id))
        .map_err(|e| ApiError::internal(format!("语料配置目录创建失败: {e}")))?;
    std::fs::write(
        corpus_meta_path(base_dir, project_id),
        serde_json::to_string_pretty(books).map_err(|e| ApiError::internal(e.to_string()))?,
    )
    .map_err(|e| ApiError::internal(format!("语料元数据保存失败: {e}")))
}

/// 加载风格配方（无则 None）；项目级路径
pub(crate) fn load_style_recipe(base_dir: &str, project_id: &str) -> Option<StyleRecipe> {
    std::fs::read_to_string(recipe_path(base_dir, project_id))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// 配方注入文本（写作/审校提示词通用）
pub(crate) fn recipe_injection_text(recipe: &StyleRecipe) -> String {
    let strength_pct = (recipe.strength * 100.0).round() as i32;
    let mut parts = Vec::new();
    parts.push(format!(
        "## 风格配方（书籍蒸馏，强度 {strength_pct}%，来源 {} 本）",
        recipe.books.len()
    ));
    for dim in &recipe.dimensions {
        parts.push(format!(
            "{}：{}",
            dim.dimension,
            dim.features.join("；")
        ));
    }
    if !recipe.genes.is_empty() {
        parts.push(format!("写作基因：{}", recipe.genes.join("；")));
    }
    if !recipe.bans.is_empty() {
        parts.push(format!("禁用（本书绝不会出现）：{}", recipe.bans.join("；")));
    }
    parts.push("版权约束：以上为抽象风格规律，禁止复制任何原书句子或原文片段。".to_string());
    parts.join("\n")
}

// ---- 格式解析 ----

/// 依据格式解析原始字节为纯文本
fn parse_text(_title: &str, format: &str, bytes: &[u8]) -> Result<String, ApiError> {
    match format {
        "txt" | "md" | "markdown" => Ok(String::from_utf8_lossy(bytes).into_owned()),
        "epub" => parse_epub(bytes),
        "pdf" => parse_pdf(bytes),
        other => Err(ApiError::bad_request(format!(
            "不支持的格式: {other}（支持 txt / md / epub / pdf）"
        ))),
    }
}

/// epub：zip 解压 → 按文件名顺序读 xhtml → 剥离标签
fn parse_epub(bytes: &[u8]) -> Result<String, ApiError> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ApiError::bad_request(format!("epub 解压失败: {e}")))?;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ApiError::bad_request(format!("epub 读取失败: {e}")))?;
        let name = file.name().to_string().to_lowercase();
        if name.ends_with(".xhtml") || name.ends_with(".html") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| ApiError::bad_request(format!("epub 内容读取失败: {e}")))?;
            files.push((name, buf));
        }
    }
    if files.is_empty() {
        return Err(ApiError::bad_request("epub 中未找到任何 HTML 正文"));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut text = String::new();
    for (_name, buf) in files {
        let raw = String::from_utf8_lossy(&buf);
        // 剥离标签与常见实体
        let stripped = strip_html_tags(&raw);
        text.push_str(&stripped);
        text.push('\n');
    }
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}'))
        .collect();
    Ok(cleaned)
}

/// 简易 HTML 标签剥离（epub xhtml）
fn strip_html_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for c in raw.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// pdf：lopdf 按页提取文本
fn parse_pdf(bytes: &[u8]) -> Result<String, ApiError> {
    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| ApiError::bad_request(format!("pdf 解析失败: {e}")))?;
    let page_count = doc.get_pages().len();
    let mut text = String::new();
    for page_no in 1..=page_count as u32 {
        if let Ok(t) = doc.extract_text(&[page_no]) {
            text.push_str(&t);
            text.push('\n');
        }
    }
    if text.trim().is_empty() {
        return Err(ApiError::bad_request(
            "pdf 未能提取到文本（可能是扫描版/图片型 PDF，请转为 txt 后再试）",
        ));
    }
    Ok(text)
}

/// 全本多点采样（2026-08-13 用户决策：阅读完全本后再采样）。
/// 全书按字符等分为 6 区段，每段取约 budget/6 字并标注区段位置，
/// 覆盖开篇/中段/结尾的调性，避免只取开头（旧实现在上传时截取 12000 字并丢弃全本）。
/// 全书不超过预算时直接返回全文。
fn sample_across_book(text: &str, budget: usize) -> String {
    let total = text.chars().count();
    if total <= budget {
        return text.to_string();
    }
    const SEGMENTS: usize = 6;
    let seg_budget = (budget / SEGMENTS).max(1);
    let chars: Vec<char> = text.chars().collect();
    let seg_size = total / SEGMENTS;
    let mut out = String::with_capacity(budget + 64);
    for i in 0..SEGMENTS {
        let start = i * seg_size;
        let end = (start + seg_budget).min(total);
        let label = match i {
            0 => "【开篇】",
            x if x == SEGMENTS - 1 => "【结尾】",
            _ => "【中段】",
        };
        out.push_str(label);
        out.extend(&chars[start..end]);
        out.push('\n');
    }
    out
}

// ---- API ----

#[derive(Deserialize)]
pub struct CorpusParams {
    pub title: String,
    pub format: String,
    /// base64 编码的原始文件字节
    pub content_b64: String,
    /// 权重（0-1，默认 1.0）
    pub weight: Option<String>,
}

#[derive(Deserialize)]
pub struct AnalyzeParams {
    /// 逗号分隔 id=weight，如 a=0.8,b=0.4（可选，缺省用入库权重）
    pub weights: Option<String>,
}

#[derive(Deserialize)]
pub struct RecipeUpdateParams {
    /// 风格强度 0.3-1.0
    pub strength: Option<String>,
}

/// 摄取一本语料（解析 → 保存全本 → 存盘，不立即分析）
/// 2026-08-13 用户决策：保存全本（不再截取开头 12000 字），分析时再全本多点采样
pub async fn add_corpus(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<CorpusParams>,
) -> Result<String, ApiError> {
    use base64::Engine as _;
    let title = params.title.trim();
    if title.is_empty() {
        return Err(ApiError::bad_request("书名不能为空"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(params.content_b64.trim())
        .map_err(|e| ApiError::bad_request(format!("base64 解码失败: {e}")))?;
    let text = parse_text(title, params.format.trim(), &bytes)?;
    let full_text = text.trim();
    if full_text.is_empty() {
        return Err(ApiError::bad_request("语料解析后为空"));
    }
    let weight = match params.weight.as_deref() {
        None | Some("") => 1.0,
        Some(w) => w
            .parse::<f32>()
            .map_err(|_| ApiError::bad_request("权重必须是 0-1 的数字"))?
            .clamp(0.1, 1.0),
    };

    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let id = uuid::Uuid::new_v4().to_string();
    let dir = corpus_dir(&base_dir, &project_id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| ApiError::internal(format!("语料目录创建失败: {e}")))?;
    std::fs::write(dir.join(format!("{id}.txt")), full_text)
        .map_err(|e| ApiError::internal(format!("语料保存失败: {e}")))?;

    let book = BookSource {
        id: id.clone(),
        title: title.to_string(),
        format: params.format.trim().to_string(),
        chars: full_text.chars().count(),
        weight,
    };
    let mut books = load_corpus_meta(&base_dir, &project_id);
    books.push(book);
    save_corpus_meta(&base_dir, &project_id, &books)?;
    serde_json::to_string(&serde_json::json!({
        "id": id,
        "chars": full_text.chars().count(),
        "full_text": true,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 语料列表（不含正文）
pub async fn list_corpus(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let books = load_corpus_meta(&base_dir, &project_id);
    serde_json::to_string(&books).map_err(|e| ApiError::internal(e.to_string()))
}

/// 删除一本语料（同时清理正文文件）
/// 安全约束：id 必须是 UUID 格式（上传时生成），且必须存在于元数据中，
/// 防止路径穿越删除数据目录外文件；删除失败显式报错而非静默吞掉。
pub async fn delete_corpus(
    State(state): State<Arc<RwLock<AppState>>>,
    axum::extract::Query(params): axum::extract::Query<DeleteCorpusParams>,
) -> Result<String, ApiError> {
    let id = params.id;
    // 白名单校验：语料 id 由 uuid::Uuid::new_v4() 生成，严格按 UUID 格式解析
    uuid::Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("语料 id 格式非法"))?;
    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let mut books = load_corpus_meta(&base_dir, &project_id);
    let exists = books.iter().any(|b| b.id == id);
    if !exists {
        return Err(ApiError::not_found("语料不存在"));
    }
    books.retain(|b| b.id != id);
    save_corpus_meta(&base_dir, &project_id, &books)?;
    std::fs::remove_file(corpus_dir(&base_dir, &project_id).join(format!("{id}.txt")))
        .map_err(|e| ApiError::internal(format!("语料文件删除失败: {e}")))?;
    Ok("ok".to_string())
}

#[derive(Debug, Deserialize)]
pub struct DeleteCorpusParams {
    pub id: String,
}

/// 风格分析：全部语料 → LLM(Distiller) → 7+1 维度 → 混合配方 → 落盘
pub async fn analyze(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<AnalyzeParams>,
) -> Result<String, ApiError> {
    let (base_dir, project_id, books) = {
        let state = state.read().await;
        let (base_dir, project_id) = current_project(&state)?;
        let mut books = load_corpus_meta(&base_dir, &project_id);
        if books.is_empty() {
            return Err(ApiError::bad_request("没有语料，请先上传书籍"));
        }
        // 可选权重覆盖
        if let Some(weights) = &params.weights {
            for w in weights.split(',') {
                let w = w.trim();
                if w.is_empty() {
                    continue;
                }
                let Some((id, weight)) = w.split_once('=') else {
                    continue;
                };
                if let Ok(weight) = weight.parse::<f32>() {
                    if let Some(book) = books.iter_mut().find(|b| b.id == id) {
                        book.weight = weight.clamp(0.1, 1.0);
                    }
                }
            }
            save_corpus_meta(&base_dir, &project_id, &books)?;
        }
        (base_dir, project_id, books)
    };

    // 组装语料（按权重降序，主风格在前）：
    // 2026-08-13 用户决策——每本书全本多点采样（开篇/中段/结尾均匀抽样），
    // 不再只取开头；正文缺失的书跳过并在结果中显式列出
    let mut corpus_text = String::new();
    let mut missing_books: Vec<String> = Vec::new();
    for book in &books {
        let path = corpus_dir(&base_dir, &project_id).join(format!("{}.txt", book.id));
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let sampled = sample_across_book(&text, 60_000);
                corpus_text.push_str(&format!(
                    "【书籍《{}》（权重 {:.1}，全书 {} 字，以下为全本多点采样）】\n{}\n\n",
                    book.title,
                    book.weight,
                    book.chars,
                    sampled
                ));
            }
            Err(e) => {
                missing_books.push(format!("《{}》（{e}）", book.title));
            }
        }
    }
    if missing_books.len() == books.len() {
        return Err(ApiError::bad_request(format!(
            "全部语料正文缺失，无法蒸馏：{}",
            missing_books.join("、")
        )));
    }

    let provider =
        crate::commands::agent::resolve(&base_dir, AgentRole::Distiller).map_err(|_| {
            ApiError::bad_request("蒸馏 Agent 未配置 LLM。请在「设定 → Agent 模型」绑定或配置默认 LLM。")
        })?;
    let client = llm_client(&provider);
    let user_content = corpus_text.chars().take(60_000).collect::<String>();

    // 调用 + 解析，LLM 偶发输出损坏 JSON 时自动重试一次（2026-08-13 实测修复）
    let parsed: serde_json::Value = {
        let mut parsed = None;
        for attempt in 0..2 {
            let request = build_llm_request(
                &provider,
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: user_content.clone(),
                }],
                distill_system_prompt(books.len()),
                true,
                // 蒸馏 JSON 体积大（7 维度 + 基因 + 禁用清单），给足输出空间防止截断
                8000,
            );
            let resp = client
                .complete(request)
                .await
                .map_err(|e| ApiError::internal(format!("蒸馏分析失败: {e}")))?;
            match pensoul_infra::llm::parse_llm_json::<serde_json::Value>(&resp.content) {
                Ok(v) => {
                    parsed = Some(v);
                    break;
                }
                Err(e) if attempt == 0 => {
                    eprintln!("[PenSoul][distill] 首次解析失败（LLM 偶发输出异常），重试: {e}");
                }
                Err(e) => return Err(ApiError::internal(format!("蒸馏响应解析失败: {e}"))),
            }
        }
        parsed.expect("循环必产生结果")
    };

    let dimensions: Vec<DimFeature> = parsed
        .get("dimensions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    Some(DimFeature {
                        dimension: d.get("dimension")?.as_str()?.to_string(),
                        features: d
                            .get("features")?
                            .as_array()?
                            .iter()
                            .filter_map(|f| f.as_str().map(|s| s.to_string()))
                            .collect(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let genes: Vec<String> = parsed
        .get("genes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let bans: Vec<String> = parsed
        .get("bans")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|b| b.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if dimensions.is_empty() {
        return Err(ApiError::internal("蒸馏结果为空（LLM 未返回有效维度）"));
    }

    let recipe = StyleRecipe {
        books,
        strength: 0.8,
        dimensions,
        genes,
        bans,
        generated_at: chrono::Utc::now().to_rfc3339(),
        model: provider.model_id,
    };
    std::fs::create_dir_all(project_config_dir(&base_dir, &project_id))
        .map_err(|e| ApiError::internal(format!("配方目录创建失败: {e}")))?;
    std::fs::write(
        recipe_path(&base_dir, &project_id),
        serde_json::to_string_pretty(&recipe).map_err(|e| ApiError::internal(e.to_string()))?,
    )
    .map_err(|e| ApiError::internal(format!("配方落盘失败: {e}")))?;

    serde_json::to_string(&serde_json::json!({
        "ok": true,
        "books": recipe.books.iter().map(|b| b.title.clone()).collect::<Vec<_>>(),
        "dimension_count": recipe.dimensions.len(),
        "gene_count": recipe.genes.len(),
        "ban_count": recipe.bans.len(),
        "strength": recipe.strength,
        "model": recipe.model,
        "missing_books": missing_books,
    }))
    .map_err(|e| ApiError::internal(e.to_string()))
}

/// 读取当前配方（无则返回空结构）；项目级路径
pub async fn get_recipe(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let recipe = load_style_recipe(&base_dir, &project_id).unwrap_or_default();
    serde_json::to_string(&recipe).map_err(|e| ApiError::internal(e.to_string()))
}

/// 调整风格强度（0.3-1.0；权重调整请重新分析）
pub async fn update_recipe(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<RecipeUpdateParams>,
) -> Result<String, ApiError> {
    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let mut recipe = load_style_recipe(&base_dir, &project_id)
        .ok_or(ApiError::bad_request("还没有风格配方，请先上传语料并分析"))?;
    if let Some(strength) = params.strength {
        let strength: f32 = strength
            .parse()
            .map_err(|_| ApiError::bad_request("强度必须是 0.3-1.0 的数字"))?;
        if !(0.3..=1.0).contains(&strength) {
            return Err(ApiError::bad_request("强度必须在 0.3 ~ 1.0 之间"));
        }
        recipe.strength = strength;
    }
    std::fs::write(
        recipe_path(&base_dir, &project_id),
        serde_json::to_string_pretty(&recipe).map_err(|e| ApiError::internal(e.to_string()))?,
    )
    .map_err(|e| ApiError::internal(format!("配方保存失败: {e}")))?;
    Ok("ok".to_string())
}

/// 删除当前项目的风格配方（清除蒸馏结果；语料不受影响）
pub async fn delete_recipe(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    let (base_dir, project_id) = {
        let state = state.read().await;
        current_project(&state)?
    };
    let path = recipe_path(&base_dir, &project_id);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok("ok".to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // 幂等：配方本就不存在时视为删除成功，前端可直接刷新为空
            Ok("ok".to_string())
        }
        Err(e) => Err(ApiError::internal(format!("配方删除失败: {e}"))),
    }
}

/// 蒸馏分析系统提示词（7+1 维度，无风格锚，版权红线内置）
fn distill_system_prompt(book_count: usize) -> String {
    let mix = if book_count > 1 {
        "多本书混合蒸馏：以权重最高的书为主风格，其余书作辅助风味；每条特征注明主要来源书。"
    } else {
        "单本书蒸馏：提炼该书的稳定风格规律。"
    };
    format!(
        "你是 PenSoul 的风格蒸馏师：从用户提供的书籍语料中提炼「可执行的写作风格配方」，只输出 JSON，不要任何解释。\n\
         分析维度（7+1）：\n\
         - 词汇指纹：高频词倾向、词汇丰富度、特有搭配\n\
         - 句法节奏：平均句长、句式比例、段落节奏\n\
         - 修辞偏好：比喻密度、排比/设问、具体-抽象比\n\
         - 叙事手法：描写优先级（听觉>视觉>触觉?）、对话写法、视角\n\
         - 节奏与悬念：章节结尾手法、信息释放节奏\n\
         - 母题与主题：反复出现的主题词、价值取向\n\
         - 情感基调：整体情绪色调、冷热控制\n\
         - 写作基因：3-5 项跨维度可执行规律（如「短句收尾+冰山式留白」）\n\
         {mix}\n\
         输出格式：\n\
         {{\n\
           \"dimensions\": [{{\"dimension\": \"词汇\", \"features\": [3-5 条可执行特征]}}, ...],\n\
           \"genes\": [3-5 条写作基因],\n\
           \"bans\": [5-10 条「该书绝不会出现的写法」，兼作反 AI 味清单]\n\
         }}\n\
         铁律：\n\
         1. 只提炼抽象规律，禁止摘抄或改写原文句子、禁止保留任何原书片段；\n\
         2. 特征必须可执行（能指导 AI 写作），不要空泛形容词；\n\
         3. 如语料不足（<3000 字）或混杂，在特征中如实注明局限。"
    )
}

// ---- 单元测试 ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn strip_html_removes_tags() {
        let raw = "<html><body><p>第一段 &amp; 第二段</p><div>第三</div></body></html>";
        assert_eq!(strip_html_tags(raw), "第一段 & 第二段第三");
    }

    #[test]
    fn sample_short_text_returns_all() {
        let text = "短文本。".repeat(100);
        assert_eq!(
            sample_across_book(&text, 5000).chars().count(),
            text.chars().count()
        );
    }

    #[test]
    fn sample_long_text_covers_beginning_and_end() {
        let text = "很长的一段话，用于测试全本多点采样是否覆盖开篇与结尾。".repeat(5000);
        let sampled = sample_across_book(&text, 12000);
        assert!(
            sampled.chars().count() <= 12000 + 64,
            "采样应受上限约束: {}",
            sampled.chars().count()
        );
        assert!(sampled.contains("【开篇】"), "应包含开篇标注");
        assert!(sampled.contains("【结尾】"), "应包含结尾标注");
        assert!(sampled.matches("【中段】").count() >= 4, "应包含多个中段采样");
    }

    #[test]
    fn recipe_injection_contains_disclaimer() {
        let recipe = StyleRecipe {
            books: vec![BookSource {
                id: "a".into(),
                title: "测试书".into(),
                format: "txt".into(),
                chars: 100,
                weight: 1.0,
            }],
            strength: 0.8,
            dimensions: vec![DimFeature {
                dimension: "句法节奏".into(),
                features: vec!["短句收尾".into()],
            }],
            genes: vec!["冰山式留白".into()],
            bans: vec!["不使用感叹号堆砌".into()],
            generated_at: "now".into(),
            model: "m".into(),
        };
        let text = recipe_injection_text(&recipe);
        assert!(text.contains("风格配方"));
        assert!(text.contains("短句收尾"));
        assert!(text.contains("禁止复制任何原书句子"), "必须内置版权红线");
    }

    #[test]
    fn parse_epub_extracts_text() {
        // 构造最小 epub：zip 内含两个 xhtml
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("chapter1.xhtml", opts).unwrap();
        zip.write_all("<html><body><p>第一段落内容。</p><p>第二段落内容。</p></body></html>".as_bytes())
            .unwrap();
        zip.start_file("chapter2.xhtml", opts).unwrap();
        zip.write_all("<html><body><p>第三段落内容。</p></body></html>".as_bytes()).unwrap();
        let buf = zip.finish().unwrap().into_inner();

        let text = parse_epub(&buf).unwrap();
        assert!(text.contains("第一段落内容"), "应提取 xhtml 文本: {text}");
        assert!(text.contains("第三段落内容"), "应按文件名顺序提取全部章节");
        assert!(!text.contains("<p>"), "应剥离 HTML 标签");
    }

    #[test]
    fn parse_pdf_rejects_garbage() {
        let err = parse_pdf(b"not a pdf at all");
        assert!(err.is_err(), "非 PDF 字节应报错");
    }

    #[test]
    fn distill_prompt_handles_multi_book() {
        let single = distill_system_prompt(1);
        assert!(single.contains("单本书蒸馏"));
        let multi = distill_system_prompt(3);
        assert!(multi.contains("多本书混合蒸馏"));
        assert!(multi.contains("禁止摘抄或改写原文句子"));
    }
}
