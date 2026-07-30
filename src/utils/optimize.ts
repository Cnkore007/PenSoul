import { optimizeContent } from "../ipc";

/**
 * 全局优化管理器 —— 世界观/人物志「优化」的执行中枢。
 *
 * 解决三个问题：
 * 1. 优化在模块级执行，与 React 组件生命周期解耦——切换页面后优化继续，
 *    回到页面时内容已经更新（apply 使用 App 层稳定的 persistProjectData 函数式更新）。
 * 2. 每次优化前自动保存快照，支持「撤回」恢复到优化之前。
 * 3. 通过订阅机制向当前已挂载的页面广播进度（loading / 成功 / 失败）。
 */

export type OptimizeType = "world" | "character";

export interface OptimizeEvent {
  type: OptimizeType;
  kind: "start" | "done" | "error" | "undo";
  message?: string;
}

type Listener = (evt: OptimizeEvent) => void;

const running = new Set<OptimizeType>();
const backups = new Map<OptimizeType, string>();
const listeners = new Set<Listener>();

function emit(evt: OptimizeEvent) {
  listeners.forEach(fn => fn(evt));
}

export function subscribeOptimize(fn: Listener): () => void {
  listeners.add(fn);
  return () => { listeners.delete(fn); };
}

export function isOptimizing(type: OptimizeType): boolean {
  return running.has(type);
}

export function hasOptimizeBackup(type: OptimizeType): boolean {
  return backups.has(type);
}

/**
 * 启动优化。contentJson 为优化前的页面数据（同时作为撤回快照）。
 * apply 负责把优化结果写回项目数据（应使用函数式更新，保证组件卸载后仍生效）。
 * modelId 为空时使用后端默认模型。
 */
export function startOptimize(
  type: OptimizeType,
  contentJson: string,
  modelId: string | null,
  apply: (parsed: any) => void,
): boolean {
  if (running.has(type)) return false;
  running.add(type);
  backups.set(type, contentJson); // 优化前快照
  emit({ type, kind: "start" });

  (async () => {
    try {
      const result = await optimizeContent(type, contentJson, modelId);
      const parsed = JSON.parse(result);
      apply(parsed);
      emit({ type, kind: "done", message: "已优化整理本页内容，可点击「撤回」恢复" });
    } catch (e: any) {
      // 失败时不留下快照，避免误导用户以为有东西可撤回
      backups.delete(type);
      emit({ type, kind: "error", message: `优化失败: ${e?.message || e}` });
    } finally {
      running.delete(type);
    }
  })();
  return true;
}

/**
 * 撤回上一次优化：把页面内容恢复到优化前的快照。
 * apply 语义同 startOptimize。
 */
export function undoOptimize(type: OptimizeType, apply: (parsed: any) => void): boolean {
  const snapshot = backups.get(type);
  if (snapshot === undefined) return false;
  try {
    apply(JSON.parse(snapshot));
    backups.delete(type);
    emit({ type, kind: "undo", message: "已恢复到优化之前" });
    return true;
  } catch {
    backups.delete(type);
    emit({ type, kind: "error", message: "撤回失败：快照已损坏" });
    return false;
  }
}
