// remote.rs — 远程能力：抓取模型官方文档、拉取供应商模型列表

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// 文档抓取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDoc {
    pub model_id: String,
    pub url: String,
    pub title: String,
    pub description: String,
    pub text_preview: String,
    pub saved_file: String,
    pub fetched_at: String,
}

/// 供应商模型条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteModel {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

/// 从文档中提取的模型参数（启发式，供导入配置时参考）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelDocParams {
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub thinking_supported: Option<bool>,
    pub notes: Vec<String>,
    /// 摘取参数所用的文档来源（页面标题 + URL）
    pub sources: Vec<DocSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocSource {
    pub title: String,
    pub url: String,
}

/// 根据供应商文档根地址，构造模型专属文档链接
pub fn suggest_model_doc_url(doc_url: Option<&str>, model_id: &str) -> Option<String> {
    let root = doc_url?.trim_end_matches('/');
    if root.is_empty() {
        return None;
    }
    Some(format!("{root}/models/{}", sanitize_filename(model_id)))
}

/// 从抓取的 HTML 中启发式提取模型参数
pub fn extract_model_params(html: &str, model_id: &str) -> ModelDocParams {
    let mut params = ModelDocParams::default();
    let text = extract_doc_info(html).2;

    // 优先在模型 ID 附近聚焦查找，避免无关数字误报
    let focus = if let Some(rel) = text.find(model_id) {
        let mut start = rel.saturating_sub(150);
        while start < rel && !text.is_char_boundary(start) {
            start += 1;
        }
        let mut end = (rel + 300).min(text.len());
        while end > rel && !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[start..end]
    } else {
        &text
    };

    // 上下文窗口：找 "xxxK context" / "上下文窗口 xxxK" 之类的模式
    let k_patterns = [
        "context window",
        "context",
        "上下文窗口",
        "上下文长度",
    ];
    for pattern in k_patterns {
        if let Some(k) = find_window_k(focus, pattern) {
            params.context_window = Some(k);
            break;
        }
    }

    // 最大输出：找 "max output ... N" / "最大输出 ... N"
    if let Some(tokens) = find_max_output(focus) {
        params.max_output_tokens = Some(tokens);
    }

    // 思考模式支持
    let lower = focus.to_lowercase();
    let mentions_thinking = lower.contains("thinking")
        || lower.contains("思考模式")
        || lower.contains("reasoner")
        || lower.contains("推理模型");
    if mentions_thinking {
        params.thinking_supported = Some(true);
    }

    // 记录找到的规模提示，并把文档中明确出现的规模转为上下文窗口候选
    for token in ["1m", "256k", "200k", "128k", "64k", "32k"] {
        if lower.contains(token) {
            params.notes.push(format!("文档中出现 {token} 上下文标识"));
            if params.context_window.is_none() {
                let value = match token {
                    "1m" => 1_000_000,
                    "256k" => 256_000,
                    "200k" => 200_000,
                    "128k" => 128_000,
                    "64k" => 64_000,
                    _ => 32_000,
                };
                params.context_window = Some(value);
            }
        }
    }
    if lower.contains("reasoner") {
        params.notes.push("文档标注为推理/思考模型".to_string());
    }
    if model_id.contains("reasoner") || model_id.contains("thinking") {
        params.notes.push("模型 ID 包含 reasoner/thinking 标识".to_string());
    }
    // 中文单位兜底（"100 万 token 上下文窗口"）
    if params.context_window.is_none() {
        params.context_window = extract_context_from_description(focus);
    }
    params.notes.dedup();
    params
}

/// 从 Markdown 表格行解析模型 ID 与描述：| `kimi-k3` | 描述 |
pub fn parse_markdown_table_row(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let cells: Vec<&str> = trimmed
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    if cells.len() < 2 {
        return None;
    }
    let model_id = cells[0].trim_matches('`').trim().to_string();
    if model_id.is_empty() || model_id.contains(' ') {
        return None;
    }
    Some((model_id, cells[1].to_string()))
}

