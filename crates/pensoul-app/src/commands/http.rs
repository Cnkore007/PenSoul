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
#[tauri::command]
pub async fn http_request(request: HttpRequest) -> Result<HttpResponse, String> {
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
