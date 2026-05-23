//! Assistant command surface — re-exports from `crate::assistant` so lib.rs's
//! `invoke_handler!` can reference everything under `commands::*` (#20).
//!
//! Wildcard re-export is required: `#[tauri::command]` expands to the fn PLUS
//! sibling items (`__cmd__<name>`, `__tauri_command_name_<name>`) that
//! `generate_handler!` needs at the resolved path. Explicit `pub use { ... }`
//! only forwards named items, so the macro siblings would be missing and
//! `invoke_handler!` would fail to compile.

pub use crate::assistant::*;
