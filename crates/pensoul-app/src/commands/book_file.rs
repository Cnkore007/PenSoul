//! 书籍文件解析 —— 书籍蒸馏的全文输入通道。
//!
//! 支持格式：txt / md 直读；epub 解 zip 按 OPF spine 顺序拼接正文；
//! pdf 用 pdf-extract 提取文本（扫描版无文本层的会报错引导）。
//!
//! 全书动辄几十万字，无法整体塞进蒸馏 prompt——统一走
//! 「开头 + 中段均匀抽样 + 结尾」抽样（默认 2 万字预算），
//! 对写法类蒸馏（文风/结构/人物/张力）抽样代表性足够，
//! 全景与跨章证据由模型的作品知识储备补充。
use std::io::Read;
use std::path::Path;

/// 抽样总预算（字符）
const SAMPLE_BUDGET: usize = 20_000;
/// 抽样段数：开头 1 段 + 中段 3 段 + 结尾 1 段
const HEAD_CHARS: usize = 6_000;
const MID_CHARS: usize = 3_000;
const MID_COUNT: usize = 3;
const TAIL_CHARS: usize = 3_000;

/// 解析后的书籍文本
pub struct BookText {
    /// 全文纯文本（未抽样）
    pub full_text: String,
    /// 按扩展名猜测的书名（文件名去扩展名）
    pub title_guess: String,
}

/// 读取并解析书籍文件为纯文本（按扩展名分派）
pub fn read_book_file(path: &str) -> Result<BookText, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("文件不存在: {path}"));
    }
    // 文件名安全：只读，不写入；路径由系统文件对话框给出
    let title_guess = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知书籍")
        .to_string();
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let full_text = match ext.as_str() {
        "txt" | "md" | "markdown" | "text" => read_plain(p)?,
        "epub" => read_epub(p)?,
        "pdf" => read_pdf(p)?,
        other => {
            // 未知格式按纯文本尝试（用户说「任意格式」，多数中文书源是 txt 换皮）
            match read_plain(p) {
                Ok(t) if !t.trim().is_empty() => t,
                _ => {
                    return Err(format!(
                        "暂不支持的格式 .{other}，且按纯文本读取失败。请转为 txt / md / epub / pdf 后重试"
                    ));
                }
            }
        }
    };

    let cleaned = normalize_text(&full_text);
    if cleaned.chars().count() < 500 {
        return Err(format!(
            "从「{title_guess}」中提取到的文本过少（{} 字符），可能是扫描版/图片型书籍或加密文件",
            cleaned.chars().count()
        ));
    }
    Ok(BookText {
        full_text: cleaned,
        title_guess,
    })
}

/// 全文抽样：开头 + 中段均匀 + 结尾，总长不超过 SAMPLE_BUDGET。
/// 文本本身不足预算时原样返回。
pub fn sample_text(text: &str) -> String {
    let total = text.chars().count();
    if total <= SAMPLE_BUDGET {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    out.push_str(&chars[..HEAD_CHARS].iter().collect::<String>());
    // 中段均匀采样：在 (HEAD, total-TAIL-MID) 区间等距取 MID_COUNT 段
    let usable_end = total.saturating_sub(TAIL_CHARS + MID_CHARS);
    for i in 0..MID_COUNT {
        let frac = (i + 1) as f64 / (MID_COUNT + 1) as f64;
        let start = HEAD_CHARS + ((usable_end - HEAD_CHARS) as f64 * frac) as usize;
        let start = start.min(total.saturating_sub(MID_CHARS));
        out.push_str(&format!(
            "\n\n〔……中段抽样 {}／{}……〕\n\n",
            i + 1,
            MID_COUNT
        ));
        out.push_str(&chars[start..start + MID_CHARS].iter().collect::<String>());
    }
    out.push_str("\n\n〔……结尾抽样……〕\n\n");
    out.push_str(&chars[total - TAIL_CHARS..].iter().collect::<String>());
    out
}

// ── 各格式解析 ──

/// txt / md：直接读文本（兼容 UTF-8；失败尝试 Latin-1 字节流失真转换兜底）
fn read_plain(p: &Path) -> Result<String, String> {
    let bytes = std::fs::read(p).map_err(|e| format!("读取文件失败: {e}"))?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => {
            // GBK 等中文编码常见：逐字节有损转换至少能读出结构（乱码仍会污染蒸馏，明确提示）
            let lossy: String = e
                .into_bytes()
                .iter()
                .map(|&b| if b.is_ascii() { b as char } else { '□' })
                .collect();
            if lossy.chars().filter(|c| *c != '□').count() > 2000 {
                Ok(lossy)
            } else {
                Err("文件不是 UTF-8 编码（可能是 GBK），请另存为 UTF-8 后重试".to_string())
            }
        }
    }
}

/// epub：解 zip，优先按 OPF spine 顺序拼接正文 xhtml；
/// OPF 解析失败时降级为按文件名排序拼接所有 xhtml/html。
fn read_epub(p: &Path) -> Result<String, String> {
    let file = std::fs::File::open(p).map_err(|e| format!("打开 epub 失败: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 epub（zip）失败: {e}"))?;

    // 收集所有正文候选文件名
    let mut html_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let Ok(f) = archive.by_index(i) else {
            continue;
        };
        let name = f.name().to_string();
        let lower = name.to_lowercase();
        if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
            html_names.push(name);
        }
    }
    if html_names.is_empty() {
        return Err("epub 中未找到任何 xhtml/html 正文文件".to_string());
    }

    // 尝试 OPF spine 顺序；失败降级文件名排序
    let ordered = opf_spine_order(&mut archive, &html_names).unwrap_or_else(|| {
        let mut sorted = html_names.clone();
        sorted.sort();
        sorted
    });

    let mut out = String::new();
    for name in ordered {
        let Ok(mut f) = archive.by_name(&name) else {
            continue;
        };
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_err() {
            continue; // 单文件编码异常跳过，不拖垮整本
        }
        let text = strip_html(&buf);
        if !text.trim().is_empty() {
            out.push_str(&text);
            out.push('\n');
        }
    }
    if out.trim().is_empty() {
        return Err("epub 正文提取为空".to_string());
    }
    Ok(out)
}

