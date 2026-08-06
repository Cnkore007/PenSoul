// 蓝图作者语言翻译：把内部枚举/规则 ID 翻译成作者能看懂的表述
import type { BlueprintReport, BookBlueprint, CheckIssue } from "../types";

// ── 状态与枚举翻译 ──

export function commitmentStatusLabel(status: string): string {
  switch (status) {
    case "active": return "在守";
    case "fulfilled": return "已兑现";
    case "waived": return "已豁免";
    case "broken": return "破了";
    default: return status;
  }
}

export function commitmentKindLabel(kind: string): string {
  switch (kind) {
    case "theme": return "主题";
    case "promise": return "读者承诺";
    case "tone": return "基调";
    case "rule": return "铁律";
    case "no_go": return "禁区";
    default: return kind;
  }
}

export function roleLabel(role: string): string {
  switch (role) {
    case "protagonist": return "主角";
    case "ally": return "盟友";
    case "mentor": return "导师";
    case "antagonist": return "对手";
    case "love_interest": return "恋人";
    case "supporting": return "配角";
    case "rival": return "劲敌";
    case "group": return "群像";
    default: return role;
  }
}

export function functionLabel(fn: string): string {
  switch (fn) {
    case "setup": return "开局";
    case "escalation": return "升级";
    case "climax": return "高潮";
    case "resolution": return "收束";
    default: return fn;
  }
}

export function entityTypeLabel(type: string): string {
  switch (type) {
    case "character": return "人物";
    case "location": return "地点";
    case "faction": return "势力";
    default: return type;
  }
}

export function foreshadowStatusLabel(status: string): string {
  switch (status) {
    case "planned": return "待埋设";
    case "planted": return "已埋设";
    case "progressing": return "推进中";
    case "resolved": return "已回收";
    case "paid_off": return "已回收";
    case "waived": return "已放弃";
    case "abandoned": return "已放弃";
    case "overdue": return "已逾期";
    case "broken": return "破了";
    default: return status;
  }
}

export function subplotStatusLabel(status: string): string {
  switch (status) {
    case "planned": return "待启动";
    case "active": return "进行中";
    case "paused": return "暂停";
    case "dormant": return "休眠";
    case "resolved": return "已解决";
    case "abandoned": return "已放弃";
    default: return status;
  }
}

export function resourceStatusLabel(status: string): string {
  switch (status) {
    case "available": return "可用";
    case "used": return "已使用";
    case "consumed": return "已消耗";
    case "lost": return "已丢失";
    case "destroyed": return "已毁";
    case "transferred": return "已转手";
    case "revealed": return "已公开";
    default: return status;
  }
}

export function volumeStatusLabel(status: string): string {
  switch (status) {
    case "planned": return "待规划";
    case "outlined": return "已定纲";
    case "drafting": return "写作中";
    case "closed": return "已完成";
    default: return status;
  }
}

// ── 检查报告翻译：规则 ID → 作者话术 ──

interface IssueHint {
  title: (target: string) => string;
  fix: string;
}

const RULE_HINTS: Record<string, IssueHint> = {
  "CMT-H1": {
    title: target => `承诺「${target}」还差一个兑现安排`,
    fix: "要么写明它在第几章兑现，要么把它标为持续承诺。可在蓝图页修改。",
  },
  "VOL-H1": {
    title: () => "卷与卷的章节范围重叠或没接上",
    fix: "在蓝图里调整各卷的起止章，让它们连续且不重叠。",
  },
  "VOL-H2": {
    title: target => `第 ${target} 卷还没有高潮`,
    fix: "补上这一卷的高潮场景与高潮章节。",
  },
  "VOL-H3": {
    title: target => `第 ${target} 卷缺卷末钩子`,
    fix: "给卷尾安排一个悬念钩子，勾住读者往下看。",
  },
  "VOL-S1": {
    title: target => `第 ${target} 卷的读者承诺没有登记`,
    fix: "把这一卷对读者的承诺补进承诺清单，或删除卷描述里的承诺话。",
  },
  "VOL-S2": {
    title: () => "节奏缺口：卷首缺少钩子",
    fix: "在卷的开头 10% 章节安排一个钩子或冲突升温点。",
  },
  "VOL-S3": {
    title: () => "爽点间隔太长",
    fix: "两次爽点之间别隔太远，在节奏表里补一个蓄力或兑现点。",
  },
  "FS-H1": {
    title: target => `伏笔「${target}」还没有回收安排`,
    fix: "给伏笔指定回收章、卷或事件，确定它最终会被解决。",
  },
  "FS-H2": {
    title: target => `伏笔「${target}」埋设章已过，正文里却没埋`,
    fix: "回到对应章节补上伏笔，或把埋设章改到后面。",
  },
  "FS-H3": {
    title: target => `伏笔「${target}」过了回收章还没解决`,
    fix: "在正文中安排回收，或明确调整回收章并留痕。",
  },
  "CHR-H1": {
    title: target => `角色「${target}」还没有核心欲望`,
    fix: "在人物档案里补上他最想要的东西——没有欲望就没有故事。",
  },
  "SP-H1": {
    title: target => `支线「${target}」很久没出现了`,
    fix: "尽快让这条支线回到正文，或把它暂停并记录悬念。",
  },
  "DOS-H2": {
    title: target => `角色状态卡「${target}」的变更记录对不上当前状态`,
    fix: "检查状态卡的技术细节，修正变更记录或当前状态，保证留痕一致。",
  },
};

/** 从检查结果翻译成作者话术：返回标题与建议，找不到规则映射时退回原文 */
export function translateIssue(issue: CheckIssue, bp: BookBlueprint): { title: string; fix: string } {
  const hint = RULE_HINTS[issue.rule_id];
  if (!hint) {
    return { title: issue.message, fix: "详见技术细节中的规则 " + issue.rule_id };
  }
  const target = resolveTargetName(issue, bp);
  return { title: hint.title(target), fix: hint.fix };
}

/** 把 target_id（cmt-001 / fs-002 等）解析成作者能识别的名字 */
function resolveTargetName(issue: CheckIssue, bp: BookBlueprint): string {
  const id = issue.target_id;
  if (issue.ledger === "commitments") {
    const it = bp.commitments.find(c => c.commitment_id === id);
    return it ? it.statement.slice(0, 18) + "…" : id;
  }
  if (issue.ledger === "skeleton") {
    const it = bp.volumes.find(v => String(v.volume_no) === id);
    return it ? `第 ${it.volume_no} 卷` : id;
  }
  if (issue.ledger === "foreshadows") {
    const it = bp.foreshadows.find(f => f.foreshadow_id === id);
    return it ? it.name : id;
  }
  if (issue.ledger === "characters") {
    const it = bp.character_matrix.find(c => c.character_name === id);
    return it ? it.character_name : id;
  }
  if (issue.ledger === "subplots") {
    const it = bp.subplots.find(s => s.subplot_id === id);
    return it ? it.name : id;
  }
  if (issue.ledger === "resources") {
    const it = bp.resources.find(r => r.resource_id === id);
    return it ? it.name : id;
  }
  if (issue.ledger === "dossiers") {
    const it = bp.dossiers.find(d => d.entity_id === id);
    return it ? it.name : id;
  }
  return id;
}

/** 统计报告摘要，供顶部提示使用 */
export function reportSummary(report: BlueprintReport): string {
  const hard = report.hard_count;
  const soft = report.soft_count;
  if (hard === 0 && soft === 0) return "全部通过，没有需要处理的问题";
  if (hard === 0) return `没有硬性问题，有 ${soft} 条建议`;
  return `${hard} 条硬性问题需要处理，${soft} 条建议`;
}
