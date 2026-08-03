//! 版本信息与更新检查命令
//!
//! - `app_version`：返回当前应用版本（与 Cargo.toml / tauri.conf.json 保持一致）。
//! - `check_latest_release`：请求 GitHub Releases API 查询最新版本与更新日志，
//!   供前端展示「检查更新」结果；实际自动安装由 tauri-plugin-updater 完成。
use crate::state::AppState;
use serde::Serialize;

/// 当前应用版本（编译期注入，与 Cargo.toml 同步）
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub 仓库（检查更新与下载链接）
const GITHUB_REPO: &str = "Cnkore007/PenSoul";

/// 检查更新结果
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// 是否有可用更新（远端版本 > 当前版本）
    pub has_update: bool,
    /// 当前版本
    pub current_version: String,
    /// 最新版本（无更新时为当前版本）
    pub latest_version: String,
    /// 更新日志（Release 正文，可能为空）
    pub notes: String,
    /// 下载页 URL（GitHub Releases 页面）
    pub url: String,
}

/// 返回当前应用版本
#[tauri::command]
pub async fn app_version() -> Result<String, String> {
    Ok(APP_VERSION.to_string())
}

/// 检查 GitHub 最新 Release 是否晚于当前版本（语义化版本比较）
#[tauri::command]
pub async fn check_latest_release() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("初始化更新检查失败: {e}"))?;

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let resp = client
        .get(&url)
        .header("User-Agent", "PenSoul-Updater")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("检查更新失败（网络错误）: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "检查更新失败（GitHub 返回 {status}），请稍后重试",
            status = resp.status()
        ));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析更新信息失败: {e}"))?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let notes = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("https://github.com/{GITHUB_REPO}/releases"))
        .to_string();

    let has_update = !tag.is_empty()
        && tag != APP_VERSION
        && version_gt(&tag, APP_VERSION);

    Ok(UpdateInfo {
        has_update,
        current_version: APP_VERSION.to_string(),
        latest_version: if tag.is_empty() { APP_VERSION.to_string() } else { tag },
        notes,
        url: html_url,
    })
}

/// 语义化版本比较：a > b 返回 true（忽略 v 前缀与 -beta/-rc 等预发布后缀）
fn version_gt(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    }
    let pa = parts(a);
    let pb = parts(b);
    for (x, y) in pa.iter().zip(pb.iter()) {
        if x != y {
            return x > y;
        }
    }
    pa.len() > pb.len()
}

/// 获取当前应用版本（供状态栏/关于等 UI 使用）
pub async fn current_version(_state: &AppState) -> String {
    APP_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_gt_basic() {
        assert!(version_gt("1.0.3", "1.0.2"));
        assert!(version_gt("1.1.0", "1.0.9"));
        assert!(!version_gt("1.0.2", "1.0.2"));
        assert!(!version_gt("1.0.2", "1.0.3"));
    }

    #[test]
    fn test_version_gt_with_prefix_and_suffix() {
        assert!(version_gt("v1.0.3", "1.0.2"));
        assert!(version_gt("1.0.3-beta", "1.0.2"));
        assert!(!version_gt("1.0.2-beta", "1.0.2"));
    }
}
