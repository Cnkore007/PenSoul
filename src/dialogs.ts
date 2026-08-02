import { confirm, message } from "@tauri-apps/plugin-dialog";

// Tauri 2 的 webview 不支持 window.confirm / alert / prompt（调用会报
// "dialog.confirm not allowed. Command not found"），统一改用 dialog 插件；
// 非 Tauri 环境（浏览器调试）自动降级到原生方法。

export async function confirmDialog(messageText: string): Promise<boolean> {
  try {
    return await confirm(messageText, { title: "PenSoul", kind: "warning" });
  } catch {
    return window.confirm(messageText);
  }
}

export async function messageDialog(messageText: string): Promise<void> {
  try {
    await message(messageText, { title: "PenSoul" });
  } catch {
    window.alert(messageText);
  }
}
