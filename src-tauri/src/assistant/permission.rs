//! Pending-request registry for the CLI's `can_use_tool` permission prompts.
//!
//! Flow (prompting modes — default / acceptEdits / plan):
//!   1. `assistant_send` spawns the CLI with `--permission-prompt-tool stdio`,
//!      so per-action permission asks ride the stream-json control channel.
//!   2. When the model wants a gated tool, the CLI emits a
//!      `control_request { subtype: "can_use_tool", tool_use_id, .. }` on
//!      stdout.
//!   3. The stdout reader registers a oneshot here keyed by `request_id`,
//!      emits `assistant://permission-request` to the frontend, and `await`s.
//!   4. The user clicks Allow / Deny on the tool chip; the frontend invokes
//!      `assistant_answer_permission`, which resolves the oneshot with the
//!      decision payload (`{ behavior: "allow", updatedInput }` or
//!      `{ behavior: "deny", message }`).
//!   5. The reader writes that decision back as a `control_response` on the
//!      child's stdin, unblocking the CLI's tool execution.
//!
//! Mirrors `ask_user::AskUserRegistry` — same single-instance `tauri::State`
//! oneshot pattern, distinct type so the two surfaces never alias request ids.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::oneshot;

pub struct PermissionRegistry {
    inner: Mutex<HashMap<String, oneshot::Sender<Value>>>,
}

impl PermissionRegistry {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// Register a pending permission ask. The stdout reader awaits the returned
    /// Receiver; `resolve` fires it from the `assistant_answer_permission`
    /// command thread.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PermissionRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.insert(request_id, tx);
        rx
    }

    /// Resolve a pending ask. Returns true on success; false if the entry was
    /// already cancelled / never registered (stale UI re-submit, turn ended).
    pub fn resolve(&self, request_id: &str, value: Value) -> bool {
        let tx = match self.inner.lock() {
            Ok(mut g) => g.remove(request_id),
            Err(_) => return false,
        };
        match tx {
            Some(tx) => tx.send(value).is_ok(),
            None => false,
        }
    }

    /// Drop a pending ask without resolving — used after a timeout / turn end
    /// so a later answer submission for this id is a no-op.
    pub fn cancel(&self, request_id: &str) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PermissionRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.remove(request_id);
    }
}

impl Default for PermissionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
