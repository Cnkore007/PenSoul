use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    UserEdit,
    AiGenerate,
    AiRevision,
    SystemImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Applied,
    Conflict,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterVersion {
    pub chapter_id: String,
    pub version: i32,
    pub checksum: String,
    pub last_modified_by: String,
    pub last_modified_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op_id: String,
    pub op_type: OperationType,
    pub chapter_id: String,
    pub content: String,
    pub expected_version: i32,
    pub timestamp: u64,
    pub status: OperationStatus,
    pub actual_version: Option<i32>,
}

pub struct ConcurrencyController {
    versions: Mutex<HashMap<String, ChapterVersion>>,
    operation_log: Mutex<Vec<Operation>>,
}

impl Default for ConcurrencyController {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrencyController {
    pub fn new() -> Self {
        Self {
            versions: Mutex::new(HashMap::new()),
            operation_log: Mutex::new(Vec::new()),
        }
    }

    pub fn register_chapter(&self, chapter_id: &str, initial_content: &str) {
        let mut hasher = Hasher::new();
        hasher.update(initial_content.as_bytes());
        let checksum = hasher.finalize().to_hex().to_string();

        let version = ChapterVersion {
            chapter_id: chapter_id.to_string(),
            version: 1,
            checksum,
            last_modified_by: "system".to_string(),
            last_modified_at: now_millis(),
        };

        self.versions
            .lock()
            .unwrap()
            .insert(chapter_id.to_string(), version);
    }

    pub fn get_version(&self, chapter_id: &str) -> i32 {
        let versions = self.versions.lock().unwrap();
        versions.get(chapter_id).map(|v| v.version).unwrap_or(-1)
    }

    pub fn submit_operation(&self, mut op: Operation) -> Operation {
        let mut versions = self.versions.lock().unwrap();

        let chapter_version = match versions.get_mut(&op.chapter_id) {
            Some(cv) => cv,
            None => {
                op.status = OperationStatus::Rejected;
                op.actual_version = None;
                self.operation_log.lock().unwrap().push(op.clone());
                return op;
            }
        };

        if chapter_version.version == op.expected_version {
            // Version matches — apply
            let mut hasher = Hasher::new();
            hasher.update(op.content.as_bytes());
            let checksum = hasher.finalize().to_hex().to_string();

            chapter_version.version += 1;
            chapter_version.checksum = checksum;
            chapter_version.last_modified_by = format!("{:?}", op.op_type);
            chapter_version.last_modified_at = op.timestamp;

            op.status = OperationStatus::Applied;
            op.actual_version = Some(chapter_version.version);
        } else {
            // Version mismatch — conflict
            op.status = OperationStatus::Conflict;
            op.actual_version = Some(chapter_version.version);
        }

        drop(versions);
        self.operation_log.lock().unwrap().push(op.clone());
        op
    }

    pub fn get_chapter_lock(&self, chapter_id: &str) -> Option<ChapterVersion> {
        let versions = self.versions.lock().unwrap();
        versions.get(chapter_id).cloned()
    }

    pub fn get_pending_ops(&self) -> Vec<Operation> {
        let log = self.operation_log.lock().unwrap();
        log.iter()
            .filter(|op| op.status == OperationStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn get_operation_log(&self) -> Vec<Operation> {
        self.operation_log.lock().unwrap().clone()
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: &str, chapter: &str, expected_ver: i32) -> Operation {
        Operation {
            op_id: id.to_string(),
            op_type: OperationType::UserEdit,
            chapter_id: chapter.to_string(),
            content: format!("content-{}", id),
            expected_version: expected_ver,
            timestamp: 1000,
            status: OperationStatus::Pending,
            actual_version: None,
        }
    }

    // 1. Version match → Applied
    #[test]
    fn test_version_match_applied() {
        let ctrl = ConcurrencyController::new();
        ctrl.register_chapter("ch1", "initial");
        let result = ctrl.submit_operation(op("op1", "ch1", 1));
        assert_eq!(result.status, OperationStatus::Applied);
        assert_eq!(result.actual_version, Some(2));
    }

    // 2. Version mismatch → Conflict
    #[test]
    fn test_version_mismatch_conflict() {
        let ctrl = ConcurrencyController::new();
        ctrl.register_chapter("ch1", "initial");
        ctrl.submit_operation(op("op1", "ch1", 1));
        let result = ctrl.submit_operation(op("op2", "ch1", 1)); // stale expected
        assert_eq!(result.status, OperationStatus::Conflict);
        assert_eq!(result.actual_version, Some(2));
    }

    // 3. Operation log completeness
    #[test]
    fn test_operation_log完整性() {
        let ctrl = ConcurrencyController::new();
        ctrl.register_chapter("ch1", "init");
        ctrl.submit_operation(op("op1", "ch1", 1));
        ctrl.submit_operation(op("op2", "ch1", 1)); // conflict
        let log = ctrl.get_operation_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].op_id, "op1");
        assert_eq!(log[1].op_id, "op2");
    }

    // 4. Version increment
    #[test]
    fn test_version_increments() {
        let ctrl = ConcurrencyController::new();
        ctrl.register_chapter("ch1", "init");
        assert_eq!(ctrl.get_version("ch1"), 1);
        ctrl.submit_operation(op("op1", "ch1", 1));
        assert_eq!(ctrl.get_version("ch1"), 2);
        ctrl.submit_operation(op("op2", "ch1", 2));
        assert_eq!(ctrl.get_version("ch1"), 3);
    }

    // 5. Concurrent conflict detection (two ops expecting same version)
    #[test]
    fn test_concurrent_conflict_detection() {
        let ctrl = ConcurrencyController::new();
        ctrl.register_chapter("ch1", "init");
        let r1 = ctrl.submit_operation(op("op1", "ch1", 1));
        let r2 = ctrl.submit_operation(op("op2", "ch1", 1));
        assert_eq!(r1.status, OperationStatus::Applied);
        assert_eq!(r2.status, OperationStatus::Conflict);
    }

    // 6. Unregistered chapter → Rejected
    #[test]
    fn test_unregistered_chapter_rejected() {
        let ctrl = ConcurrencyController::new();
        let result = ctrl.submit_operation(op("op1", "unknown", 1));
        assert_eq!(result.status, OperationStatus::Rejected);
        assert!(result.actual_version.is_none());
    }
}
