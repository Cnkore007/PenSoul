pub mod conflict;
pub mod lock;
pub mod version;

pub use conflict::{ConflictError, ConflictResolver};
pub use lock::{ChapterVersion, ConcurrencyController, Operation, OperationStatus, OperationType};
pub use version::VersionManager;
