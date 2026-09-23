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
//! Mirrors `permission::PermissionRegistry`; both share their register /
//! resolve / cancel mechanics through `pending::PendingRegistry`.

use super::pending::{PendingGuard, PendingRegistry};

/// Marker type distinguishing this registry's `PendingRegistry` instantiation
/// from `permission`'s — no runtime cost, just keeps the two types distinct.
pub struct AskUserMarker;

pub type AskUserRegistry = PendingRegistry<AskUserMarker>;

/// RAII guard mirroring `PermissionGuard`: cancels the registry entry on drop.
/// Covers the case where the hosting bridge task is aborted mid-await (runtime
/// shutdown / explicit abort) — the await point is cancelled, so neither the
/// timeout nor error arm runs `cancel`, and the HashMap entry would otherwise
/// leak until app restart (RR7).
pub type AskUserGuard = PendingGuard<AskUserMarker>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

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
        assert!(matches!(rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)));
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
        assert!(matches!(rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)));
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
        assert!(matches!(a.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)));
        assert!(matches!(b.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Closed)));
        // Session B is untouched — still resolvable.
        assert!(reg.resolve("c", json!({"ok": true})));
        assert_eq!(c.try_recv().unwrap()["ok"], true);
        // Cancelling a session with no entries returns 0.
        assert_eq!(reg.cancel_all_for_session("sess-A"), 0);
    }
}
