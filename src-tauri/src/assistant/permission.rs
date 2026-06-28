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
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::oneshot;

/// A parked permission ask: the sender that resolves it, tagged with the
/// `session_id` that raised it so Stop can cancel a whole session's pending asks.
struct Pending {
    tx: oneshot::Sender<Value>,
    session_id: String,
}

pub struct PermissionRegistry {
    inner: Mutex<HashMap<String, Pending>>,
}

/// RAII guard: cancels the registered entry on drop unless `resolve`/`cancel`
/// already removed it. Closes the leak where `stdout_task` is aborted while
/// awaiting the user's decision — the future is dropped at the suspension point,
/// so the explicit `cancel` call never runs, but this guard's `Drop` does.
pub struct PermissionGuard {
    registry: Arc<PermissionRegistry>,
    request_id: String,
}

impl Drop for PermissionGuard {
    fn drop(&mut self) {
        self.registry.cancel(&self.request_id);
    }
}

impl PermissionRegistry {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// Register a pending permission ask. The stdout reader awaits the returned
    /// Receiver; `resolve` fires it from the `assistant_answer_permission`
    /// command thread. `session_id` tags the entry so `cancel_all_for_session`
    /// (the Stop path) can drop it.
    pub fn register(&self, request_id: String, session_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PermissionRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.insert(request_id, Pending { tx, session_id });
        rx
    }

    /// Register + return an RAII guard that cancels the entry on drop. Use when
    /// the await may be cancelled out from under the caller (task abort), so the
    /// HashMap entry can't leak. Call `PermissionGuard::disarm` is unnecessary —
    /// `resolve`/`cancel` already remove the entry, making the drop a no-op.
    pub fn register_guarded(
        self: &Arc<Self>,
        request_id: String,
        session_id: String,
    ) -> (oneshot::Receiver<Value>, PermissionGuard) {
        let rx = self.register(request_id.clone(), session_id);
        (rx, PermissionGuard { registry: self.clone(), request_id })
    }

    /// Resolve a pending ask. Returns true on success; false if the entry was
    /// already cancelled / never registered (stale UI re-submit, turn ended).
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

    /// Cancel every pending permission ask raised by `session_id`. Dropping each
    /// `oneshot::Sender` makes the parked stdout-reader `rx.await` resolve `Err`
    /// immediately → that arm writes a `deny` control_response (or ends the turn)
    /// → the CLI child unblocks and the UI tool-chip clears. The Stop-path safety
    /// net: `assistant_stop` kills the warm child by PID, but if that PID was
    /// already cleared (eviction / prior-turn cleanup) the kill is a no-op and the
    /// permission await would otherwise park for the full 120s timeout, leaving the
    /// UI stuck. (mega-audit cont.228 — stop-permission-registry-not-cancelled.)
    /// Returns how many entries were cancelled.
    pub fn cancel_all_for_session(&self, session_id: &str) -> usize {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PermissionRegistry mutex poisoned — recovering"); p.into_inner() }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn register_then_resolve_delivers_value() {
        let reg = PermissionRegistry::new();
        let mut rx = reg.register("req-1".into(), "sess-1".into());
        assert!(reg.resolve("req-1", json!({ "behavior": "allow" })));
        assert_eq!(rx.try_recv().unwrap(), json!({ "behavior": "allow" }));
    }

    #[test]
    fn resolve_unknown_id_is_false() {
        let reg = PermissionRegistry::new();
        assert!(!reg.resolve("nope", json!({ "behavior": "deny" })));
    }

    #[test]
    fn double_resolve_second_is_false() {
        let reg = PermissionRegistry::new();
        let _rx = reg.register("req-2".into(), "sess".into());
        assert!(reg.resolve("req-2", json!(1)));
        // entry was removed on first resolve — second is a no-op.
        assert!(!reg.resolve("req-2", json!(2)));
    }

    #[test]
    fn cancel_then_resolve_is_false() {
        let reg = PermissionRegistry::new();
        let _rx = reg.register("req-3".into(), "sess".into());
        reg.cancel("req-3");
        assert!(!reg.resolve("req-3", json!({ "behavior": "allow" })));
    }

    #[test]
    fn resolve_after_receiver_dropped_is_false() {
        let reg = PermissionRegistry::new();
        let rx = reg.register("req-4".into(), "sess".into());
        drop(rx); // UI gone / turn ended before the answer lands
        assert!(!reg.resolve("req-4", json!({ "behavior": "allow" })));
    }

    #[test]
    fn distinct_ids_do_not_alias() {
        let reg = PermissionRegistry::new();
        let mut a = reg.register("a".into(), "sess".into());
        let mut b = reg.register("b".into(), "sess".into());
        assert!(reg.resolve("b", json!("B")));
        assert!(reg.resolve("a", json!("A")));
        assert_eq!(a.try_recv().unwrap(), json!("A"));
        assert_eq!(b.try_recv().unwrap(), json!("B"));
    }

    #[test]
    fn cancel_all_for_session_scopes_to_one_session_and_unblocks() {
        let reg = PermissionRegistry::new();
        let a1 = reg.register("a1".into(), "sess-A".into());
        let a2 = reg.register("a2".into(), "sess-A".into());
        let b1 = reg.register("b1".into(), "sess-B".into());
        // Cancels only sess-A's two entries.
        assert_eq!(reg.cancel_all_for_session("sess-A"), 2);
        // The cancelled receivers resolve Err (sender dropped) → the await unblocks.
        assert!(a1.blocking_recv().is_err());
        assert!(a2.blocking_recv().is_err());
        // sess-B is untouched and still resolvable.
        assert!(reg.resolve("b1", json!({ "behavior": "allow" })));
        assert_eq!(b1.blocking_recv().unwrap(), json!({ "behavior": "allow" }));
        // Re-cancelling a drained session is a no-op.
        assert_eq!(reg.cancel_all_for_session("sess-A"), 0);
    }

    #[test]
    fn reregister_same_id_overwrites_sender() {
        let reg = PermissionRegistry::new();
        let mut first = reg.register("dup".into(), "sess".into());
        let mut second = reg.register("dup".into(), "sess".into());
        // Only the latest sender is kept; the first can never be resolved.
        assert!(reg.resolve("dup", json!("second")));
        assert_eq!(second.try_recv().unwrap(), json!("second"));
        assert!(first.try_recv().is_err());
    }
}