/// 从 epub 的 OPF 文件解析 spine 阅读顺序，映射为文件路径列表。
/// 简易容错实现：正则提取 manifest 的 id→href 与 spine 的 idref 顺序，
/// 不做完整 XML 解析（命名空间差异大，正则对 99% 的 epub 够用）。
fn opf_spine_order(
    archive: &mut zip::ZipArchive<std::fs::File>,
    html_names: &[String],
) -> Option<Vec<String>> {
    // 找 .opf 文件
    let mut opf_name = None;
    let mut opf_dir = String::new();
    for i in 0..archive.len() {
        let f = archive.by_index(i).ok()?;
        let name = f.name().to_string();
        if name.to_lowercase().ends_with(".opf") {
            opf_dir = name
                .rsplit_once('/')
                .map(|(d, _)| format!("{d}/"))
                .unwrap_or_default();
            opf_name = Some(name);
            break;
        }
    }
    let opf_name = opf_name?;
    let mut opf = String::new();
    archive
        .by_name(&opf_name)
        .ok()?
        .read_to_string(&mut opf)
        .ok()?;

    // manifest: <item id="..." href="...">
    let mut id_to_href: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for caps in opf.split("<item ").skip(1) {
        let id = extract_attr(caps, "id")?;
        let href = extract_attr(caps, "href");
        if let (Some(id), Some(href)) = (Some(id), href) {
            id_to_href.insert(id, href);
        }
    }
    // spine: <itemref idref="...">
    let mut ordered = Vec::new();
    for caps in opf.split("<itemref ").skip(1) {
        if let Some(idref) = extract_attr(caps, "idref")
            && let Some(href) = id_to_href.get(&idref)
        {
            let full = format!("{opf_dir}{href}");
            // href 可能带 ../ 或锚点，规范化匹配 html_names
            let full = full.split('#').next().unwrap_or("").to_string();
            if let Some(hit) = html_names
                .iter()
                .find(|n| **n == full || n.ends_with(href.as_str()))
                && !ordered.contains(hit)
            {
                ordered.push(hit.clone());
            }
        }
    }
    if ordered.is_empty() {
        None
    } else {
        Some(ordered)
    }
}

/// 从 XML 标签片段中提取属性值（容错：单双引号均可）
fn extract_attr(tag_fragment: &str, attr: &str) -> Option<String> {
    let pat_dq = format!("{attr}=\"");
    let pat_sq = format!("{attr}='");
    if let Some(idx) = tag_fragment.find(&pat_dq) {
        let rest = &tag_fragment[idx + pat_dq.len()..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else if let Some(idx) = tag_fragment.find(&pat_sq) {
        let rest = &tag_fragment[idx + pat_sq.len()..];
        let end = rest.find('\'')?;
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// pdf：pdf-extract 提取文本层（扫描版无文本层会在调用方因文本过短报错）
fn read_pdf(p: &Path) -> Result<String, String> {
    pdf_extract::extract_text(p)
        .map_err(|e| format!("解析 pdf 失败: {e}（扫描版 PDF 无文本层，请用文字版或转 txt）"))
}

/// 剥 HTML 标签：去 script/style 块、标签、实体，保留段落换行
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut chars = html.chars().peekable();
    let mut in_tag = false;
    let mut skip_depth = 0; // script/style 内容丢弃
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                in_tag = true;
                // 检测 script/style 开闭
                let rest: String = chars.clone().take(7).collect();
                let rest_lower = rest.to_lowercase();
                if rest_lower.starts_with("script") || rest_lower.starts_with("style") {
                    skip_depth += 1;
                } else if rest_lower.starts_with("/script") || rest_lower.starts_with("/style") {
                    skip_depth = (skip_depth - 1).max(0);
                }
            }
            '>' => {
                in_tag = false;
            }
            _ if in_tag => {}
            _ if skip_depth > 0 => {}
            _ => out.push(c),
        }
    }
    // 常见实体还原（够用集合）
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// 文本规范化：统一换行、压缩 3+ 连续空行为 1 个空行
fn normalize_text(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_short_text_passthrough() {
        let text = "短文本".repeat(100);
        assert_eq!(sample_text(&text), text);
    }

    #[test]
    fn test_sample_long_text_within_budget() {
        let text = "字".repeat(50_000);
        let sampled = sample_text(&text);
        // 6000 头 + 3×3000 中 + 3000 尾 + 抽样标记 ≈ 1.9 万字符，不超预算太多
        assert!(sampled.chars().count() <= SAMPLE_BUDGET + 200);
        assert!(sampled.contains("中段抽样"));
        assert!(sampled.contains("结尾抽样"));
    }

    #[test]
    fn test_strip_html_basic() {
        let html =
            "<html><body><p>第一段</p><script>var x=1;</script><p>第二&nbsp;段</p></body></html>";
        let text = strip_html(html);
        assert!(text.contains("第一段"));
        assert!(text.contains("第二 段"));
        assert!(!text.contains("var x"));
    }

    #[test]
    fn test_normalize_text_collapses_blank_lines() {
        let text = "a\n\n\n\n\nb";
        assert_eq!(normalize_text(text), "a\n\nb\n");
    }
}
