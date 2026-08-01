/// Agent 讨论配置
use serde::Deserialize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentDiscussionConfig {
    /// Agent 唯一 ID
    pub id: String,
    /// Agent 显示名称
    pub name: String,
    /// 使用的模型 ID
    pub model: String,
    /// 评审提示词
    pub prompt: String,
    /// 评审维度名称
    pub perspective: String,
    /// 是否启用
    pub enabled: bool,
    /// 来源专家 ID（从专家库添加时记录，用于去重）
    #[serde(default)]
    pub expert_id: Option<String>,
    /// 专家蒸馏技能文件路径（Experts/<名字>-expert/SKILL.md），
    /// 讨论时加载为系统提示词；预置/自定义 Agent 为 None
    #[serde(default)]
    pub skill_path: Option<String>,
}

// ── 讨论记录 ──
//
// 最近一次多 Agent 讨论的完整结果（发言 + 结构化成果），
// 随项目持久化，切换页面/重启客户端后不丢失。
// 这些类型同时是 LLM 成果提炼的解析目标，因此反序列化做了容错
// （强度容忍字符串、特质容忍多种结构）。

/// 一轮发言记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentTurn {
    pub agent_id: String,
    pub agent_name: String,
    #[serde(default)]
    pub perspective: String,
    /// 1=立论 2=交锋
    pub round: u8,
    #[serde(default)]
    pub content: String,
}

/// 讨论成果中的地点/设定规则条目
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NamedDesc {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// 讨论成果中的时间线条目
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TimelineItem {
    pub story_time: String,
    #[serde(default)]
    pub description: String,
}

/// 讨论成果中的人物关系
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RelationItem {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    #[serde(default = "default_strength", deserialize_with = "de_f32_lenient")]
    pub strength: f32,
}

fn default_strength() -> f32 {
    0.5
}

/// 讨论成果中的人物条目
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CharacterItem {
    pub name: String,
    #[serde(default, deserialize_with = "de_trait_pairs")]
    pub personality_traits: Vec<(String, f32)>,
    #[serde(default)]
    pub current_mood: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub relationships: Vec<RelationItem>,
}

/// 讨论成果中的情节节点（确认后写入大纲）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OutlineBeat {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub chapter_hint: String,
}

/// 结构化讨论成果
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DiscussionSynthesis {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub locations: Vec<NamedDesc>,
    #[serde(default)]
    pub timeline_events: Vec<TimelineItem>,
    #[serde(default)]
    pub setting_rules: Vec<NamedDesc>,
    #[serde(default)]
    pub characters: Vec<CharacterItem>,
    #[serde(default)]
    pub outline_beats: Vec<OutlineBeat>,
}

/// 一次讨论的完整记录（全部发言 + 提炼成果）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DiscussionRecord {
    #[serde(default)]
    pub turns: Vec<AgentTurn>,
    #[serde(default)]
    pub synthesis: DiscussionSynthesis,
}

/// 萌芽数据 — 核心想法 + 讨论 Agent 配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SproutData {
    /// 用户对故事想法的自由描述
    pub idea_description: String,
    /// 讨论 Agent 配置列表
    pub agents: Vec<AgentDiscussionConfig>,
    /// 预置 Agent 是否已被移除/替换（true 时不再回退到预置）
    #[serde(default)]
    pub presets_dismissed: bool,
    /// 最近一次讨论的结果（切换页面/重启后仍可查看）
    #[serde(default)]
    pub last_discussion: Option<DiscussionRecord>,
}

impl SproutData {
    pub fn new() -> Self {
        Self {
            idea_description: String::new(),
            agents: Vec::new(),
            presets_dismissed: false,
            last_discussion: None,
        }
    }
}

impl Default for SproutData {
    fn default() -> Self {
        Self::new()
    }
}

// ── 容错反序列化辅助（LLM 输出的强度/特质字段形态多变）──

/// 容错反序列化 f32：容忍数字字符串（如 `"0.7"`），无法解析时按 0.5
fn de_f32_lenient<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(lenient_f64(&v).unwrap_or(0.5) as f32)
}

/// 容错反序列化 `Vec<(String, f32)>`（人物特质列表）
///
/// 容忍模型输出的多种变体：
/// - `[["冷静", 0.8]]`         标准二元组数组
/// - `[{"name": "冷静", "strength": 0.8}]` 对象数组（也认 trait/key/value/score 键名）
/// - `{"冷静": 0.8}`           映射
/// - `["冷静"]`                纯字符串（强度按 0.5）
fn de_trait_pairs<'de, D>(deserializer: D) -> Result<Vec<(String, f32)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    let v = Value::deserialize(deserializer)?;
    let mut out = Vec::new();
    match v {
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::Array(pair) if !pair.is_empty() => {
                        let name = pair[0].as_str().unwrap_or_default().to_string();
                        let score = pair.get(1).and_then(lenient_f64).unwrap_or(0.5);
                        if !name.is_empty() {
                            out.push((name, score as f32));
                        }
                    }
                    Value::Object(m) => {
                        let name = ["name", "trait", "key"]
                            .iter()
                            .find_map(|k| m.get(*k).and_then(Value::as_str))
                            .unwrap_or_default()
                            .to_string();
                        let score = ["strength", "value", "score"]
                            .iter()
                            .find_map(|k| m.get(*k).and_then(lenient_f64))
                            .unwrap_or(0.5);
                        if !name.is_empty() {
                            out.push((name, score as f32));
                        }
                    }
                    Value::String(s) if !s.is_empty() => out.push((s, 0.5)),
                    _ => {}
                }
            }
        }
        Value::Object(m) => {
            for (k, v) in m {
                out.push((k, lenient_f64(&v).unwrap_or(0.5) as f32));
            }
        }
        _ => {}
    }
    Ok(out)
}

