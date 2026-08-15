// labels.ts — 用户可见文案中文化映射
// 规则：中文为主，英文术语以「中文（English）」标注；找不到映射时回退原文

export const chapterStatusLabels: Record<string, string> = {
  Draft: "草稿（Draft）",
  Reviewing: "审阅中（Reviewing）",
  Reviewed: "已审阅（Reviewed）",
  Polished: "已润色（Polished）",
  Published: "已发布（Published）",
};

export const foreshadowStatusLabels: Record<string, string> = {
  Planned: "待埋（Planned）",
  Planted: "已埋（Planted）",
  Progressing: "推进中（Progressing）",
  Resolved: "已回收（Resolved）",
  Abandoned: "已废弃（Abandoned）",
  Overdue: "已逾期（Overdue）",
};

export const entityTypeLabels: Record<string, string> = {
  Character: "角色（Character）",
  Event: "事件（Event）",
  Setting: "设定（Setting）",
  Foreshadow: "伏笔（Foreshadow）",
};

export const thinkingModeLabels: Record<string, string> = {
  None: "无（None）",
  Always: "总是（Always）",
  Toggleable: "可切换（Toggleable）",
};

export const editingModeLabels: Record<string, string> = {
  Drafting: "起草（Drafting）",
  Revising: "修改（Revising）",
  Reviewing: "审查（Reviewing）",
};

export const providerLabels: Record<string, string> = {
  openai: "OpenAI",
  moonshot: "Moonshot（月之暗面）",
  deepseek: "DeepSeek（深度求索）",
  anthropic: "Anthropic（Claude）",
  custom: "自定义中转",
};

export function label(labels: Record<string, string>, key: string): string {
  return labels[key] ?? key;
}
