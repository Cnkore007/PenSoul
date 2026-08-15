// techniques.rs — 叙事技巧库（调研 F12 / F15）
// 内置可计算化的叙事技巧与网文节奏模板：每条含「生成指导」（注入提示词）
// 与「审校检查项」（供 AI 审校逐条验证）。
// 定位：软约束 / 建议制 —— 用户在笔耕页选择技巧，生成时显式注入，
// 不强制、可豁免，避免同质化。

use axum::extract::State;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiError;
use crate::state::AppState;

/// 技巧条目
#[derive(Debug, Clone, Serialize)]
pub struct Technique {
    pub id: &'static str,
    pub name: &'static str,
    /// 分类：叙事技巧 / 网文节奏
    pub category: &'static str,
    pub description: &'static str,
    /// 注入生成提示词的写作指导（一句话可执行）
    pub guidance: &'static str,
    /// 审校检查项（AI 审校时逐条验证）
    pub check_items: &'static [&'static str],
}

/// 列出全部可用技巧（F12/F15）
pub async fn list_techniques(
    State(_state): State<Arc<RwLock<AppState>>>,
) -> Result<String, ApiError> {
    serde_json::to_string(&all_techniques()).map_err(|e| ApiError::internal(e.to_string()))
}

/// 内置技巧库（真实数据，静态定义）
const TECHNIQUES: &[Technique] = &[
    // ---- 叙事技巧（F12，参考 ACL 2025 六技巧） ----
    Technique {
        id: "suspense",
        name: "悬念",
        category: "叙事技巧",
        description: "在场景/章节结尾留下未解问题或危险逼近感，让读者有继续阅读的动力。",
        guidance: "本章至少设置一个悬而未决的问题：可埋在段末、场景末或章末；问题要有具体利害，且不立即解答。",
        check_items: &["是否至少有一个未即时解答的悬念", "章末是否停在张力点而非松懈处"],
    },
    Technique {
        id: "reversal",
        name: "反转",
        category: "叙事技巧",
        description: "先建立读者的预期，再以合理方式打破，形成认知冲击。",
        guidance: "先铺垫一个看似确定的事实或方向，再在恰当时机给出颠覆性信息；反转必须由前文伏笔支撑，不能凭空。",
        check_items: &["反转是否有前文依据而非凭空出现", "反转是否带来信息增量而非单纯意外"],
    },
    Technique {
        id: "nonlinear",
        name: "非线性叙事",
        category: "叙事技巧",
        description: "通过闪回、闪前或多线交织制造层次感。",
        guidance: "允许闪回/多线交织，但每处时间跳转必须有清晰标记，不得破坏时间线可读性。",
        check_items: &["时间跳转是否有明确标记", "多线叙事是否最终汇合于主线"],
    },
    Technique {
        id: "irony",
        name: "讽刺",
        category: "叙事技巧",
        description: "言语、情境或结构上的错位，言与意、表与里的反差。",
        guidance: "通过人物言行与实际处境的反差制造讽刺，克制直白评判，让读者自行领会。",
        check_items: &["是否存在言与意的错位", "讽刺是否克制而非说教"],
    },
    Technique {
        id: "foreshadow",
        name: "伏笔呼应",
        category: "叙事技巧",
        description: "在早前章节埋设细节，在后续章节回收，形成前后呼应。",
        guidance: "主动与正典中活跃伏笔联动：本章可埋设新伏笔（记录到伏笔库）或为已有伏笔提供推进信号，不得提前回收未埋设的伏笔。",
        check_items: &["是否与活跃伏笔联动", "是否出现未埋先收的伏笔"],
    },
    Technique {
        id: "symbol",
        name: "象征",
        category: "叙事技巧",
        description: "以物件、意象或场景承载主题，形成贯穿性的象征系统。",
        guidance: "选取 1-2 个核心意象贯穿本章，意象的出现应与人物处境或主题呼应，避免滥用堆砌。",
        check_items: &["意象是否前后呼应而非孤立炫技", "象征是否服务于主题或人物"],
    },
    // ---- 网文节奏模板（F15，行业方法论工程化） ----
    Technique {
        id: "golden_three",
        name: "黄金三章（开篇）",
        category: "网文节奏",
        description: "开篇节奏模板：首章冲突种子、次章金手指亮相、三章核心矛盾锚定。",
        guidance: "若为开篇章节：300 字内抛出主角+困境+即时冲突；世界观边写边抖，不堆设定；三章内完成一次小高潮。",
        check_items: &["前 300 字是否出现主角与即时冲突", "设定是否随剧情展开而非集中灌输"],
    },
    Technique {
        id: "hook_matrix",
        name: "钩子矩阵",
        category: "网文节奏",
        description: "四维钩子：章末悬念（短期）、30 章级爆点（中期）、世界观谜题（长期）、角色暗线（隐性）。",
        guidance: "每章至少一个短期钩（章末悬念）；有节奏地铺设中期钩；与伏笔库联动形成长期钩与隐性钩。",
        check_items: &["章末是否有短期钩", "是否有中期/长期钩在推进而非停滞"],
    },
    Technique {
        id: "payoff_rhythm",
        name: "爽点节奏",
        category: "网文节奏",
        description: "情绪曲线模板：压抑积累 → 反击释放 → 新期待，先抑后扬。",
        guidance: "冲突先压制主角（对手占优），再让主角以既有设定与金手指合理破局；爽点后立刻埋新钩，避免连续压抑或连续爽。",
        check_items: &["是否先抑后扬而非直接给爽点", "爽点是否有新钩接续"],
    },
];

