// pensoul-app: HTTP API 服务层

pub mod state;
pub mod commands;
pub mod api;
pub mod error;

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub use state::AppState;

/// 构建 CORS 层：只允许本机开发前端源
pub fn build_cors() -> CorsLayer {
    let allowed_origins = [
        "http://localhost:1420".parse::<HeaderValue>().unwrap(),
        "http://127.0.0.1:1420".parse::<HeaderValue>().unwrap(),
    ];
    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::PUT, Method::DELETE, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}
