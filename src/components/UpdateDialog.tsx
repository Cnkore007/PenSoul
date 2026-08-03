import { useEffect, useState } from "react";
import { X, Download, Loader2, ExternalLink, Sparkles } from "lucide-react";
import { appVersion, checkLatestRelease } from "../ipc";

interface UpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  notes: string;
  url: string;
}

interface UpdateDialogProps {
  onClose: () => void;
}

// 更新弹窗：展示当前版本、检查 GitHub 最新 Release，有更新时引导自动安装或下载。
// 自动安装走 tauri-plugin-updater（Windows 可用；macOS 需应用签名，
// 未签名时降级为打开下载页手动安装）。
export function UpdateDialog({ onClose }: UpdateDialogProps) {
  const [version, setVersion] = useState("");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState("");
  const [installMsg, setInstallMsg] = useState("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const v = await appVersion();
        if (cancelled) return;
        setVersion(v);
        const res = await checkLatestRelease();
        if (cancelled) return;
        setInfo(res);
      } catch (e: any) {
        if (cancelled) return;
        setError(typeof e === "string" ? e : e?.message || String(e));
      } finally {
        if (!cancelled) setChecking(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const isWin = navigator.userAgent.includes("Win");

  // 自动安装：updater 插件下载并安装；失败时降级为打开下载页
  async function handleInstall() {
    if (!info) return;
    setInstalling(true);
    setError("");
    setInstallMsg("");
    try {
      const updater = await import("@tauri-apps/plugin-updater");
      const update = await updater.check();
      if (!update) {
        setInstallMsg("已是最新版本，无需更新");
        return;
      }
      setInstallMsg("正在下载更新包…");
      await update.downloadAndInstall();
      setInstallMsg("更新已下载完成，请重启应用完成安装");
    } catch (e: any) {
      // updater 不可用（如 macOS 未签名 / 清单缺失）时打开下载页
      setInstallMsg("");
      setError((typeof e === "string" ? e : e?.message || String(e)) + "\n已为你打开下载页，可手动下载安装。");
      try {
        const { open } = await import("@tauri-apps/plugin-shell");
        await open(info.url);
      } catch { /* 打不开下载页时忽略 */ }
    } finally {
      setInstalling(false);
    }
  }

  async function handleOpenPage() {
    if (!info) return;
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(info.url);
    } catch {
      window.open(info.url, "_blank");
    }
  }

  return (
    <div className="pm-modal-mask" onClick={onClose}>
      <div className="pm-modal" style={{ maxWidth: 560 }} onClick={e => e.stopPropagation()}>
        <div className="pm-modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Sparkles size={17} style={{ color: "var(--color-accent)" }} />
            <span style={{ fontFamily: "var(--font-brush)", fontSize: "var(--text-md)", letterSpacing: "2px" }}>关于 PenSoul</span>
          </div>
          <button className="pv-icon-btn" onClick={onClose} title="关闭"><X size={15} /></button>
        </div>
        <div className="pm-modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>当前版本</span>
            <span style={{ fontSize: "var(--text-sm)", fontWeight: 600 }}>v{version || "…"}</span>
            <span style={{ marginLeft: "auto", display: "inline-flex", alignItems: "center", gap: 6, fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
              {checking && <Loader2 size={13} className="spinning" />}
              {!checking && info?.has_update && <span style={{ color: "var(--color-accent)", fontWeight: 500 }}>发现新版本 v{info.latest_version}</span>}
              {!checking && info && !info.has_update && <span style={{ color: "var(--color-jade)" }}>已是最新版本</span>}
            </span>
          </div>

          {!checking && info?.has_update && (
            <>
              {info.notes.trim() ? (
                <div style={{
                  maxHeight: 240, overflowY: "auto",
                  fontSize: "var(--text-xs)", lineHeight: 1.8,
                  color: "var(--color-ink-2)", whiteSpace: "pre-wrap",
                  padding: "var(--space-sm) var(--space-md)",
                  background: "var(--color-paper-warm)", borderRadius: "var(--radius-sm)",
                }}>
                  {info.notes}
                </div>
              ) : (
                <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>本版本未附带更新日志。</div>
              )}

              <div style={{ display: "flex", gap: 8, marginTop: 4 }}>
                <button className="btn btn-primary" onClick={handleInstall} disabled={installing}>
                  {installing
                    ? <><Loader2 size={14} className="spinning" /> {installMsg || "正在准备更新…"}</>
                    : <><Download size={14} /> {isWin ? "下载并自动安装" : "下载更新包"}</>}
                </button>
                <button className="btn btn-secondary" onClick={handleOpenPage}>
                  <ExternalLink size={14} /> 前往下载页
                </button>
              </div>
              {installMsg && !error && (
                <div style={{ fontSize: "var(--text-xs)", color: "var(--color-jade)" }}>{installMsg}</div>
              )}
            </>
          )}

          {!checking && !info?.has_update && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
              你的 PenSoul 已是最新版本。自动更新通过 GitHub Releases 分发，后续发版后在此提醒。
            </div>
          )}

          {error && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-error)", background: "var(--color-error-wash)", padding: "var(--space-sm) var(--space-md)", borderRadius: "var(--radius-sm)", whiteSpace: "pre-wrap" }}>
              {error}
            </div>
          )}

          {!info && !checking && !error && (
            <div style={{ fontSize: "var(--text-xs)", color: "var(--color-ink-3)" }}>
              暂时无法获取版本信息，请检查网络后重试。
            </div>
          )}
        </div>
        <div className="pm-modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>关闭</button>
        </div>
      </div>
    </div>
  );
}
