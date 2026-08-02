pub mod aesthetic;
pub mod chapter;
pub mod character;
pub mod concept;
pub mod error;
pub mod id;
pub mod narrative;
pub mod ontology;
pub mod settings;
pub mod sprout;
pub mod workflow;
pub mod world;

pub use crate::aesthetic::*;
pub use crate::chapter::*;
pub use crate::character::*;
pub use crate::concept::CoreConcept;
pub use crate::error::{PensoulError, Result};
pub use crate::id::*;
pub use crate::narrative::*;
pub use crate::ontology::NovelOntology;
pub use crate::settings::ProjectSettings;
pub use crate::sprout::{
    AgentDiscussionConfig, AgentTurn, CharacterItem, Disagreement, DisagreeSide,
    DiscussionRecord, DiscussionSynthesis, NamedDesc, OutlineBeat, RelationItem, SproutData,
    TimelineItem,
};
pub use crate::narrative::EditSample;
pub use crate::workflow::*;
pub use crate::world::*;

/// 专家 — 蒸馏自著名人物的认知框架
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Expert {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_persona: String,
    pub model_id: String,
    pub perspective: String,
    pub default_prompt: String,
    pub created_at: String,
    pub skill_path: Option<String>,
    pub skill_summary: Option<String>,
}

/// 专家列表
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpertList {
    pub experts: Vec<Expert>,
}
