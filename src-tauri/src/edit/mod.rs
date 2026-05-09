// Phase 1j — port of Rift Project/Services/Edit/EditInPlace.cs.
//
// EditInPlaceManager orchestrates the "edit a remote file in your local
// editor, watch for saves, prompt-or-auto reupload" flow. Phase 4 wires the
// reupload prompt UI on top of this; for now the manager surfaces save
// detections via Tauri events (`edit://changed`) so the UI can react.

pub mod in_place;

pub use in_place::{EditInPlaceManager, WatchedFileInfo};
