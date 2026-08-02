import React, { useCallback, useEffect, useRef, useState } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import { Mark, mergeAttributes } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Underline from "@tiptap/extension-underline";
import Placeholder from "@tiptap/extension-placeholder";
import {
  Bold,
  Italic,
  Underline as UnderlineIcon,
  Heading1,
  Heading2,
  List,
  ListOrdered,
  MessageSquarePlus,
  X,
} from "lucide-react";
import type { AnnotationAnchor, ChapterAnnotation } from "../types";

// 行内批注 Mark：正文中批注过的文字显示高亮，锚点由批注数据（段落索引 + 偏移）定位
const AnnotationMark = Mark.create({
  name: "annotationMark",
  addAttributes() {
    return {
      annoId: { default: null },
    };
  },
  parseHTML() {
    return [{ tag: "span[data-anno-id]" }];
  },
  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes({ class: "anno-marker" }, HTMLAttributes)];
  },
  addCommands() {
    return {
      setAnnoMark:
        (annoId: string) =>
        ({ tr, dispatch }: any) => {
          const { from, to } = tr.selection;
          if (from === to) return false;
          tr = tr.addMark(from, to, this.type.create({ annoId }));
          if (dispatch) dispatch(tr);
          return true;
        },
      unsetAnnoMark:
        (annoId: string) =>
        ({ tr, dispatch }: any) => {
          let changed = false;
          tr.doc.descendants((node: any, pos: number) => {
            if (!node.isText) return;
            for (const m of node.marks) {
              if (m.type.name === "annotationMark" && m.attrs.annoId === annoId) {
                tr = tr.removeMark(pos, pos + node.nodeSize, m);
                changed = true;
              }
            }
          });
          if (dispatch && changed) dispatch(tr);
          return changed;
        },
    } as any;
  },
});

interface TipTapEditorProps {
  content: string;
  onChange: (html: string) => void;
  placeholder?: string;
  // 行内批注：供删除/高亮同步
  annotations?: ChapterAnnotation[];
  // 新建批注：返回新批注 ID（null = 取消）
  onAddAnnotation?: (
    anchor: AnnotationAnchor,
    kind: "issue" | "suggestion" | "note",
    content: string
  ) => string | null;
}

interface SelectionState {
  from: number;
  to: number;
  text: string;
  top: number;
  left: number;
}

const KIND_LABELS: Record<string, string> = {
  issue: "问题",
  suggestion: "修改建议",
  note: "备注",
};

