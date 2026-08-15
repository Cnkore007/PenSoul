// api.rs — HTTP API 集成测试

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;

use pensoul_app::state::AppState;

/// 构建带临时数据目录的路由
fn test_app() -> (Router, TempDir) {
    let dir = TempDir::new().expect("创建临时目录失败");
    let state = Arc::new(RwLock::new(AppState::new(
        dir.path().to_string_lossy().to_string(),
    )));
    let app = pensoul_app::api::create_router(state).layer(pensoul_app::build_cors());
    (app, dir)
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

fn put_form(uri: &str, form: &str) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(form.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn project_id_validation_blocks_path_traversal() {
    let (app, _dir) = test_app();
    let resp = app
        .oneshot(post_form("/api/projects/create", "project_id=../escape&title=x"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_project_create_conflicts() {
    let (app, _dir) = test_app();
    let ok = app
        .clone()
        .oneshot(post_form("/api/projects/create", "project_id=proj1&title=项目一"))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    let dup = app
        .oneshot(post_form("/api/projects/create", "project_id=proj1&title=重复"))
        .await
        .unwrap();
    assert_eq!(dup.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn delete_missing_project_returns_404() {
    let (app, _dir) = test_app();
    let resp = app
        .oneshot(delete("/api/projects/delete?project_id=missing"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_projects_returns_chinese_title() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=yishan&title=移山"))
        .await
        .unwrap();

    let resp = app.oneshot(get("/api/projects")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_text(resp).await;
    assert!(body.contains("yishan"), "列表应包含项目 ID: {body}");
    assert!(body.contains("移山"), "列表应包含中文标题: {body}");
}

#[tokio::test]
async fn sprout_session_apply_requires_pending_proposal() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=sprout&title=萌芽"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=sprout"))
        .await
        .unwrap();

    // 空会话可读取
    let session_resp = app
        .clone()
        .oneshot(get("/api/sprout/session"))
        .await
        .unwrap();
    assert_eq!(session_resp.status(), StatusCode::OK);
    let session = body_text(session_resp).await;
    assert!(session.contains("\"messages\":[]"), "会话应为空: {session}");

    // 没有待确认提案时不能应用
    let apply_resp = app
        .clone()
        .oneshot(post_form("/api/sprout/apply", ""))
        .await
        .unwrap();
    assert_eq!(apply_resp.status(), StatusCode::BAD_REQUEST);

    // 清空会话正常
    let clear_resp = app
        .oneshot(post_form("/api/sprout/clear", ""))
        .await
        .unwrap();
    assert_eq!(clear_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn writing_generate_validation() {
    let (app, _dir) = test_app();

    // 未打开项目
    let no_project = app
        .clone()
        .oneshot(post_form(
            "/api/writing/generate",
            "chapter_id=x&mode=draft",
        ))
        .await
        .unwrap();
    assert_eq!(no_project.status(), StatusCode::BAD_REQUEST);

    // 打开项目后：章节不存在 → 404
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=w&title=写"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=w"))
        .await
        .unwrap();
    let missing = app
        .clone()
        .oneshot(post_form(
            "/api/writing/generate",
            "chapter_id=nope&mode=draft",
        ))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    // mode 非法 → 400（不触发 LLM 调用）
    let bad_mode = app
        .oneshot(post_form(
            "/api/writing/generate",
            "chapter_id=nope&mode=rewrite",
        ))
        .await
        .unwrap();
    assert_eq!(bad_mode.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn writing_generate_requires_llm_config() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=wl&title=写"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=wl"))
        .await
        .unwrap();

    // 创建章节（章节号回填为 1）
    let create_resp = app
        .clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    let chapter_id = body_text(create_resp).await;

    // 章节存在但未配置默认 LLM → 400 且提示配置（验证静态上下文/记忆检索/约束快照路径可达）
    let resp = app
        .oneshot(post_form(
            "/api/writing/generate",
            &format!("chapter_id={chapter_id}&mode=draft"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_text(resp).await;
    assert!(
        body.contains("LLM 配置") || body.contains("API Key"),
        "应提示配置 LLM: {body}"
    );
}

#[tokio::test]
async fn chapter_save_and_load_roundtrip() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=novel&title=新书"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=novel"))
        .await
        .unwrap();

    // 创建章节
    let create_resp = app
        .clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let chapter_id = body_text(create_resp).await;
    assert!(!chapter_id.is_empty());

    // 保存正文（PUT + form，前端实际行为）
    let save_resp = app
        .clone()
        .oneshot(put_form(
            "/api/outline/chapters/content",
            &format!("chapter_id={chapter_id}&content=夜雨敲窗，他提笔写下第一行。"),
        ))
        .await
        .unwrap();
    assert_eq!(
        save_resp.status(),
        StatusCode::OK,
        "保存正文应成功: {}",
        body_text(save_resp).await
    );

    // 读取正文
    let get_resp = app
        .clone()
        .oneshot(get(&format!(
            "/api/outline/chapters/content?chapter_id={chapter_id}"
        )))
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let detail = body_text(get_resp).await;
    let json: serde_json::Value = serde_json::from_str(&detail).unwrap();
    assert!(json["content"].as_str().unwrap().contains("夜雨敲窗"));
    assert_eq!(json["version"], 2);
    assert_eq!(json["revision_count"], 1);
    assert_eq!(json["consistency_score"], 1.0);
}

#[tokio::test]
async fn chapter_status_gate_rejects_illegal_jump() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=flow&title=流程"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=flow"))
        .await
        .unwrap();
    let create_resp = app
        .clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    let chapter_id = body_text(create_resp).await;

    // 草稿直接跳到已发布：拒绝
    let jump = app
        .clone()
        .oneshot(put_form(
            "/api/outline/chapters/update",
            &format!("chapter_id={chapter_id}&status=Published"),
        ))
        .await
        .unwrap();
    assert_eq!(jump.status(), StatusCode::BAD_REQUEST);

    // 按流程推进：草稿 → 审阅中
    let ok = app
        .oneshot(put_form(
            "/api/outline/chapters/update",
            &format!("chapter_id={chapter_id}&status=Reviewing"),
        ))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn foreshadow_status_gate_rejects_illegal_jump() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=fs&title=伏笔"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=fs"))
        .await
        .unwrap();
    // 伏笔埋设章号必须指向真实存在的章节（P1-7 后端校验），先建第 1 章
    app.clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    let fs_resp = app
        .clone()
        .oneshot(post_form(
            "/api/world/foreshadows/add",
            "name=玉佩&planted_chapter=1",
        ))
        .await
        .unwrap();
    let fs_id = body_text(fs_resp).await;

    // Planned 直接变 Resolved：拒绝
    let jump = app
        .clone()
        .oneshot(put_form(
            "/api/world/foreshadows/update",
            &format!("id={fs_id}&status=Resolved"),
        ))
        .await
        .unwrap();
    assert_eq!(jump.status(), StatusCode::BAD_REQUEST);

    // Planned → Planted 允许
    let ok = app
        .oneshot(put_form(
            "/api/world/foreshadows/update",
            &format!("id={fs_id}&status=Planted"),
        ))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}

#[tokio::test]
async fn entity_add_is_persisted() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=persist&title=落盘"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=persist"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/entities/character", "name=林默"))
        .await
        .unwrap();

    // 从磁盘重新加载后角色仍在
    let state = Arc::new(RwLock::new(AppState::new(
        _dir.path().to_string_lossy().to_string(),
    )));
    state
        .write()
        .await
        .load_project("persist")
        .expect("重新加载项目失败");
    let binding = state.read().await;
    let characters = &binding.ontology.as_ref().unwrap().characters.characters;
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].name, "林默");
}

