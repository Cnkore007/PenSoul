/// Layer 2 角色层类型定义
use crate::id::CharacterId;

/// 角色层
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterLayer {
    /// 角色列表
    pub characters: Vec<Character>,
    /// 角色关系
    pub relationships: Vec<Relationship>,
}

/// 角色
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Character {
    /// 角色 ID
    pub id: CharacterId,
    /// 角色名称
    pub name: String,
    /// 核心性格
    pub core_personality: PersonalityVector,
    /// 当前情绪
    pub current_mood: Emotion,
    /// 当前位置
    pub current_location: String,
    /// 当前知识
    pub current_knowledge: KnowledgeSet,
    /// 状态历史
    pub state_history: Vec<StateTransition>,
    /// 转换规则
    pub transition_rules: Vec<TransitionRule>,
    /// 对话风格
    pub dialogue_style: DialogueStyle,
    /// 成长曲线
    pub growth_curve: Vec<GrowthPoint>,
    /// 知识库
    pub knowledge_base: CharacterKnowledgeBase,
    /// 批注（实体级或字段级）
    #[serde(default)]
    pub annotations: Vec<crate::chapter::ChapterAnnotation>,
}

/// 性格向量
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersonalityVector {
    /// 性格特质列表 (特质名, 强度)
    pub traits: Vec<(String, f32)>,
}

/// 情绪
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Emotion {
    /// 主要情绪
    pub primary: String,
    /// 情绪强度
    pub intensity: f32,
    /// 次要情绪
    pub secondary: String,
}

/// 知识集
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeSet {
    /// 知识项列表
    pub facts: Vec<KnowledgeItem>,
}

/// 知识项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeItem {
    /// 事实 ID
    pub fact_id: String,
    /// 知识内容
    pub content: String,
    /// 知识来源
    pub source: KnowledgeSource,
    /// 可靠性
    pub reliability: f32,
}

/// 知识来源
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum KnowledgeSource {
    /// 亲眼所见
    Observed,
    /// 他人告知
    Told {
        /// 告知者 ID
        from: CharacterId,
    },
    /// 推理得出
    Inferred,
    /// 回忆
    Remembered,
}

/// 状态转换
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateTransition {
    /// 源状态
    pub from: String,
    /// 目标状态
    pub to: String,
    /// 触发事件
    pub trigger: String,
    /// 所属章节 ID
    #[serde(deserialize_with = "crate::id::flexible_id::deserialize_chapter_id")]
    pub chapter_id: crate::id::ChapterId,
    /// 故事时间
    pub story_time: String,
    /// 因果关系
    pub causality: String,
}

/// 转换规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransitionRule {
    /// 事件模式
    pub event_pattern: String,
    /// 源状态
    pub from_state: String,
    /// 目标状态
    pub to_state: String,
    /// 转换条件
    pub condition: String,
}

/// 对话风格
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DialogueStyle {
    /// 对话模式
    pub patterns: Vec<String>,
    /// 词汇水平
    pub vocabulary_level: String,
    /// 平均句子长度
    pub sentence_length_avg: f32,
    /// 口头禅
    pub catchphrases: Vec<String>,
}

/// 成长点
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrowthPoint {
    /// 所属章节 ID
    #[serde(deserialize_with = "crate::id::flexible_id::deserialize_chapter_id")]
    pub chapter_id: crate::id::ChapterId,
    /// 成长维度
    pub dimension: String,
    /// 成长值
    pub value: f32,
    /// 成长备注
    pub note: String,
}

/// 角色知识库
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterKnowledgeBase {
    /// 已知事实
    pub known_facts: Vec<KnowledgeItem>,
    /// 知识来源记录
    pub knowledge_sources: Vec<KnowledgeSourceRecord>,
    /// 衰减模型
    pub decay_model: DecayModel,
}

/// 知识来源记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeSourceRecord {
    /// 来源
    pub source: KnowledgeSource,
    /// 获取时间
    pub obtained_at: String,
    /// 可靠性
    pub reliability: f32,
}

/// 衰减模型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecayModel {
    /// 半衰期（章节）
    pub half_life_chapters: i32,
    /// 最小可靠性
    pub min_reliability: f32,
}

/// 角色关系
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relationship {
    /// 源角色 ID
    pub from: CharacterId,
    /// 目标角色 ID
    pub to: CharacterId,
    /// 关系类型
    pub relation_type: String,
    /// 关系强度
    pub strength: f32,
    /// 关系历史（缺省为空，兼容只携带当前关系的外部数据）
    #[serde(default)]
    pub history: Vec<RelationshipChange>,
}

/// 关系变更
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipChange {
    /// 所属章节 ID
    #[serde(deserialize_with = "crate::id::flexible_id::deserialize_chapter_id")]
    pub chapter_id: crate::id::ChapterId,
    /// 旧关系类型
    pub old_type: String,
    /// 新关系类型
    pub new_type: String,
    /// 变更原因
    pub reason: String,
}
