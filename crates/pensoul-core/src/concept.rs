/// 核心概念 / 高概念 — 整部小说的"种子"
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreConcept {
    /// 高概念 / 核心想法（一句话概括）
    pub high_concept: String,
    /// 故事前提 / 冲突前提
    pub premise: String,
    /// 主角雏形
    pub protagonist_hint: String,
    /// 故事基调 / 风格
    pub tone: String,
    /// 核心冲突
    pub central_conflict: String,
    /// 灵感来源 / 创作缘由
    pub inspiration: String,
}

impl CoreConcept {
    pub fn new() -> Self {
        Self {
            high_concept: String::new(),
            premise: String::new(),
            protagonist_hint: String::new(),
            tone: String::new(),
            central_conflict: String::new(),
            inspiration: String::new(),
        }
    }
}

impl Default for CoreConcept {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_fields_empty() {
        let c = CoreConcept::new();
        assert!(c.high_concept.is_empty());
        assert!(c.premise.is_empty());
        assert!(c.protagonist_hint.is_empty());
        assert!(c.tone.is_empty());
        assert!(c.central_conflict.is_empty());
        assert!(c.inspiration.is_empty());
    }

    #[test]
    fn test_default_matches_new() {
        let a = CoreConcept::new();
        let b = CoreConcept::default();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
    }

    #[test]
    fn test_concept_serde_round_trip() {
        let mut c = CoreConcept::new();
        c.high_concept = "废柴逆袭".to_string();
        c.central_conflict = "人与天命".to_string();
        let json = serde_json::to_string(&c).unwrap();
        let back: CoreConcept = serde_json::from_str(&json).unwrap();
        assert_eq!(back.high_concept, "废柴逆袭");
        assert_eq!(back.central_conflict, "人与天命");
    }
}
