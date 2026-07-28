use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConflictError {
    #[error("merge failed: {0}")]
    MergeFailed(String),
}

pub struct ConflictResolver;

impl ConflictResolver {
    /// Two operations conflict if they target the same chapter.
    pub fn detect_conflict(
        op_a_chapter: &str,
        op_b_chapter: &str,
    ) -> bool {
        op_a_chapter == op_b_chapter
    }

    /// Merge two content strings by concatenation.
    pub fn merge(content_a: &str, content_b: &str) -> Result<String, ConflictError> {
        if content_a.is_empty() && content_b.is_empty() {
            return Err(ConflictError::MergeFailed(
                "both contents are empty".to_string(),
            ));
        }
        Ok(format!("{}\n{}", content_a, content_b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_conflict_same_chapter() {
        assert!(ConflictResolver::detect_conflict("ch1", "ch1"));
    }

    #[test]
    fn test_detect_conflict_different_chapter() {
        assert!(!ConflictResolver::detect_conflict("ch1", "ch2"));
    }

    #[test]
    fn test_merge() {
        let result = ConflictResolver::merge("hello", "world").unwrap();
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn test_merge_both_empty() {
        let result = ConflictResolver::merge("", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_one_empty() {
        let result = ConflictResolver::merge("hello", "").unwrap();
        assert_eq!(result, "hello\n");
    }
}