#[tokio::test]
async fn cors_rejects_unknown_origin() {
    let (app, _dir) = test_app();
    let evil_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/projects")
                .header("Origin", "https://evil.example")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // 恶意来源不被放行：不携带 ACAO 头，浏览器会阻止跨域读取
    assert!(evil_resp
        .headers()
        .get("access-control-allow-origin")
        .is_none());

    // 本机开发源正常放行
    let local_resp = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/projects")
                .header("Origin", "http://localhost:1420")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        local_resp
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:1420")
    );
}

#[tokio::test]
async fn update_missing_entity_returns_404() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=e404&title=缺失"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=e404"))
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(put_form(
            "/api/outline/chapters/update",
            "chapter_id=nonexistent&title=x",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

fn post_form(uri: &str, form: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(form.to_string()))
        .unwrap()
}

#[tokio::test]
async fn llm_config_full_lifecycle() {
    let (app, _dir) = test_app();

    // 新增配置
    let create_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=DeepSeek官方&provider=deepseek&model_id=deepseek-chat&api_key=sk-abcdefgh12345678&context_window=64000&max_output_tokens=4096&temperature=0.7",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let config_id = body_text(create_resp).await;
    assert!(!config_id.is_empty());

    // 拉取列表：密钥必须脱敏
    let list_resp = app.clone().oneshot(get("/api/llm/configs")).await.unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list: serde_json::Value = serde_json::from_str(&body_text(list_resp).await).unwrap();
    assert_eq!(list["providers"].as_array().unwrap().len(), 1);
    let provider = &list["providers"][0];
    assert_eq!(provider["has_key"], true);
    assert!(provider.get("api_key").is_none(), "不得返回完整密钥");
    assert_eq!(provider["api_key_masked"], "sk-a***5678");

    // 设为默认
    let default_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/default",
            &format!("config_id={config_id}"),
        ))
        .await
        .unwrap();
    assert_eq!(default_resp.status(), StatusCode::OK);

    // 更新：api_key 留空不覆盖
    let update_resp = app
        .clone()
        .oneshot(put_form(
            "/api/llm/configs",
            &format!("id={config_id}&temperature=0.3&api_key="),
        ))
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let list2: serde_json::Value = serde_json::from_str(
        &body_text(app.clone().oneshot(get("/api/llm/configs")).await.unwrap()).await,
    )
    .unwrap();
    let temp = list2["providers"][0]["temperature"].as_f64().unwrap();
    assert!(
        (temp - 0.3).abs() < 1e-6,
        "temperature 应更新为 0.3，实际 {temp}"
    );
    assert_eq!(
        list2["providers"][0]["has_key"], true,
        "留空密钥不应覆盖原密钥"
    );

    // 上下文检测（按配置）
    let ctx_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/context-check",
            &format!("config_id={config_id}&text=第一章，夜雨敲窗。"),
        ))
        .await
        .unwrap();
    assert_eq!(ctx_resp.status(), StatusCode::OK);
    let ctx: serde_json::Value = serde_json::from_str(&body_text(ctx_resp).await).unwrap();
    assert_eq!(ctx["context_window"], 64000);
    assert_eq!(ctx["fits"], true);
    assert!(ctx["estimated_tokens"].as_u64().unwrap() >= 1);

    // 删除
    let del_resp = app
        .clone()
        .oneshot(delete(&format!("/api/llm/configs?config_id={config_id}")))
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // 删除后默认与列表清空；再删除返回 404
    let list3: serde_json::Value = serde_json::from_str(
        &body_text(app.clone().oneshot(get("/api/llm/configs")).await.unwrap()).await,
    )
    .unwrap();
    assert_eq!(list3["providers"].as_array().unwrap().len(), 0);
    assert!(list3["default_provider_id"].is_null());
    let del_again = app
        .oneshot(delete(&format!("/api/llm/configs?config_id={config_id}")))
        .await
        .unwrap();
    assert_eq!(del_again.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn llm_config_rejects_invalid_params() {
    let (app, _dir) = test_app();

    // 窗口小于输出上限
    let bad_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=错误&provider=openai&model_id=gpt-4o&context_window=1000&max_output_tokens=2000",
        ))
        .await
        .unwrap();
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);

    // 未知供应商
    let bad_provider = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=错误&provider=unknown&model_id=x&context_window=10000&max_output_tokens=1000",
        ))
        .await
        .unwrap();
    assert_eq!(bad_provider.status(), StatusCode::BAD_REQUEST);

    // temperature 越界
    let bad_temp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=错误&provider=openai&model_id=gpt-4o&context_window=10000&max_output_tokens=1000&temperature=5",
        ))
        .await
        .unwrap();
    assert_eq!(bad_temp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn llm_pull_models_requires_key() {
    let (app, _dir) = test_app();

    // 先创建一条没有密钥的配置
    let create_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=无密钥&provider=deepseek&model_id=deepseek-chat&context_window=64000&max_output_tokens=4096",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let config_id = body_text(create_resp).await;

    let resp = app
        .oneshot(post_form(
            "/api/llm/models/pull",
            &format!("config_id={config_id}"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let text = body_text(resp).await;
    assert!(text.contains("API Key"), "应提示缺少密钥: {text}");
}

#[tokio::test]
async fn llm_config_accepts_empty_optional_numbers() {
    let (app, _dir) = test_app();

    // 数值/布尔字段传空字符串不应导致反序列化失败
    let create_resp = app
        .clone()
        .oneshot(post_form(
            "/api/llm/configs",
            "name=宽松解析&provider=deepseek&model_id=deepseek-chat&context_window=64000&max_output_tokens=4096&thinking_budget=&timeout_seconds=&json_mode=&supports_streaming=&enabled=&temperature=&top_p=",
        ))
        .await
        .unwrap();
    assert_eq!(
        create_resp.status(),
        StatusCode::OK,
        "空字段应被容忍: {}",
        body_text(create_resp).await
    );
    let config_id = body_text(create_resp).await;

    // 空字段应回落默认值
    let list: serde_json::Value = serde_json::from_str(
        &body_text(app.clone().oneshot(get("/api/llm/configs")).await.unwrap()).await,
    )
    .unwrap();
    let provider = &list["providers"][0];
    assert_eq!(provider["timeout_seconds"], 120);
    assert_eq!(provider["supports_streaming"], true);
    assert_eq!(provider["enabled"], true);
    assert!(provider["thinking_budget"].is_null());

    // 更新时同样容忍空字段，且不覆盖已有值
    let update_resp = app
        .oneshot(put_form(
            "/api/llm/configs",
            &format!("id={config_id}&thinking_budget=&temperature=&timeout_seconds="),
        ))
        .await
        .unwrap();
    assert_eq!(
        update_resp.status(),
        StatusCode::OK,
        "更新空字段应被容忍: {}",
        body_text(update_resp).await
    );
}

#[tokio::test]
async fn foreshadow_payoff_null_transfers_as_clear() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=fsnull&title=清空回收"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=fsnull"))
        .await
        .unwrap();
    // 伏笔埋设章号必须指向真实存在的章节（P1-7 后端校验），先建第 1 章
    app.clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    let fs_id = body_text(
        app.clone()
            .oneshot(post_form(
                "/api/world/foreshadows/add",
                "name=玉佩&planted_chapter=1",
            ))
            .await
            .unwrap(),
    )
    .await;

    // 先设置实际回收章节
    let set_resp = app
        .clone()
        .oneshot(put_form(
            "/api/world/foreshadows/update",
            &format!("id={fs_id}&actual_payoff=5"),
        ))
        .await
        .unwrap();
    assert_eq!(set_resp.status(), StatusCode::OK);

    // 传空字符串 = 清空，而不是报 400
    let clear_resp = app
        .clone()
        .oneshot(put_form(
            "/api/world/foreshadows/update",
            &format!("id={fs_id}&actual_payoff="),
        ))
        .await
        .unwrap();
    assert_eq!(
        clear_resp.status(),
        StatusCode::OK,
        "空字段应被容忍: {}",
        body_text(clear_resp).await
    );

    let list: serde_json::Value = serde_json::from_str(
        &body_text(
            app.clone()
                .oneshot(get("/api/world/foreshadows"))
                .await
                .unwrap(),
        )
        .await,
    )
    .unwrap();
    assert!(list[0]["actual_payoff"].is_null(), "空字段应清空回收章节");
}

#[tokio::test]
async fn event_chapter_null_is_ignored() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=evnull&title=空章节"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=evnull"))
        .await
        .unwrap();
    // 事件章号必须指向真实存在的章节（P1-7 后端校验），先建第 3 章
    app.clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第一章"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第二章"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=第三章"))
        .await
        .unwrap();
    let ev_id = body_text(
        app.clone()
            .oneshot(post_form("/api/entities/event", "name=决战&chapter_id=3"))
            .await
            .unwrap(),
    )
    .await;

    // 更新时章节字段传空 = 保留原值，不应报错
    let resp = app
        .clone()
        .oneshot(put_form(
            "/api/entities/event/update",
            &format!("id={ev_id}&chapter_id=&description=更新描述"),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "空章节字段应被容忍: {}",
        body_text(resp).await
    );

    let list: serde_json::Value = serde_json::from_str(
        &body_text(
            app.clone()
                .oneshot(get("/api/world/timeline"))
                .await
                .unwrap(),
        )
        .await,
    )
    .unwrap();
    assert_eq!(list[0]["chapter_id"], 3, "空字段不应覆盖原章节号");
}

