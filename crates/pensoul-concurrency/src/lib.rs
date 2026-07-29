pub mod conflict;
pub mod lock;
pub mod queue;
pub mod version;

pub use conflict::{ConflictError, ConflictResolver};
pub use lock::{ChapterVersion, ConcurrencyController, Operation, OperationStatus, OperationType};
pub use queue::OperationQueue;
pub use version::VersionManager;