/// 在模型列表 Markdown 中查找指定模型的描述
pub fn find_model_description(markdown: &str, model_id: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        let (id, description) = parse_markdown_table_row(line)?;
        if id == model_id {
            Some(description)
        } else {
            None
        }
    })
}

/// 从描述/文本中提取上下文规模：支持 "256k"、"128K"、"100 万 token" 等写法
pub fn extract_context_from_description(desc: &str) -> Option<u32> {
    // 优先结构化模式：数字 + k/K/m/M 或 万
    if let Some(value) = parse_k_value(desc) {
        return Some(value);
    }
    // "100 万 token 上下文窗口"
    let lower = desc.to_lowercase();
    for unit in ["万", "w"] {
        let mut search_from = 0usize;
        while let Some(rel_rel) = lower[search_from..].find(unit) {
            let rel = search_from + rel_rel;
            let before = lower[..rel].trim_end();
            let digits: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit() || *c == ',')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            // 跳过小数（如 "2.8 万亿参数" 不应解析为 8 万）
            let after_digits = &before[..before.len() - digits.len()];
            if digits.is_empty() || after_digits.ends_with('.') {
                search_from = rel + unit.len();
                continue;
            }
            let number: u64 = digits.replace(',', "").parse().ok()?;
            let multiplier = 10_000;
            return (number * multiplier).try_into().ok();
        }
    }
    None
}

/// 解析 llms.txt / Markdown 链接列表：返回 (标题, URL)
pub fn parse_markdown_links(content: &str) -> Vec<(String, String)> {
    let mut links = Vec::new();
    for line in content.lines() {
        // [标题](url)
        if let Some(open) = line.find('[') {
            if let Some(close) = line[open..].find(']') {
                let title = line[open + 1..open + close].trim().to_string();
                let rest = &line[open + close..];
                if let Some(paren_open) = rest.find('(') {
                    if let Some(paren_close) = rest[paren_open..].find(')') {
                        let url = rest[paren_open + 1..paren_open + paren_close].trim().to_string();
                        if !title.is_empty() && url.starts_with("http") {
                            links.push((title, url));
                        }
                    }
                }
            }
        }
    }
    links
}

/// 从文档索引中挑选与模型相关的页面：模型专属页优先，其次模型列表页与定价页
pub fn pick_relevant_pages(
    links: &[(String, String)],
    model_id: &str,
) -> Vec<(String, String)> {
    let model_lower = model_id.to_lowercase();
    let mut model_pages = Vec::new();
    let mut list_pages = Vec::new();
    let mut pricing_pages = Vec::new();

    for (title, url) in links {
        let haystack = format!("{title} {url}").to_lowercase();
        if haystack.contains(&model_lower) {
            model_pages.push((title.clone(), url.clone()));
        } else if haystack.contains("pricing") || title.contains("定价") {
            pricing_pages.push((title.clone(), url.clone()));
        } else if haystack.contains("models") && (haystack.contains(".md") || title.contains("模型")) {
            list_pages.push((title.clone(), url.clone()));
        }
    }

    let mut result = Vec::new();
    result.extend(model_pages);
    result.extend(list_pages);
    result.extend(pricing_pages);
    result.dedup_by(|a, b| a.1 == b.1);
    result
}

/// 在文本中查找 "N K" 形式的上下文规模
fn find_window_k(text: &str, pattern: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(pattern) {
        let idx = search_from + rel;
        // 从模式之后的内容开始查找规模数值
        let content_start = idx + pattern.len();
        let mut end = (content_start + 64).min(lower.len());
        while end > content_start && !lower.is_char_boundary(end) {
            end -= 1;
        }
        let window = &lower[content_start..end];
        if let Some(k) = parse_k_value(window) {
            return Some(k);
        }
        // 模式前面也可能有数值（如 "128K context"）
        let mut before_start = idx.saturating_sub(40);
        while before_start < idx && !lower.is_char_boundary(before_start) {
            before_start += 1;
        }
        let before = &lower[before_start..idx];
        if let Some(k) = parse_k_value(before) {
            return Some(k);
        }
        search_from = idx + pattern.len();
    }
    None
}