/// 从 JSON 值中尽力提取 f64（数字或数字字符串）
fn lenient_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_empty() {
        let s = SproutData::new();
        assert!(s.idea_description.is_empty());
        assert!(s.agents.is_empty());
        assert!(s.last_discussion.is_none());
    }

    #[test]
    fn test_default_matches_new() {
        let a = SproutData::new();
        let b = SproutData::default();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
    }

    #[test]
    fn test_sprout_serde_round_trip_with_agents() {
        let mut s = SproutData::new();
        s.idea_description = "一个关于时间循环的故事".to_string();
        s.agents.push(AgentDiscussionConfig {
            id: "agent-1".to_string(),
            name: "逻辑评审".to_string(),
            model: "kimi".to_string(),
            prompt: "检查逻辑漏洞".to_string(),
            perspective: "逻辑".to_string(),
            enabled: true,
            expert_id: Some("expert-9".to_string()),
            skill_path: Some("/ Experts/鲁迅-expert/SKILL.md".to_string()),
        });
        let json = serde_json::to_string(&s).unwrap();
        let back: SproutData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.idea_description, "一个关于时间循环的故事");
        assert_eq!(back.agents.len(), 1);
        assert!(back.agents[0].enabled);
        assert_eq!(back.agents[0].perspective, "逻辑");
        assert_eq!(
            back.agents[0].skill_path.as_deref(),
            Some("/ Experts/鲁迅-expert/SKILL.md")
        );
        assert_eq!(back.agents[0].expert_id.as_deref(), Some("expert-9"));
        assert!(!back.presets_dismissed);
    }

    #[test]
    fn test_sprout_compat_with_old_json_missing_new_fields() {
        // 旧版项目 JSON 没有 skill_path / expert_id / presets_dismissed / last_discussion，应能正常加载
        let json = r#"{
            "idea_description": "旧项目",
            "agents": [{
                "id": "agent-1",
                "name": "忘语",
                "model": "deepseek-v4-flash",
                "prompt": "评审",
                "perspective": "叙事",
                "enabled": true
            }]
        }"#;
        let s: SproutData = serde_json::from_str(json).unwrap();
        assert_eq!(s.agents.len(), 1);
        assert!(s.agents[0].skill_path.is_none());
        assert!(s.agents[0].expert_id.is_none());
        assert!(!s.presets_dismissed);
        assert!(s.last_discussion.is_none());
    }

    #[test]
    fn test_last_discussion_round_trip() {
        let mut s = SproutData::new();
        s.last_discussion = Some(DiscussionRecord {
            turns: vec![AgentTurn {
                agent_id: "agent-1".to_string(),
                agent_name: "鲁迅".to_string(),
                perspective: "批判".to_string(),
                round: 1,
                content: "立论内容".to_string(),
            }],
            synthesis: DiscussionSynthesis {
                summary: "共识总结".to_string(),
                locations: vec![NamedDesc {
                    name: "咸亨酒店".to_string(),
                    description: "小镇酒馆".to_string(),
                }],
                timeline_events: vec![TimelineItem {
                    story_time: "清末".to_string(),
                    description: "故事开端".to_string(),
                }],
                setting_rules: vec![NamedDesc {
                    name: "科举制度".to_string(),
                    description: "束缚读书人".to_string(),
                }],
                characters: vec![CharacterItem {
                    name: "孔乙己".to_string(),
                    personality_traits: vec![("迂腐".to_string(), 0.8)],
                    current_mood: "落魄".to_string(),
                    description: "站着喝酒的读书人".to_string(),
                    relationships: vec![RelationItem {
                        from: "孔乙己".to_string(),
                        to: "掌柜".to_string(),
                        relation_type: "主顾".to_string(),
                        strength: 0.4,
                    }],
                }],
                outline_beats: vec![OutlineBeat {
                    title: "登场".to_string(),
                    description: "引出主角".to_string(),
                    chapter_hint: "第1章".to_string(),
                }],
            },
        });
        let json = serde_json::to_string(&s).unwrap();
        let back: SproutData = serde_json::from_str(&json).unwrap();
        let rec = back.last_discussion.expect("讨论记录应保留");
        assert_eq!(rec.turns.len(), 1);
        assert_eq!(rec.turns[0].agent_name, "鲁迅");
        assert_eq!(rec.synthesis.characters.len(), 1);
        assert_eq!(
            rec.synthesis.characters[0].personality_traits,
            vec![("迂腐".to_string(), 0.8)]
        );
        assert_eq!(rec.synthesis.outline_beats[0].chapter_hint, "第1章");
    }

    #[test]
    fn test_synthesis_lenient_deserialize() {
        // LLM 产物：强度给成字符串、特质给成对象/映射，都应容错解析
        let json = r#"{
            "summary": "s",
            "characters": [{
                "name": "甲",
                "personality_traits": [{"name": "果断", "strength": "0.7"}],
                "relationships": [{"from": "甲", "to": "乙", "relation_type": "对手", "strength": "0.6"}]
            }]
        }"#;
        let s: DiscussionSynthesis = serde_json::from_str(json).unwrap();
        assert_eq!(
            s.characters[0].personality_traits,
            vec![("果断".to_string(), 0.7)]
        );
        assert!((s.characters[0].relationships[0].strength - 0.6).abs() < 1e-6);
    }
}