/// 全部技巧
pub fn all_techniques() -> Vec<Technique> {
    TECHNIQUES.to_vec()
}

/// 按 id 解析技巧；返回 (命中技巧, 未知 id 列表)
pub fn resolve(ids: &[String]) -> (Vec<Technique>, Vec<String>) {
    let mut hit = Vec::new();
    let mut unknown = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            continue;
        }
        match TECHNIQUES.iter().find(|t| t.id == id) {
            Some(t) => hit.push(t.clone()),
            None => unknown.push(id.to_string()),
        }
    }
    (hit, unknown)
}

/// 把选中技巧的生成指导汇总为提示词段落
pub fn guidance_block(ids: &[String]) -> Option<String> {
    let (hit, _unknown) = resolve(ids);
    if hit.is_empty() {
        return None;
    }
    let lines = hit
        .iter()
        .map(|t| format!("- {}（{}）：{}", t.name, t.category, t.guidance))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("## 本章写作技巧（用户指定，请落实）\n{lines}"))
}

/// 全部技巧的审校检查项（供 AI 审校使用）
pub fn check_items_for(ids: &[String]) -> Vec<String> {
    let (hit, _) = resolve(ids);
    hit.iter()
        .flat_map(|t| t.check_items.iter().map(|c| c.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_techniques_has_known_ids() {
        let all = all_techniques();
        assert!(all.len() >= 9, "内置技巧应不少于 9 条: {}", all.len());
        for t in &all {
            assert!(!t.id.is_empty());
            assert!(!t.guidance.is_empty());
            assert!(!t.check_items.is_empty(), "{} 应有检查项", t.name);
        }
    }

    #[test]
    fn resolve_known_and_unknown() {
        let ids = vec!["suspense".to_string(), "nope".to_string()];
        let (hit, unknown) = resolve(&ids);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].id, "suspense");
        assert_eq!(unknown, vec!["nope".to_string()]);
    }

    #[test]
    fn guidance_block_only_for_hits() {
        assert!(guidance_block(&["missing".to_string()]).is_none());
        let block = guidance_block(&["hook_matrix".to_string()]).unwrap();
        assert!(block.contains("钩子矩阵"));
        assert!(block.contains("本章写作技巧"));
    }

    #[test]
    fn check_items_collects_from_selected() {
        let items = check_items_for(&["suspense".to_string(), "payoff_rhythm".to_string()]);
        assert!(items.iter().any(|i| i.contains("悬念")));
        assert!(items.iter().any(|i| i.contains("先抑后扬")));
    }
}
