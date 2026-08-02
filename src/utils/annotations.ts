// 全链路批注状态管理 hook —— 以 target 定位串为维度，增删改查走后端 IPC
import { useCallback, useEffect, useState } from "react";
import {
  annotationAdd,
  annotationRemove,
  annotationResolve,
  annotationUpdate,
  annotationsList,
} from "../ipc";
import type { ChapterAnnotation } from "../types";

export interface AnnotationHandlers {
  annotations: ChapterAnnotation[];
  open: boolean;
  setOpen: (v: boolean) => void;
  add: (kind: ChapterAnnotation["kind"], content: string) => Promise<void>;
  update: (id: string, patch: { kind?: string; content?: string; status?: string }) => Promise<void>;
  remove: (id: string) => Promise<void>;
  resolve: (id: string, accept: boolean) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useAnnotations(target: string): AnnotationHandlers {
  const [annotations, setAnnotations] = useState<ChapterAnnotation[]>([]);
  const [open, setOpen] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setAnnotations(await annotationsList(target));
    } catch (e) {
      console.error("加载批注失败:", target, e);
    }
  }, [target]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  const add = useCallback(
    async (kind: ChapterAnnotation["kind"], content: string) => {
      await annotationAdd(target, kind, content);
      await refresh();
    },
    [target, refresh]
  );

  const update = useCallback(
    async (id: string, patch: { kind?: string; content?: string; status?: string }) => {
      await annotationUpdate(target, id, patch);
      await refresh();
    },
    [target, refresh]
  );

  const remove = useCallback(
    async (id: string) => {
      await annotationRemove(target, id);
      await refresh();
    },
    [target, refresh]
  );

  const resolve = useCallback(
    async (id: string, accept: boolean) => {
      await annotationResolve(target, [{ annotation_id: id, accept }]);
      await refresh();
    },
    [target, refresh]
  );

  return { annotations, open, setOpen, add, update, remove, resolve, refresh };
}