#[tokio::test]
async fn techniques_list_returns_builtin() {
    let (app, _dir) = test_app();
    let resp = app.oneshot(get("/api/writing/techniques")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_text(resp).await;
    assert!(body.contains("suspense"), "应包含叙事技巧: {body}");
    assert!(body.contains("hook_matrix"), "应包含网文节奏模板: {body}");
    assert!(!body.contains("本章写作技巧"), "列表不应含提示词段落: {body}");
}

#[tokio::test]
async fn style_notes_roundtrip_persisted() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=st&title=风"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=st"))
        .await
        .unwrap();

    let initial = app
        .clone()
        .oneshot(get("/api/world/style"))
        .await
        .unwrap();
    assert_eq!(initial.status(), StatusCode::OK);

    let upd = app
        .clone()
        .oneshot(put_form(
            "/api/world/style",
            "style_notes=冷峻克制，短句推进&pacing_notes=先抑后扬",
        ))
        .await
        .unwrap();
    assert_eq!(upd.status(), StatusCode::OK);

    // 重新打开项目，验证风格笔记已落盘正典
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=st"))
        .await
        .unwrap();
    let again = app.oneshot(get("/api/world/style")).await.unwrap();
    let body = body_text(again).await;
    assert!(body.contains("冷峻克制"), "风格笔记应持久化: {body}");
    assert!(body.contains("先抑后扬"), "节奏笔记应持久化: {body}");
}

