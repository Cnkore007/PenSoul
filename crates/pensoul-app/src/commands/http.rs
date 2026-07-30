/// HTTP 代理命令 — 通过 Rust 端发请求绕过 WebView CSP 限制
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub body: String,
    pub ok: bool,
}

/// 通用 HTTP 请求代理
///
/// # 安全策略
/// 该命令是 WebView 的网络出口，必须防止被滥用为开放代理：
/// - 仅允许 `https`；`http` 只允许本机回环地址（本地模型调试）。
/// - 禁止云元数据地址（169.254.169.254 等 link-local）。
/// - 禁止指向私网网段的 http 请求。
#[tauri::command]
pub async fn http_request(request: HttpRequest) -> Result<HttpResponse, String> {
    validate_request_url(&request.url)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("PenSoul/0.1")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let method = match request.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        other => return Err(format!("不支持的 HTTP 方法: {}", other)),
    };

    let mut req_builder = client.request(method, &request.url);

    // 设置请求头
    if let Some(headers) = &request.headers {
        for (key, value) in headers {
            req_builder = req_builder.header(key.as_str(), value.as_str());
        }
    }

    // 添加请求体，若无 Content-Type 则默认 JSON
    if let Some(body) = &request.body {
        let has_ct = request
            .headers
            .as_ref()
            .is_some_and(|h| h.keys().any(|k| k.eq_ignore_ascii_case("content-type")));
        if !has_ct {
            req_builder = req_builder.header("content-type", "application/json");
        }
        req_builder = req_builder.body(body.clone());
    }

    // 发送请求
    let response = req_builder
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败 [{} {}]: {}", request.method, request.url, e))?;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();
    let ok = response.status().is_success();

    let body = response
        .text()
        .await
        .map_err(|e| format!("读取响应体失败: {}", e))?;

    Ok(HttpResponse {
        status,
        status_text,
        body,
        ok,
    })
}

/// 校验代理请求的目标 URL。
fn validate_request_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("非法的 URL: {e}"))?;
    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("").to_lowercase();

    let is_loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "[::1]")
        || host.starts_with("127.");

    // link-local / 云元数据地址一律禁止
    if host.starts_with("169.254.") {
        return Err("禁止访问 link-local 地址（云元数据风险）".to_string());
    }

    match scheme {
        "https" => Ok(()),
        "http" => {
            if is_loopback {
                Ok(())
            } else {
                Err("http 明文请求仅允许指向本机回环地址".to_string())
            }
        }
        other => Err(format!("不允许的协议: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_request_url;

    #[test]
    fn test_https_public_allowed() {
        assert!(validate_request_url("https://api.openai.com/v1/chat").is_ok());
    }

    #[test]
    fn test_http_loopback_allowed() {
        assert!(validate_request_url("http://localhost:11434/v1/chat").is_ok());
        assert!(validate_request_url("http://127.0.0.1:8080/test").is_ok());
    }

    #[test]
    fn test_http_remote_rejected() {
        assert!(validate_request_url("http://evil.example.com/").is_err());
    }

    #[test]
    fn test_metadata_ip_rejected() {
        assert!(validate_request_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_request_url("https://169.254.169.254/").is_err());
    }

    #[test]
    fn test_other_scheme_rejected() {
        assert!(validate_request_url("file:///etc/passwd").is_err());
        assert!(validate_request_url("ftp://example.com/").is_err());
    }
}
