use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConflictError {
    #[error("merge failed: {0}")]
    MergeFailed(String),
}

pub struct ConflictResolver;

impl ConflictResolver {
    /// Two operations conflict if they target the same chapter.
    pub fn detect_conflict(op_a_chapter: &str, op_b_chapter: &str) -> bool {
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

pub struct VersionManager {
    versions: HashMap<String, i32>,
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
        }
    }

    /// Return the next version number for a chapter, creating it at 1 if absent.
    pub fn next_version(&mut self, chapter_id: &str) -> i32 {
        let entry = self.versions.entry(chapter_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Return the current version, or 0 if the chapter has not been registered.
    pub fn current_version(&self, chapter_id: &str) -> i32 {
        self.versions.get(chapter_id).copied().unwrap_or(0)
    }

    /// Reset a chapter's version to 0.
    pub fn reset(&mut self, chapter_id: &str) {
        self.versions.insert(chapter_id.to_string(), 0);
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

    #[test]
    fn test_next_version_increments() {
        let mut vm = VersionManager::new();
        assert_eq!(vm.next_version("ch1"), 1);
        assert_eq!(vm.next_version("ch1"), 2);
        assert_eq!(vm.next_version("ch1"), 3);
    }

    #[test]
    fn test_current_version_unknown() {
        let vm = VersionManager::new();
        assert_eq!(vm.current_version("ch1"), 0);
    }

    #[test]
    fn test_current_version_after_next() {
        let mut vm = VersionManager::new();
        vm.next_version("ch1");
        vm.next_version("ch1");
        assert_eq!(vm.current_version("ch1"), 2);
    }

    #[test]
    fn test_reset() {
        let mut vm = VersionManager::new();
        vm.next_version("ch1");
        vm.next_version("ch1");
        vm.reset("ch1");
        assert_eq!(vm.current_version("ch1"), 0);
        assert_eq!(vm.next_version("ch1"), 1);
    }
}