#[tokio::test]
async fn review_local_mode_without_llm() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=rv&title=审"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=rv"))
        .await
        .unwrap();

    // 章节不存在 → 404
    let missing = app
        .clone()
        .oneshot(post_form("/api/writing/review", "chapter_id=nope"))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    // 创建章节并保存正文，再审校（无 LLM 配置 → 降级 local 模式，仍可用）
    let create_resp = app
        .clone()
        .oneshot(post_form("/api/outline/chapters/create", "title=一章"))
        .await
        .unwrap();
    let chapter_id = body_text(create_resp).await;
    let save = app
        .clone()
        .oneshot(put_form(
            "/api/outline/chapters/content",
            &format!("chapter_id={chapter_id}&content=本章他感到紧张，仿佛一切都要结束了。"),
        ))
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);

    let review_resp = app
        .oneshot(post_form("/api/writing/review", &format!("chapter_id={chapter_id}")))
        .await
        .unwrap();
    assert_eq!(review_resp.status(), StatusCode::OK);
    let body = body_text(review_resp).await;
    assert!(body.contains("\"mode\":\"local\""), "无 LLM 配置应降级为 local: {body}");
    assert!(body.contains("meta_narration_hits"), "应包含元叙述检测: {body}");
    assert!(body.contains("感到"), "应包含说教密度统计: {body}");
}

