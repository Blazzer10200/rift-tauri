//! Generic pending-request registry shared by `permission::PermissionRegistry`
//! and `ask_user::AskUserRegistry`.
//!
//! Both surfaces park a oneshot keyed by `request_id`, tag it with the
//! `session_id` that raised it (so Stop can cancel a whole session's pending
//! asks), and resolve it from a Tauri command thread. This type carries that
//! shape; each caller keeps its own type name (so the two surfaces never
//! alias request ids), payloads, emits, and timeout values.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::oneshot;

/// A parked request: the sender that resolves it, tagged with the
/// `session_id` that raised it so Stop can cancel a whole session's pending asks.
struct Pending {
    tx: oneshot::Sender<Value>,
    session_id: String,
}

pub struct PendingRegistry<T> {
    inner: Mutex<HashMap<String, Pending>>,
    /// Zero-sized marker so each caller (`PermissionRegistry`, `AskUserRegistry`)
    /// gets a distinct monomorphized type — request ids from one never alias
    /// the other even though the storage is shared.
    _marker: std::marker::PhantomData<T>,
}

/// RAII guard: cancels the registered entry on drop unless `resolve`/`cancel`
/// already removed it. Closes the leak where the hosting task is aborted while
/// awaiting the response — the future is dropped at the suspension point, so
/// an explicit `cancel` call never runs, but this guard's `Drop` does.
pub struct PendingGuard<T> {
    registry: Arc<PendingRegistry<T>>,
    request_id: String,
}

impl<T> Drop for PendingGuard<T> {
    fn drop(&mut self) {
        self.registry.cancel(&self.request_id);
    }
}

impl<T> PendingRegistry<T> {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()), _marker: std::marker::PhantomData }
    }

    /// Register a pending request. The caller awaits the returned Receiver;
    /// `resolve` fires it from the command thread that carries the answer.
    /// `session_id` tags the entry so `cancel_all_for_session` (the Stop path)
    /// can drop it.
    pub fn register(&self, request_id: String, session_id: String) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PendingRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.insert(request_id, Pending { tx, session_id });
        rx
    }

    /// Register + return an RAII guard that cancels the entry on drop. Use when
    /// the await may be cancelled out from under the caller (task abort), so the
    /// HashMap entry can't leak. Calling `disarm` is unnecessary — `resolve`/
    /// `cancel` already remove the entry, making the drop a no-op.
    pub fn register_guarded(
        self: &Arc<Self>,
        request_id: String,
        session_id: String,
    ) -> (oneshot::Receiver<Value>, PendingGuard<T>) {
        let rx = self.register(request_id.clone(), session_id);
        (rx, PendingGuard { registry: self.clone(), request_id })
    }

    /// Resolve a pending request. Returns true on success; false if the entry was
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

    /// Cancel every pending request raised by `session_id`. Dropping each
    /// `oneshot::Sender` makes the parked waiter's `rx.await` resolve `Err`
    /// immediately, unblocking the caller and clearing the UI. Returns how many
    /// entries were cancelled.
    pub fn cancel_all_for_session(&self, session_id: &str) -> usize {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PendingRegistry mutex poisoned — recovering"); p.into_inner() }
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

    /// Drop a pending request without resolving — used after a timeout / turn
    /// end so a later answer submission for this id is a no-op.
    pub fn cancel(&self, request_id: &str) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => { log::error!("PendingRegistry mutex poisoned — recovering"); p.into_inner() }
        };
        g.remove(request_id);
    }
}

impl<T> Default for PendingRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Marker;

    #[test]
    fn register_then_resolve_delivers_value() {
        let reg: PendingRegistry<Marker> = PendingRegistry::new();
        let mut rx = reg.register("req-1".into(), "sess-1".into());
        assert!(reg.resolve("req-1", json!({ "behavior": "allow" })));
        assert_eq!(rx.try_recv().unwrap(), json!({ "behavior": "allow" }));
    }

    #[test]
    fn cancel_then_resolve_is_false() {
        let reg: PendingRegistry<Marker> = PendingRegistry::new();
        let mut rx = reg.register("req-2".into(), "sess".into());
        reg.cancel("req-2");
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        assert!(!reg.resolve("req-2", json!({ "behavior": "allow" })));
    }

    #[test]
    fn guard_drop_cancels_entry() {
        let reg = Arc::new(PendingRegistry::<Marker>::new());
        let mut rx = {
            let (rx, _guard) = reg.register_guarded("req-3".into(), "sess".into());
            rx
            // _guard dropped here at scope end -> cancel("req-3")
        };
        assert!(matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
        assert!(!reg.resolve("req-3", json!({})), "entry must be gone after guard drop");
    }

    #[test]
    fn cancel_all_for_session_scopes_to_one_session() {
        let reg: PendingRegistry<Marker> = PendingRegistry::new();
        let a1 = reg.register("a1".into(), "sess-A".into());
        let a2 = reg.register("a2".into(), "sess-A".into());
        let b1 = reg.register("b1".into(), "sess-B".into());
        assert_eq!(reg.cancel_all_for_session("sess-A"), 2);
        assert!(a1.blocking_recv().is_err());
        assert!(a2.blocking_recv().is_err());
        assert!(reg.resolve("b1", json!({ "behavior": "allow" })));
        assert_eq!(b1.blocking_recv().unwrap(), json!({ "behavior": "allow" }));
        assert_eq!(reg.cancel_all_for_session("sess-A"), 0);
    }
}
