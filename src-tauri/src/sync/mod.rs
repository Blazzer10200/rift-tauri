//! Phase 1b sync surface: drift detection + edit trail. AutoSync (1c) lands here too.

pub mod auto_sync;
pub mod drift_scanner;
pub mod edit_trail;
pub mod ignore;
pub mod lock_presence;

pub use auto_sync::{
    AutoSyncEngine, AutoSyncState, AutoSyncStatus, ActivityRow, ActivityKind,
    ConflictRecord, ConflictResolution, FolderSpec,
};
pub use drift_scanner::{DriftBucket, DriftEntry, DriftScanner, FolderTarget, ScanResult};
pub use edit_trail::EditTrail;
pub use lock_presence::{LockPresence, RemoteLock};