#[tokio::test]
async fn agent_configs_list_and_validate_binding() {
    let (app, _dir) = test_app();

    // 列表含全部角色（P0b）
    let list = app
        .clone()
        .oneshot(get("/api/agent/configs"))
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let body = body_text(list).await;
    for role in ["writer", "reviewer", "extractor", "outliner", "distiller"] {
        assert!(body.contains(role), "应包含角色 {role}: {body}");
    }

    // 绑定不存在的 LLM 配置 → 400
    let bad = app
        .clone()
        .oneshot(put_form("/api/agent/configs", "role_id=writer&llm_config_id=nope"))
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

    // 未知角色 → 400
    let unknown = app
        .clone()
        .oneshot(put_form("/api/agent/configs", "role_id=ghost&llm_config_id=x"))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);

    // 绑定空 → 回退默认（ok，落盘 agent-config.json）
    let reset = app
        .oneshot(put_form("/api/agent/configs", "role_id=writer&llm_config_id="))
        .await
        .unwrap();
    assert_eq!(reset.status(), StatusCode::OK);
}

#[tokio::test]
async fn organization_crud_and_character_archive_fields_persist() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p0&title=档"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p0"))
        .await
        .unwrap();

    // 组织档案：添加 → 列表 → 更新
    let add = app
        .clone()
        .oneshot(post_form("/api/entities/organization", "name=青云宗&category=宗门"))
        .await
        .unwrap();
    assert_eq!(add.status(), StatusCode::OK);
    let org_id = body_text(add).await;

    let list = app
        .clone()
        .oneshot(get("/api/entities/organizations"))
        .await
        .unwrap();
    let body = body_text(list).await;
    assert!(body.contains("青云宗"), "组织应出现在列表: {body}");

    let upd = app
        .clone()
        .oneshot(put_form(
            "/api/entities/organization/update",
            &format!("id={org_id}&goals=执掌正道&structure=宗主/长老/弟子"),
        ))
        .await
        .unwrap();
    assert_eq!(upd.status(), StatusCode::OK);

    // 人物档案新字段（外貌/衣着/功法/境界/法宝）
    let cid = body_text(
        app.clone()
            .oneshot(post_form("/api/entities/character", "name=林默"))
            .await
            .unwrap(),
    )
    .await;
    let cupd = app
        .clone()
        .oneshot(put_form(
            "/api/entities/character/update",
            &format!(
                "id={cid}&attire=青衫长袍&techniques=青云剑诀,御风术&realm=筑基&items=青锋剑"
            ),
        ))
        .await
        .unwrap();
    assert_eq!(cupd.status(), StatusCode::OK);

    // 重新打开项目，验证组织与人物档案字段已落盘
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p0"))
        .await
        .unwrap();
    let orgs = body_text(
        app.clone()
            .oneshot(get("/api/entities/organizations"))
            .await
            .unwrap(),
    )
    .await;
    assert!(orgs.contains("执掌正道"), "组织目标应持久化: {orgs}");
    let chars = body_text(app.oneshot(get("/api/world/characters")).await.unwrap()).await;
    assert!(chars.contains("青衫长袍"), "人物衣着应持久化: {chars}");
    assert!(chars.contains("青云剑诀"), "人物功法应持久化: {chars}");
    assert!(chars.contains("筑基"), "人物境界应持久化: {chars}");
}