export function TipTapEditor({
  content,
  onChange,
  placeholder = "落笔之处，便是江湖...",
  annotations = [],
  onAddAnnotation,
}: TipTapEditorProps) {
  const editor = useEditor({
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2] } }),
      Underline,
      Placeholder.configure({ placeholder }),
      AnnotationMark,
    ],
    content,
    onUpdate: ({ editor }) => {
      onChange(editor.getHTML());
    },
  });

  // 选中文本 → 批注浮层
  const [sel, setSel] = useState<SelectionState | null>(null);
  const [composerOpen, setComposerOpen] = useState(false);
  const [composerKind, setComposerKind] = useState<"issue" | "suggestion" | "note">("issue");
  const [composerText, setComposerText] = useState("");
  const annoIds = annotations.map(a => a.annotation_id);
  const removedIdsRef = useRef<string[]>([]);

  // 监听选中变化：非空选中且同一段落内时，浮出「批注」按钮
  useEffect(() => {
    if (!editor) return;
    const onSelection = () => {
      const { from, to, empty } = editor.state.selection;
      if (empty || to - from > 600) {
        if (!composerOpen) setSel(null);
        return;
      }
      const text = editor.state.doc.textBetween(from, to, " ");
      if (!text.trim()) {
        if (!composerOpen) setSel(null);
        return;
      }
      const rect = editor.view.coordsAtPos(from);
      setSel({ from, to, text, top: rect.top, left: rect.left });
    };
    editor.on("selectionUpdate", onSelection);
    return () => {
      editor.off("selectionUpdate", onSelection);
    };
  }, [editor, composerOpen]);

  // 批注数据变化：消失的批注移除正文中的高亮 mark
  useEffect(() => {
    if (!editor) return;
    const gone = removedIdsRef.current.filter(id => !annoIds.includes(id));
    if (gone.length === 0) return;
    for (const id of gone) {
      (editor.chain() as any).unsetAnnoMark(id).run();
    }
    removedIdsRef.current = removedIdsRef.current.filter(id => annoIds.includes(id));
  }, [editor, annoIds]);

  // 计算选中文本的段落索引与段内偏移（按顶层节点计数，与正文 HTML 段落一致）
  function computeAnchor(from: number): { paragraph_index: number; offset: number } {
    let index = 0;
    let found: { paragraph_index: number; offset: number } | null = null;
    editor!.state.doc.forEach((node, pos) => {
      if (found) return;
      const nodeEnd = pos + node.nodeSize;
      if (from >= pos && from < nodeEnd) {
        const offset = editor!.state.doc.textBetween(pos + 1, from, "", "").length;
        found = { paragraph_index: index, offset };
      }
      index += 1;
    });
    return found ?? { paragraph_index: 0, offset: 0 };
  }

  function handleAddAnnotation() {
    if (!editor || !sel || !composerText.trim() || !onAddAnnotation) return;
    const { paragraph_index, offset } = computeAnchor(sel.from);
    const anchor: AnnotationAnchor = {
      paragraph_index,
      offset,
      text: sel.text.trim(),
    };
    const newId = onAddAnnotation(anchor, composerKind, composerText.trim());
    if (newId) {
      (editor.chain() as any).focus().setAnnoMark(newId).run();
    }
    setComposerOpen(false);
    setComposerText("");
    setSel(null);
  }

  const ToolbarButton = useCallback(
    ({
      onClick,
      isActive = false,
      children,
    }: {
      onClick: () => void;
      isActive?: boolean;
      children: React.ReactNode;
    }) => (
      <button
        type="button"
        onClick={onClick}
        className={`toolbar-btn ${isActive ? "active" : ""}`}
      >
        {children}
      </button>
    ),
    []
  );

  if (!editor) {
    return <div className="loading-state">墨磨中...</div>;
  }

  return (
    <div className="tiptap-editor-wrapper">
      <div className="tiptap-toolbar">
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleBold().run()}
          isActive={editor.isActive("bold")}
        >
          <Bold size={15} />
        </ToolbarButton>
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleItalic().run()}
          isActive={editor.isActive("italic")}
        >
          <Italic size={15} />
        </ToolbarButton>
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleUnderline().run()}
          isActive={editor.isActive("underline")}
        >
          <UnderlineIcon size={15} />
        </ToolbarButton>
        <div className="tiptap-toolbar-divider" />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
          isActive={editor.isActive("heading", { level: 1 })}
        >
          <Heading1 size={15} />
        </ToolbarButton>
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
          isActive={editor.isActive("heading", { level: 2 })}
        >
          <Heading2 size={15} />
        </ToolbarButton>
        <div className="tiptap-toolbar-divider" />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleBulletList().run()}
          isActive={editor.isActive("bulletList")}
        >
          <List size={15} />
        </ToolbarButton>
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
          isActive={editor.isActive("orderedList")}
        >
          <ListOrdered size={15} />
        </ToolbarButton>
      </div>
      <div className="tiptap-content">
        <EditorContent editor={editor} />
      </div>

      {/* 选中文本 → 「批注」按钮 */}
      {sel && !composerOpen && (
        <div
          className="anno-trigger"
          style={{ position: "fixed", top: Math.max(8, sel.top - 38), left: sel.left }}
        >
          <button
            className="btn btn-accent"
            style={{ padding: "3px 10px", fontSize: "var(--text-2xs)", whiteSpace: "nowrap" }}
            onClick={() => setComposerOpen(true)}
          >
            <MessageSquarePlus size={13} /> 批注
          </button>
        </div>
      )}

      {/* 批注输入浮层 */}
      {composerOpen && sel && (
        <div
          className="anno-composer"
          style={{ position: "fixed", top: Math.max(8, sel.top - 150), left: sel.left, zIndex: 50 }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
            <span style={{ fontSize: "var(--text-xs)", fontWeight: 600, color: "var(--color-ink)" }}>添加批注</span>
            <select
              className="pm-input"
              style={{ marginBottom: 0, width: 110, padding: "2px 6px", fontSize: "var(--text-2xs)" }}
              value={composerKind}
              onChange={e => setComposerKind(e.target.value as any)}
            >
              {(Object.keys(KIND_LABELS) as Array<keyof typeof KIND_LABELS>).map(k => (
                <option key={k} value={k}>{KIND_LABELS[k]}</option>
              ))}
            </select>
            <button className="pv-icon-btn" style={{ marginLeft: "auto" }} title="关闭"
              onClick={() => { setComposerOpen(false); setComposerText(""); setSel(null); }}>
              <X size={13} />
            </button>
          </div>
          <div style={{ fontSize: "var(--text-2xs)", color: "var(--color-ink-3)", marginBottom: 4, maxWidth: 320 }}>
            「{sel.text.length > 30 ? sel.text.slice(0, 30) + "…" : sel.text}」
          </div>
          <textarea
            className="pm-textarea"
            rows={3}
            style={{ width: 320, maxWidth: "80vw" }}
            placeholder={composerKind === "suggestion" ? "怎么写更好？给出具体改法…" : "写下你的意见…"}
            value={composerText}
            autoFocus
            onChange={e => setComposerText(e.target.value)}
          />
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 6, marginTop: 6 }}>
            <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: "var(--text-xs)" }}
              onClick={handleAddAnnotation} disabled={!composerText.trim()}>
              添加
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
