/// PenSoul 预置 Agent 定义 — 6 个写作辅助智能体
use pensoul_core::AgentId;
use serde::{Deserialize, Serialize};

/// 预置 Agent 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// 一致性审查
    ConsistencyAuditor,
    /// 文风校准
    StyleAnalyzer,
    /// 伏笔追踪
    ForeshadowTracker,
    /// 对话打磨
    DialoguePolisher,
    /// 大纲规划
    PlotArchitect,
    /// 世界观构建
    WorldBuilder,
}

/// Agent 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent 类型
    pub agent_type: AgentType,
    /// Agent 唯一 ID
    pub agent_id: AgentId,
    /// 显示名称
    pub display_name: String,
    /// 功能描述
    pub description: String,
    /// 偏好模型
    pub model_preference: String,
    /// 允许使用的工具列表
    pub tools_allowed: Vec<String>,
    /// 信号通道字段
    pub signal_fields: Vec<String>,
    /// 系统提示词
    pub system_prompt: String,
}

impl AgentDefinition {
    /// 根据类型获取预置 Agent 定义
    pub fn preset(agent_type: AgentType) -> Self {
        match agent_type {
            AgentType::ConsistencyAuditor => Self {
                agent_type,
                agent_id: AgentId::new("consistency_auditor"),
                display_name: "一致性审查员".into(),
                description: "独立审查实体状态、时间线、角色行为一致性".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec![
                    "read_chapter".into(),
                    "read_character_state".into(),
                    "read_consistency_vector".into(),
                    "run_consistency_check".into(),
                ],
                signal_fields: vec!["pass".into(), "score".into(), "issues".into()],
                system_prompt: "你是一个严格的一致性审查专家。你的职责是检查小说中的实体状态、时间线和角色行为是否一致，找出矛盾之处并报告。".into(),
            },
            AgentType::StyleAnalyzer => Self {
                agent_type,
                agent_id: AgentId::new("style_analyzer"),
                display_name: "文风分析师".into(),
                description: "校准文风、反AI味检查".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_style_guide".into()],
                signal_fields: vec!["pass".into(), "style_score".into()],
                system_prompt: "你是一个专业的文风分析专家。你的职责是分析文本的文风特征，检查是否存在AI味，并给出改进建议。".into(),
            },
            AgentType::ForeshadowTracker => Self {
                agent_type,
                agent_id: AgentId::new("foreshadow_tracker"),
                display_name: "伏笔追踪员".into(),
                description: "追踪伏笔的埋设、推进和回收".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_foreshadows".into()],
                signal_fields: vec!["status_update".into(), "alerts".into()],
                system_prompt: "你是一个伏笔管理专家。你的职责是追踪所有伏笔的生命周期，确保伏笔被正确埋设、推进和回收。".into(),
            },
            AgentType::DialoguePolisher => Self {
                agent_type,
                agent_id: AgentId::new("dialogue_polisher"),
                display_name: "对话打磨师".into(),
                description: "打磨对话质量，确保角色语言个性化".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_chapter".into(), "read_character_style".into()],
                signal_fields: vec!["pass".into(), "quality_score".into()],
                system_prompt: "你是一个对话写作专家。你的职责是打磨对话质量，确保每个角色的语言风格符合其人设，对话自然流畅。".into(),
            },
            AgentType::PlotArchitect => Self {
                agent_type,
                agent_id: AgentId::new("plot_architect"),
                display_name: "大纲架构师".into(),
                description: "规划多层大纲、伏笔地图、角色弧线".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_memory".into(), "generate_outline".into()],
                signal_fields: vec!["outline_proposal".into()],
                system_prompt: "你是一个专业的小说大纲架构师。你的职责是规划多层大纲，设计伏笔地图，构建角色弧线。".into(),
            },
            AgentType::WorldBuilder => Self {
                agent_type,
                agent_id: AgentId::new("world_builder"),
                display_name: "世界观构建师".into(),
                description: "构建一致的世界观设定".into(),
                model_preference: "gpt-4o".into(),
                tools_allowed: vec!["read_world".into(), "generate_world_spec".into()],
                signal_fields: vec!["world_spec".into()],
                system_prompt: "你是一个专业的小说世界观构建师。你的职责是构建一致的世界观设定，包括空间、时间、规则和术语。".into(),
            },
        }
    }

    /// 获取所有预置 Agent 定义
    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::preset(AgentType::ConsistencyAuditor),
            Self::preset(AgentType::StyleAnalyzer),
            Self::preset(AgentType::ForeshadowTracker),
            Self::preset(AgentType::DialoguePolisher),
            Self::preset(AgentType::PlotArchitect),
            Self::preset(AgentType::WorldBuilder),
        ]
    }

    /// 检查工具是否被允许使用
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.tools_allowed.iter().any(|t| t == tool_name)
    }

    /// 检查信号字段是否有效
    pub fn has_signal_field(&self, field: &str) -> bool {
        self.signal_fields.iter().any(|f| f == field)
    }
}

impl Default for AgentDefinition {
    fn default() -> Self {
        Self::preset(AgentType::ConsistencyAuditor)
    }
}