#[tokio::test]
async fn annotation_crud_persists_with_chapter() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p2&title=改"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p2"))
        .await
        .unwrap();
    let chapter_id = body_text(
        app.clone()
            .oneshot(post_form("/api/outline/chapters/create", "title=一章"))
            .await
            .unwrap(),
    )
    .await;

    // 添加批注
    let add = app
        .clone()
        .oneshot(post_form(
            "/api/writing/annotations",
            &format!(
                "chapter_id={chapter_id}&kind=建议&content=这段太啰嗦，删一半"
            ),
        ))
        .await
        .unwrap();
    assert_eq!(add.status(), StatusCode::OK);
    let annotation_id = body_text(add).await;

    // 批注随章节内容返回
    let detail = body_text(
        app.clone()
            .oneshot(get(&format!(
                "/api/outline/chapters/content?chapter_id={chapter_id}"
            )))
            .await
            .unwrap(),
    )
    .await;
    assert!(detail.contains("太啰嗦"), "批注应出现在章节详情: {detail}");

    // 状态流转 → 已解决
    let solved = app
        .clone()
        .oneshot(put_form(
            "/api/writing/annotations/status",
            &format!(
                "chapter_id={chapter_id}&annotation_id={annotation_id}&status=已解决"
            ),
        ))
        .await
        .unwrap();
    assert_eq!(solved.status(), StatusCode::OK);

    // 删除
    let del = app
        .clone()
        .oneshot(delete(&format!(
            "/api/writing/annotations/delete?chapter_id={chapter_id}&annotation_id={annotation_id}"
        )))
        .await
        .unwrap();
    assert_eq!(del.status(), StatusCode::OK);
    let after = body_text(
        app.oneshot(get(&format!(
            "/api/outline/chapters/content?chapter_id={chapter_id}"
        )))
        .await
        .unwrap(),
    )
    .await;
    // content 接口返回结构不含批注数组本体（仅统计），此处验证批注已被删除不影响接口
    assert!(!after.contains("太啰嗦"), "批注删除后不应残留");
}

