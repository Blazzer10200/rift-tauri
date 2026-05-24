//! Pending-request registry for the `mcp__rift__ask_user` interactive tool.
//!
//! Flow:
//!   1. Claude calls `mcp__rift__ask_user(questions: [...])`.
//!   2. The MCP child (`mcp_server::tool_ask_user`) generates a `request_id`,
//!      dials the loopback bridge with `op: "ask_user"`.
//!   3. The bridge handler (`remote_bridge::ask_user_op`) registers a oneshot
//!      here keyed by `request_id`, emits `assistant://ask-user` to the
//!      frontend, and `await`s the oneshot (10-min timeout).
//!   4. The user picks an answer in the chat; the frontend invokes the
//!      `assistant_answer_ask_user` Tauri command which resolves the oneshot.
//!   5. The bridge handler returns the answer to the MCP child, which formats
//!      it as the tool_result Claude sees.
//!
//! The registry is `tauri::State`-managed — single instance for the app.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::oneshot;

pub struct AskUserRegistry {
    inner: Mutex<HashMap<String, oneshot::Sender<Value>>>,
}

impl AskUserRegistry {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// Register a pending request. The bridge dispatch task awaits the
    /// returned Receiver; `resolve` fires it from the Tauri command thread.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut g) = self.inner.lock() {
            g.insert(request_id, tx);
        }
        rx
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
        if let Ok(mut g) = self.inner.lock() {
            g.remove(request_id);
        }
    }
}

impl Default for AskUserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
