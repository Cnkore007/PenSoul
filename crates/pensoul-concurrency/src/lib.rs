pub mod lock;
pub mod queue;
pub mod conflict;
pub mod version;

pub use lock::{
    ChapterVersion, ConcurrencyController, Operation, OperationStatus, OperationType,
};
pub use queue::OperationQueue;
pub use conflict::{ConflictError, ConflictResolver};
pub use version::VersionManager;