#[tokio::test]
async fn distill_corpus_lifecycle_and_recipe_guards() {
    use base64::Engine as _;
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p3&title=蒸"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p3"))
        .await
        .unwrap();

    // 无配方 → GET 返回空结构
    let empty = app
        .clone()
        .oneshot(get("/api/distill/recipe"))
        .await
        .unwrap();
    assert_eq!(empty.status(), StatusCode::OK);
    let body = body_text(empty).await;
    assert!(body.contains("\"books\":[]"), "未蒸馏时配方应为空: {body}");

    // 无配方调整强度 → 400
    let no_recipe = app
        .clone()
        .oneshot(put_form("/api/distill/recipe/update", "strength=0.5"))
        .await
        .unwrap();
    assert_eq!(no_recipe.status(), StatusCode::BAD_REQUEST);

    // 上传 txt 语料(base64)
    let b64_raw = base64::engine::general_purpose::STANDARD
        .encode("夜色深重，他独自走在长街上。\n风穿过空荡的巷子。".as_bytes());
    // 表单编码：base64 的 + / = 需 URL 编码，否则 + 被解析为空格
    let b64 = b64_raw
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D");
    let upload = app
        .clone()
        .oneshot(post_form(
            "/api/distill/corpus",
            &format!("title=测试风格&format=txt&content_b64={b64}&weight=0.8"),
        ))
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::OK);
    let uploaded = body_text(upload).await;
    assert!(uploaded.contains("\"chars\""), "应返回语料字数: {uploaded}");

    // 列表含语料
    let list = app
        .clone()
        .oneshot(get("/api/distill/corpus/list"))
        .await
        .unwrap();
    let list_body = body_text(list).await;
    assert!(list_body.contains("测试风格"), "语料应出现在列表: {list_body}");

    // 非法格式 → 400
    let bad = app
        .oneshot(post_form(
            "/api/distill/corpus",
            &format!("title=x&format=docx&content_b64={b64}"),
        ))
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cascade_analyze_backward_only_and_heuristic_fallback() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p4&title=联"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p4"))
        .await
        .unwrap();
    let c1 = body_text(
        app.clone()
            .oneshot(post_form("/api/outline/chapters/create", "title=一章"))
            .await
            .unwrap(),
    )
    .await;
    let c2 = body_text(
        app.clone()
            .oneshot(post_form("/api/outline/chapters/create", "title=二章"))
            .await
            .unwrap(),
    )
    .await;

    // 第 2 章正文引用林默（旧境界）
    app.clone()
        .oneshot(put_form(
            "/api/outline/chapters/content",
            &format!("chapter_id={c2}&content=林默已是筑基期，与敌人对峙。"),
        ))
        .await
        .unwrap();

    // 无内容变化 → 400
    let no_change = app
        .clone()
        .oneshot(post_form(
            "/api/writing/cascade/analyze",
            &format!("chapter_id={c1}&original=一样&rewritten=一样"),
        ))
        .await
        .unwrap();
    assert_eq!(no_change.status(), StatusCode::BAD_REQUEST);

    // 启发式路径：修改章 1 中林默境界（原文含林默，改写后差异 → 提取实体）
    // 原稿必须含林默(启发式要求实体出现在原稿)
    let analyze = app
        .clone()
        .oneshot(post_form(
            "/api/writing/cascade/analyze",
            &format!(
                "chapter_id={c1}&original=林默进入筑基期。&rewritten=林默突破到金丹期。"
            ),
        ))
        .await
        .unwrap();
    assert_eq!(analyze.status(), StatusCode::OK);
    let body = body_text(analyze).await;
    assert!(body.contains("changed_facts"), "应返回变更事实: {body}");
    assert!(body.contains("affected"), "应返回受影响分析: {body}");

    // apply 未配置 LLM → 400 引导配置
    let apply = app
        .oneshot(post_form(
            "/api/writing/cascade/apply",
            &format!(
                "chapter_id={c1}&target_chapter_ids={c2}&changed_facts=[{{\"entity\":\"林默\",\"attribute\":\"境界\",\"old_value\":\"筑基\",\"new_value\":\"金丹\"}}]"
            ),
        ))
        .await
        .unwrap();
    assert_eq!(apply.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn detail_import_creates_and_updates_chapters() {
    let (app, _dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p5&title=细"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p5"))
        .await
        .unwrap();
    let c1 = body_text(
        app.clone()
            .oneshot(post_form("/api/outline/chapters/create", "title=旧标题"))
            .await
            .unwrap(),
    )
    .await;

    let items = serde_json::json!([
        {"chapter_no": 1, "title": "开篇钩子", "summary": "要点一\n要点二\n关键事件：主角觉醒\n期望字数：2000"},
        {"chapter_no": 2, "title": "风起", "summary": "反派登场\n情绪曲线：平静→紧张"}
    ]);
    let import = app
        .clone()
        .oneshot(post_form(
            "/api/outline/detail/import",
            &format!("detail_json={}", items),
        ))
        .await
        .unwrap();
    assert_eq!(import.status(), StatusCode::OK);
    let body = body_text(import).await;
    assert!(body.contains("created"), "应返回导入统计: {body}");

    // 验证：第 1 章标题已更新，第 2 章已创建
    let list = body_text(app.clone().oneshot(get("/api/outline/chapters")).await.unwrap()).await;
    assert!(list.contains("开篇钩子"), "第 1 章标题应更新: {list}");
    assert!(list.contains("风起"), "第 2 章应创建: {list}");
    let _ = &c1;

    // 非法 JSON → 400
    let bad = app
        .oneshot(post_form("/api/outline/detail/import", "detail_json=not-json"))
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn archive_compress_and_log_rollback() {
    let (app, dir) = test_app();
    app.clone()
        .oneshot(post_form("/api/projects/create", "project_id=p6&title=档"))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_form("/api/projects/open", "project_id=p6"))
        .await
        .unwrap();

    // 建 5 章并给前 3 章写入正文
    for no in 1..=5 {
        let cid = body_text(
            app.clone()
                .oneshot(post_form(
                    "/api/outline/chapters/create",
                    &format!("title=第{no}章"),
                ))
                .await
                .unwrap(),
        )
        .await;
        if no <= 3 {
            app.clone()
                .oneshot(put_form(
                    "/api/outline/chapters/content",
                    &format!("chapter_id={cid}&content=第{no}章的正文内容，用于归档测试。"),
                ))
                .await
                .unwrap();
        }
    }

    // 归档压缩：保留最近 2 章 → 归档 3 章
    let compress = app
        .clone()
        .oneshot(post_form("/api/archive/compress", "keep_recent=2"))
        .await
        .unwrap();
    assert_eq!(compress.status(), StatusCode::OK);
    let body = body_text(compress).await;
    assert!(body.contains("\"archived\":3"), "应归档 3 章: {body}");

    // 归档列表
    let list = app
        .clone()
        .oneshot(get("/api/archive/list"))
        .await
        .unwrap();
    let list_body = body_text(list).await;
    assert!(list_body.contains("volumes"), "应返回卷摘要: {list_body}");

    // 成本报告
    let cost = app
        .clone()
        .oneshot(get("/api/log/cost"))
        .await
        .unwrap();
    assert_eq!(cost.status(), StatusCode::OK);
    let cost_body = body_text(cost).await;
    assert!(cost_body.contains("agent_bindings"), "成本报告应含 Agent 绑定: {cost_body}");

    // 无日志时回滚 → 400
    let no_log = app
        .clone()
        .oneshot(post_form("/api/log/rollback", "last_n=1"))
        .await
        .unwrap();
    assert_eq!(no_log.status(), StatusCode::BAD_REQUEST);

    // 手工写入操作日志（模拟一次 new_organization 提取）→ 回滚应删除该组织
    let cfg = dir.path().join("_config");
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::write(
        cfg.join("operation-log.json"),
        r#"[{"time":"2026-08-13T00:00:00Z","chapter_id":"x","applied":["新组织: 青云宗"],"skipped":[],"warnings":[],"facts":[{"kind":"new_organization","name":"青云宗","attribute":null,"old_value":null,"new_value":null}]}]"#,
    )
    .unwrap();

    let rollback = app
        .clone()
        .oneshot(post_form("/api/log/rollback", "last_n=1"))
        .await
        .unwrap();
    assert_eq!(rollback.status(), StatusCode::OK);
    let rb = body_text(rollback).await;
    assert!(rb.contains("删除组织"), "应回滚组织创建: {rb}");

    // 日志查询
    let ops = app.oneshot(get("/api/log/operations")).await.unwrap();
    assert_eq!(ops.status(), StatusCode::OK);
}
