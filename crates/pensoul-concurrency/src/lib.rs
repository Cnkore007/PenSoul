pub mod lock;
pub mod version;

pub use lock::{ChapterVersion, ConcurrencyController, Operation, OperationStatus, OperationType};
pub use version::{ConflictError, ConflictResolver, VersionManager};
