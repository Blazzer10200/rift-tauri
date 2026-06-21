//! Pending-request registry for the `mcp__rift__ask_user` interactive tool.
//!
//! Flow:
//!   1. Claude calls `mcp__rift__ask_user(questions: [...])`.
//!   2. The MCP child (`mcp_server::tool_ask_user`) generates a `request_id`,
//!      dials the loopback bridge with `op: "ask_user"`.
//!   3. The bridge handler (`bridge::ask_user_op`) registers a oneshot
//!      here keyed by `request_id`, emits `assistant://ask-user` to the
//!      frontend, and `await`s the oneshot (10-min timeout).
//!   4. The user picks an answer in the chat; the frontend invokes the
//!      `assistant_answer_ask_user` Tauri command which resolves the oneshot.
//!   5. The bridge handler returns the answer to the MCP child, which formats
//!      it as the tool_result Claude sees.
//!
//! The registry is `tauri::State`-managed — single instance for the app.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::oneshot;

pub struct AskUserRegistry {
    inner: Mutex<HashMap<String, oneshot::Sender<Value>>>,
}

/// RAII guard mirroring `PermissionGuard`: cancels the registry entry on drop.
/// Covers the case where the hosting bridge task is aborted mid-await (runtime
/// shutdown / explicit abort) — the await point is cancelled, so neither the
/// timeout nor error arm runs `cancel`, and the HashMap entry would otherwise
/// leak until app restart (RR7).
pub struct AskUserGuard {
    registry: Arc<AskUserRegistry>,
    request_id: String,
}

impl Drop for AskUserGuard {
    fn drop(&mut self) {
        self.registry.cancel(&self.request_id);
    }
}

impl AskUserRegistry {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// Register a pending request. The bridge dispatch task awaits the
    /// returned Receiver; `resolve` fires it from the Tauri command thread.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("AskUserRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.insert(request_id, tx);
        rx
    }

    /// `register` + an RAII `AskUserGuard` that cancels the entry on drop.
    /// Use this from any path that can be cancelled at an await point.
    pub fn register_guarded(
        self: &Arc<Self>,
        request_id: String,
    ) -> (oneshot::Receiver<Value>, AskUserGuard) {
        let rx = self.register(request_id.clone());
        (rx, AskUserGuard { registry: self.clone(), request_id })
    }

    /// Resolve a pending request. Returns true on success; false if the entry
    /// was already cancelled / never registered (stale UI re-submit, etc.).
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

    /// Drop a pending request without resolving — used after a timeout so the
    /// next answer submission for this id is a no-op rather than a panic on
    /// the channel.
    pub fn cancel(&self, request_id: &str) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("AskUserRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.remove(request_id);
    }
}

impl Default for AskUserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
