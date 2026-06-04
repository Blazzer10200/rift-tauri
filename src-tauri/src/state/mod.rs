//! Rift v14 — persistent state caches.
//!
//! Phase 1a port from C# Services/State/* + Services/Sync/SyncSnapshot.cs.
//! File formats stay byte-compatible w/ the WPF side so the same ~/.rift/*.json
//! files can be read by either app during the migration window.
//!
//! EditTrail (~/.rift-trail.jsonl on the remote) lives behind SftpClient and
//! lands in Phase 1b.

pub mod paths;
