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

/// A parked ask_user request: the resolver channel + the session that raised it
/// (so a Stop on that session can cancel it even when the PID-kill path misses).
struct Pending {
    tx: oneshot::Sender<Value>,
    session_id: String,
}

pub struct AskUserRegistry {
    inner: Mutex<HashMap<String, Pending>>,
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
    pub fn register(&self, request_id: String, session_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("AskUserRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.insert(request_id, Pending { tx, session_id });
        rx
    }

    /// `register` + an RAII `AskUserGuard` that cancels the entry on drop.
    /// Use this from any path that can be cancelled at an await point.
    pub fn register_guarded(
        self: &Arc<Self>,
        request_id: String,
        session_id: String,
    ) -> (oneshot::Receiver<Value>, AskUserGuard) {
        let rx = self.register(request_id.clone(), session_id);
        (rx, AskUserGuard { registry: self.clone(), request_id })
    }

    /// Resolve a pending request. Returns true on success; false if the entry
    /// was already cancelled / never registered (stale UI re-submit, etc.).
    pub fn resolve(&self, request_id: &str, value: Value) -> bool {
        let pending = match self.inner.lock() {
            Ok(mut g) => g.remove(request_id),
            Err(_) => return false,
        };
        match pending {
            Some(p) => p.tx.send(value).is_ok(),
            None => false,
        }
    }

    /// Cancel every pending ask_user raised by `session_id`. The dropped
    /// `oneshot::Sender` makes each parked bridge waiter's `rx` resolve `Err`
    /// immediately, so the MCP child unblocks and the UI spinner clears. This is
    /// the Stop-path safety net: `assistant_stop` kills the warm child by PID,
    /// but if that PID was already cleared (eviction / prior-turn cleanup) the
    /// kill is a no-op and the bridge oneshot would otherwise park for the full
    /// 600s timeout. Returns how many entries were cancelled.
    pub fn cancel_all_for_session(&self, session_id: &str) -> usize {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("AskUserRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        let ids: Vec<String> = g
            .iter()
            .filter(|(_, p)| p.session_id == session_id)
            .map(|(k, _)| k.clone())
            .collect();
        for id in &ids {
            g.remove(id);
        }
        ids.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn register_then_resolve_delivers_value() {
        let reg = AskUserRegistry::new();
        let mut rx = reg.register("r1".into(), "s1".into());
        assert!(reg.resolve("r1", json!({"answers": [{"question": "Q", "answer": "A"}]})));
        // The value arrived on the oneshot.
        let got = rx.try_recv().expect("value should be delivered");
        assert_eq!(got["answers"][0]["answer"], "A");
    }

    #[test]
    fn resolve_unknown_id_is_false_not_panic() {
        let reg = AskUserRegistry::new();
        // Stale UI re-submit / never-registered id → false, no panic.
        assert!(!reg.resolve("ghost", json!({})));
    }

    #[test]
    fn double_resolve_second_is_false() {
        let reg = AskUserRegistry::new();
        let _rx = reg.register("r1".into(), "s1".into());
        assert!(reg.resolve("r1", json!({"x": 1})));
        // Entry was removed on first resolve — second is a no-op false.
        assert!(!reg.resolve("r1", json!({"x": 2})));
    }

    #[test]
    fn cancel_unblocks_waiter_and_blocks_later_resolve() {
        let reg = AskUserRegistry::new();
        let mut rx = reg.register("r1".into(), "s1".into());
        reg.cancel("r1");
        // The Sender dropped → rx resolves Err (the bridge waiter unblocks).
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        // A late answer for a cancelled id is a no-op, never a panic.
        assert!(!reg.resolve("r1", json!({})));
    }

    #[test]
    fn guard_drop_cancels_entry_rr7() {
        // RR7: an aborted bridge task drops the guard without hitting the
        // timeout/error arm; the Drop must still purge the registry entry.
        let reg = Arc::new(AskUserRegistry::new());
        let mut rx = {
            let (rx, _guard) = reg.register_guarded("r1".into(), "s1".into());
            rx
            // _guard dropped here at scope end → cancel("r1")
        };
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        assert!(!reg.resolve("r1", json!({})), "entry must be gone after guard drop");
    }

    #[test]
    fn cancel_all_for_session_scopes_to_one_session() {
        let reg = AskUserRegistry::new();
        let mut a = reg.register("a".into(), "sess-A".into());
        let mut b = reg.register("b".into(), "sess-A".into());
        let mut c = reg.register("c".into(), "sess-B".into());
        // Cancel only session A's two entries.
        assert_eq!(reg.cancel_all_for_session("sess-A"), 2);
        assert!(matches!(a.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        assert!(matches!(b.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        // Session B is untouched — still resolvable.
        assert!(reg.resolve("c", json!({"ok": true})));
        assert_eq!(c.try_recv().unwrap()["ok"], true);
        // Cancelling a session with no entries returns 0.
        assert_eq!(reg.cancel_all_for_session("sess-A"), 0);
    }
}