/// 从一段文本中解析 "128k"/"1m" 等规模
fn parse_k_value(snippet: &str) -> Option<u32> {
    let bytes = snippet.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b',') {
                i += 1;
            }
            let number: u64 = snippet[start..i].replace(',', "").parse().ok()?;
            let rest = &snippet[i..];
            if rest.starts_with('k') || rest.starts_with('K') {
                return (number * 1000).try_into().ok();
            }
            if rest.starts_with('m') || rest.starts_with('M') {
                return (number * 1_000_000).try_into().ok();
            }
            if number >= 1000 {
                return Some(number as u32);
            }
            continue;
        }
        i += 1;
    }
    None
}

/// 查找最大输出 token
fn find_max_output(text: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    for pattern in ["max output", "maximum output", "最大输出", "输出上限"] {
        if let Some(rel) = lower.find(pattern) {
            let start = rel + pattern.len();
            let mut end = (start + 60).min(lower.len());
            while end > start && !lower.is_char_boundary(end) {
                end -= 1;
            }
            let window = &lower[start..end];
            if let Some(k) = parse_k_value(window) {
                return Some(k);
            }
        }
    }
    None
}

/// 校验并规范化文档 URL（仅允许 http/https，拒绝回环/私网地址防 SSRF）
/// 注：仅能拦截字面 IP 与 localhost 形式的回环/内网地址；
/// 域名若解析到内网（DNS 重绑定）无法静态判定，需依赖目标站点本身的网络隔离。
pub fn validate_doc_url(input: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(input).map_err(|e| format!("URL 格式错误: {e}"))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("只允许 http/https 文档地址".to_string()),
    }
    let host = url.host_str().ok_or("URL 缺少主机名")?;
    let lower = host.to_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") {
        return Err("不允许访问本机地址（localhost）".to_string());
    }
    // 直接按 host 枚举判定 IP（对 IPv6 可靠，避免 host_str 括号/解析歧义）
    if let Some(host) = url.host() {
        let blocked = match host {
            url::Host::Ipv4(v4) => is_blocked_v4(v4),
            url::Host::Ipv6(v6) => is_blocked_v6(v6),
            url::Host::Domain(_) => false,
        };
        if blocked {
            return Err("不允许访问内网/本机/保留地址".to_string());
        }
    }
    Ok(url.to_string())
}

/// 判定 IPv4 是否为应拒绝的地址（回环/私网/链路本地/广播/未指定/组播）
fn is_blocked_v4(v4: std::net::Ipv4Addr) -> bool {
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_unspecified()
        || v4.is_multicast()
}

