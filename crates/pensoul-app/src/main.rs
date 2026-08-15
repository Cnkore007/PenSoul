// pensoul-server: HTTP API 服务入口

use pensoul_app::api::create_router;
use pensoul_app::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    println!("[PenSoul] 启动 HTTP 服务...");

    // 数据目录
    let base_dir = PathBuf::from("data");
    std::fs::create_dir_all(&base_dir).ok();
    println!("[PenSoul] 数据目录: {:?}", base_dir);

    let state = Arc::new(RwLock::new(AppState::new(
        base_dir.to_string_lossy().to_string(),
    )));
    println!("[PenSoul] AppState 创建成功");

    // 恢复上次打开的项目（若存在）：避免前端页面仍停在项目内、后端重启后
    // 内存状态被清空导致项目级接口报「没有打开的项目」
    {
        let mut guard = state.write().await;
        if let Some(project_id) = guard.restore_last_project() {
            println!("[PenSoul] 已恢复上次打开的项目: {project_id}");
        }
    }

    let app = create_router(state).layer(pensoul_app::build_cors());

    // 只监听本机回环地址，避免局域网内任意设备访问本地数据；
    // 端口可用环境变量 PENSOUL_PORT 覆盖（默认 3001）
    let port = std::env::var("PENSOUL_PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("127.0.0.1:{port}");
    println!("[PenSoul] 监听: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
