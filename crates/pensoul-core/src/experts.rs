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