/// 判定 IPv6 是否为应拒绝的地址（回环/未指定/组播/链路本地/唯一本地）
fn is_blocked_v6(v6: std::net::Ipv6Addr) -> bool {
    v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        || (v6.segments()[0] & 0xffc0) == 0xfe80 // fe80::/10 链路本地
        || (v6.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7 唯一本地地址
}

/// 模型 ID 转安全文件名
pub fn sanitize_filename(model_id: &str) -> String {
    model_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 从 HTML 提取标题、描述与正文预览
pub fn extract_doc_info(html: &str) -> (String, String, String) {
    let mut title = extract_tag(html, "title").trim().to_string();
    let description = extract_meta(html, "description").trim().to_string();

    // 去掉 script/style，取可读文本前 500 字
    let mut text = String::new();
    let mut skip_depth = 0usize;
    let mut rest = html;
    while let Some(idx) = rest.find('<') {
        if skip_depth == 0 {
            text.push_str(&rest[..idx]);
        }
        rest = &rest[idx..];
        let end = rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
        let tag = &rest[..end];
        let lower_tag = tag.to_lowercase();
        if lower_tag.contains("</script") || lower_tag.contains("</style") {
            skip_depth = skip_depth.saturating_sub(1);
        } else if lower_tag.contains("<script") || lower_tag.contains("<style") {
            skip_depth += 1;
        } else if skip_depth == 0 {
            text.push(' ');
        }
        rest = &rest[end..];
    }
    if skip_depth == 0 {
        text.push_str(rest);
    }

    let preview = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect::<String>();
    // Markdown 源文件没有 <title>：用首个 H1 标题
    if title.is_empty() {
        if let Some(h1) = html
            .lines()
            .find(|line| line.trim_start().starts_with("# "))
        {
            title = h1.trim_start_matches('#').trim().to_string();
        }
    }
    (title, description, preview)
}

fn extract_tag(html: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let Some(start) = html.find(&open) else {
        return String::new();
    };
    let Some(gt) = html[start..].find('>') else {
        return String::new();
    };
    let content_start = start + gt + 1;
    let Some(close_idx) = html[content_start..].find(&close) else {
        return String::new();
    };
    html[content_start..content_start + close_idx].to_string()
}

fn extract_meta(html: &str, name: &str) -> String {
    let lower = html.to_lowercase();
    let pattern = format!("name=\"{name}\"");
    let Some(start) = lower.find(&pattern) else {
        return String::new();
    };
    let Some(content_attr) = lower[start..].find("content=") else {
        return String::new();
    };
    let content_start = start + content_attr + "content=".len();
    let rest = &html[content_start..];
    let trimmed = rest.trim_start();
    let value = if let Some(rest_after_quote) = trimmed.strip_prefix('"') {
        rest_after_quote
            .split('"')
            .next()
            .unwrap_or("")
            .to_string()
    } else if let Some(rest_after_quote) = trimmed.strip_prefix('\'') {
        rest_after_quote
            .split('\'')
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        trimmed
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    };
    value.trim().to_string()
}

/// 抓取远程文本（UA/超时/大小限制/状态码检查）
pub async fn fetch_raw(url: &str) -> Result<(String, String), String> {
    let validated_url = validate_doc_url(url)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;
    let mut resp = client
        .get(&validated_url)
        .header(
            "User-Agent",
            "PenSoul/0.1 (文档归档; 小说创作工具)",
        )
        .send()
        .await
        .map_err(|e| format!("请求文档失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("文档站点返回 HTTP {status}"));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    // 流式读取并限制 5MB：避免超大响应先全量载入内存再检查
    const MAX_DOC_BYTES: usize = 5 * 1024 * 1024;
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let chunk = resp
            .chunk()
            .await
            .map_err(|e| format!("读取文档响应失败: {e}"))?;
        match chunk {
            Some(c) => {
                if bytes.len() + c.len() > MAX_DOC_BYTES {
                    return Err("文档超过 5MB，已放弃保存".to_string());
                }
                bytes.extend_from_slice(&c);
            }
            None => break,
        }
    }
    let html = String::from_utf8_lossy(&bytes).to_string();
    Ok((html, content_type.to_string()))
}

/// 抓取官方文档并保存到本地（HTML/Markdown + 元信息 JSON）
pub async fn fetch_doc(
    url: &str,
    save_dir: &Path,
    model_id: &str,
) -> Result<RemoteDoc, String> {
    let safe_name = sanitize_filename(model_id);
    std::fs::create_dir_all(save_dir)
        .map_err(|e| format!("创建文档目录失败: {e}"))?;

    let (html, content_type) = fetch_raw(url).await?;
    let (title, description, preview) = extract_doc_info(&html);

    // Markdown 源文件按 .md 保存，保留表格结构便于后续解析
    let is_markdown = url.ends_with(".md") || content_type.contains("markdown");
    let ext = if is_markdown { "md" } else { "html" };
    let doc_file = format!("{safe_name}.{ext}");
    let html_path = save_dir.join(&doc_file);
    std::fs::write(&html_path, &html).map_err(|e| format!("保存文档失败: {e}"))?;

    let fetched_at = chrono::Utc::now().to_rfc3339();
    let normalized_url = validate_doc_url(url)?;
    let meta = serde_json::json!({
        "model_id": model_id,
        "url": normalized_url,
        "title": title,
        "description": description,
        "fetched_at": fetched_at,
    });
    std::fs::write(save_dir.join(format!("{safe_name}.meta.json")), meta.to_string())
        .map_err(|e| format!("保存文档元信息失败: {e}"))?;

    Ok(RemoteDoc {
        model_id: model_id.to_string(),
        url: normalized_url,
        title,
        description,
        text_preview: preview,
        saved_file: html_path.to_string_lossy().to_string(),
        fetched_at,
    })
}

/// 尝试从多个候选路径抓取文档索引（llms.txt），返回第一个成功的内容
pub async fn fetch_llms_index(doc_root: &str) -> Result<String, String> {
    let root = doc_root.trim_end_matches('/');
    let candidates = if root.ends_with("/docs") {
        vec![format!("{root}/llms.txt")]
    } else {
        vec![
            format!("{root}/docs/llms.txt"),
            format!("{root}/llms.txt"),
            format!("{root}/zh-cn/llms.txt"),
        ]
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let mut last_error = String::new();
    for url in candidates {
        let resp = client
            .get(&url)
            .header("User-Agent", "PenSoul/0.1 (文档索引)")
            .send()
            .await;
        match resp {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    return Ok(text);
                }
            }
            Ok(resp) => last_error = format!("{url} 返回 HTTP {}", resp.status()),
            Err(e) => last_error = format!("{url} 请求失败: {e}"),
        }
    }
    Err(last_error)
}

/// 解析供应商模型列表响应（OpenAI 兼容 + Anthropic）
pub fn parse_models_response(json: &str) -> Result<Vec<RemoteModel>, String> {
    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        data: Vec<RemoteModel>,
    }
    let response: Response =
        serde_json::from_str(json).map_err(|e| format!("解析模型列表失败: {e}"))?;
    if response.data.is_empty() {
        return Err("供应商返回的模型列表为空".to_string());
    }
    let mut models = response.data;
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    Ok(models)
}

/// 拉取供应商模型列表（OpenAI 兼容用 Bearer；Anthropic 用 x-api-key）
pub async fn list_remote_models(
    base_url: &str,
    api_key: &str,
    provider: &str,
) -> Result<Vec<RemoteModel>, String> {
    if api_key.trim().is_empty() {
        return Err("该配置尚未填写 API Key".to_string());
    }
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;

    let mut request = client.get(&url).bearer_auth(api_key);
    if provider == "anthropic" {
        request = request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
    }
    let resp = request
        .send()
        .await
        .map_err(|e| format!("请求模型列表失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let message = if text.len() > 300 {
            format!("{}（截断）", &text[..300])
        } else {
            text
        };
        return Err(format!("模型列表接口返回 HTTP {status}: {message}"));
    }
    parse_models_response(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_validation_blocks_non_http() {
        assert!(validate_doc_url("https://api.deepseek.com").is_ok());
        assert!(validate_doc_url("file:///etc/passwd").is_err());
        assert!(validate_doc_url("javascript:alert(1)").is_err());
        assert!(validate_doc_url("不是网址").is_err());
    }

    #[test]
    fn url_validation_blocks_loopback_and_private() {
        // 回环地址（P2-7：防 SSRF 访问本机服务）
        assert!(validate_doc_url("http://127.0.0.1:3001/api/projects").is_err());
        assert!(validate_doc_url("http://localhost:1420").is_err());
        assert!(validate_doc_url("http://[::1]:3001").is_err());
        // 私网/保留地址
        assert!(validate_doc_url("http://10.0.0.1").is_err());
        assert!(validate_doc_url("http://192.168.1.1").is_err());
        assert!(validate_doc_url("http://172.16.0.1").is_err());
        assert!(validate_doc_url("http://169.254.169.254/latest/meta-data").is_err());
        // 正常公网域名仍放行
        assert!(validate_doc_url("https://api.github.com").is_ok());
    }

    #[test]
    fn filename_sanitization() {
        assert_eq!(sanitize_filename("deepseek-chat"), "deepseek-chat");
        assert_eq!(sanitize_filename("gpt-4.1"), "gpt-4.1");
        assert_eq!(sanitize_filename("claude/sonnet 4"), "claude_sonnet_4");
    }

    #[test]
    fn html_extraction_finds_title_and_preview() {
        let html = r#"
            <html><head>
                <title>DeepSeek API 文档</title>
                <meta name="description" content="DeepSeek 模型说明与参数">
            </head><body>
                <script>var x = 1;</script>
                <p>这是正文内容，用于测试文本提取。</p>
            </body></html>
        "#;
        let (title, description, preview) = extract_doc_info(html);
        assert_eq!(title, "DeepSeek API 文档");
        assert_eq!(description, "DeepSeek 模型说明与参数");
        assert!(preview.contains("这是正文内容"));
        assert!(!preview.contains("var x"));
    }

    #[test]
    fn models_response_parsing() {
        let json = r#"{"data":[{"id":"deepseek-chat"},{"id":"deepseek-reasoner","display_name":"DeepSeek Reasoner"}]}"#;
        let models = parse_models_response(json).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "deepseek-chat");
        assert_eq!(models[1].display_name.as_deref(), Some("DeepSeek Reasoner"));
    }

    #[test]
    fn models_response_dedup() {
        let json = r#"{"data":[{"id":"a"},{"id":"a"}]}"#;
        let models = parse_models_response(json).unwrap();
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn suggest_doc_url_appends_model_path() {
        let url = suggest_model_doc_url(
            Some("https://api-docs.deepseek.com/zh-cn/"),
            "deepseek-reasoner",
        );
        assert_eq!(
            url.as_deref(),
            Some("https://api-docs.deepseek.com/zh-cn/models/deepseek-reasoner")
        );
        assert!(suggest_model_doc_url(None, "x").is_none());
    }

    #[test]
    fn extract_params_from_doc_text() {
        let html = r#"
            <html><head><title>deepseek-reasoner 文档</title></head><body>
            <p>上下文窗口 128K，max output 8192 tokens，思考模式 always。</p>
            </body></html>
        "#;
        let params = extract_model_params(html, "deepseek-reasoner");
        assert_eq!(params.context_window, Some(128_000));
        assert_eq!(params.max_output_tokens, Some(8192));
        assert_eq!(params.thinking_supported, Some(true));
    }

    #[test]
    fn parse_markdown_table_finds_model_description() {
        let markdown = r#"
| 模型名称 | 描述 |
| --- | --- |
| `kimi-k3` | 拥有 100 万 token 上下文窗口 |
| `kimi-k2.7-code` | 上下文 256k |
"#;
        assert_eq!(
            find_model_description(markdown, "kimi-k3").as_deref(),
            Some("拥有 100 万 token 上下文窗口")
        );
        assert!(find_model_description(markdown, "nonexistent").is_none());
    }

    #[test]
    fn extract_context_supports_chinese_units() {
        assert_eq!(extract_context_from_description("拥有 100 万 token 上下文窗口"), Some(1_000_000));
        assert_eq!(extract_context_from_description("上下文 256k"), Some(256_000));
        assert_eq!(extract_context_from_description("上下文长度 128K"), Some(128_000));
    }

    #[test]
    fn markdown_links_parse_and_rank() {
        let content = "\
# Index
- [Kimi K3](https://platform.kimi.com/docs/guide/kimi-k3-quickstart.md)
- [模型列表](https://platform.kimi.com/docs/models.md)
- [旗舰模型 Kimi K3 定价](https://platform.kimi.com/docs/pricing/chat-k3.md)
";
        let links = parse_markdown_links(content);
        assert_eq!(links.len(), 3);
        let pages = pick_relevant_pages(&links, "kimi-k3");
        assert_eq!(pages.len(), 3);
        assert!(pages[0].1.contains("kimi-k3-quickstart"), "模型专属页应排最前");
        assert!(pages[1].1.contains("models.md"), "模型列表页其次");
    }
}
