// 工作流解析助手 — 把「项目引用 + 全局模板」合并成各环节有效配置。
//
// 解析优先级（与后端 pipeline/runner.rs 保持一致）：
// 显式参数 > 项目覆盖（workflowRef.overrides） > 模板绑定（template.bindings） > 空/自动
import type {
  WorkflowTemplate,
  WorkflowRef,
  WorkflowSkillConfig,
  StageSkillConfig,
} from "./types";

// 三个可绑定技能卡/模型的执行环节（key 与后端 applicable_stages 一致）
export const EXEC_STAGES = ["outline_expand", "chapter_writing", "review"] as const;

export function emptyStageConfig(): StageSkillConfig {
  return { model: null, cards: [] };
}

// 某环节有效配置：项目覆盖优先，模板绑定兜底
export function effectiveStageConfig(
  templates: WorkflowTemplate[],
  ref: WorkflowRef | null | undefined,
  stage: string,
): StageSkillConfig {
  const overridden = ref?.overrides?.[stage] as StageSkillConfig | undefined;
  const template = templates.find(t => t.template_id === ref?.template_id);
  const bound = template?.bindings?.[stage] as StageSkillConfig | undefined;
  const model = overridden?.model ?? bound?.model ?? null;
  const cards = Array.isArray(overridden?.cards)
    ? overridden.cards
    : Array.isArray(bound?.cards)
      ? bound.cards
      : [];
  return { model: model ?? null, cards: cards as string[] };
}

// 合并出三个执行环节的有效配置（供大纲展开/造化工坊直接使用）
export function computeEffectiveSkills(
  templates: WorkflowTemplate[],
  ref: WorkflowRef | null | undefined,
): WorkflowSkillConfig {
  return {
    outline_expand: effectiveStageConfig(templates, ref, "outline_expand"),
    chapter_writing: effectiveStageConfig(templates, ref, "chapter_writing"),
    review: effectiveStageConfig(templates, ref, "review"),
  };
}
