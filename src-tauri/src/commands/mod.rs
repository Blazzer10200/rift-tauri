//! Tauri command surface — split per domain (#20).
//!
//! lib.rs's `invoke_handler!` references these via `commands::*`. Each domain
//! file owns its #[tauri::command] fns + private helpers. Cross-domain helpers
//! (path-traversal guard, fingerprint pin guard, log-basename) live here.

pub mod assistant;
pub mod browser;
pub mod update;

pub use assistant::*;
pub use browser::*;
pub use update::*;

/// Open a path in VS Code. The opener plugin's `openWith` can't launch VS Code
/// on Windows (its CLI is `code.cmd`, which `Command::new("code")` won't resolve
/// without a shell) — so spawn through `cmd /C` on Windows, direct `code` elsewhere.
#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    // The Windows path runs through `cmd /C`, which re-parses metacharacters
    // *after* Rust's arg quoting — so `&`/`|`/`<`/`>`/`^`/`%`/`"` (and control
    // chars) could chain a second command. None are needed to open a file; reject
    // them rather than try to escape cmd.exe's quoting (F19).
    #[cfg(windows)]
    if path.bytes().any(|b| matches!(b, b'&' | b'|' | b'<' | b'>' | b'^' | b'%' | b'"') || b < 0x20) {
        return Err("path contains characters that can't be passed safely to the shell".into());
    }
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "code", &path]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = std::process::Command::new("code");
        c.arg(&path);
        c
    };
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't launch VS Code (is `code` on PATH?): {e}"))
}

/// B2 — AI Health turn-performance aggregate. Reads the persisted `turns.ndjson`
/// (p50/p90 latency, cache-hit rate, cost-by-day) off the async executor so the
/// file parse never stalls a Tauri worker. Returns a zero-filled aggregate when
/// no turns have been recorded yet (first launch) rather than erroring.
#[tauri::command]
pub async fn query_turn_perf() -> Result<crate::diagnostics::perf::TurnPerfStats, String> {
    tokio::task::spawn_blocking(crate::diagnostics::perf::query_turn_perf_sync)
        .await
        .map_err(|e| format!("query_turn_perf: {e}"))
}

/// Phase 4 — snapshot of the process-global metrics registry (counters +
/// timing histograms recorded via `metric!` / `timed!`). In-memory, session-
/// scoped; cheap synchronous read under a mutex, no file I/O.
#[tauri::command]
pub fn query_metrics() -> crate::diagnostics::metrics::MetricsSnapshot {
    crate::diagnostics::metrics::snapshot()
}

/// #37 Route A — spawn a second native window so a session can live on a
/// separate monitor. Same app URL, unique `window-<n>` label (matched by the
/// `secondary-window` capability glob). Each window boots its own store and
/// namespaces its persisted tab state by label (see `persistence.ts`).
// Must be `async`: on Windows, `WebviewWindowBuilder::build()` deadlocks WebView2
// when called from a *synchronous* command — the window opens but stays at
// about:blank (tauri-apps/tauri#13963). An async command runs off the main
// thread, so the build dispatches cleanly.
#[tauri::command]
pub async fn open_new_window(app: tauri::AppHandle) -> Result<(), String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static WINDOW_SEQ: AtomicU32 = AtomicU32::new(1);
    let label = format!("window-{}", WINDOW_SEQ.fetch_add(1, Ordering::Relaxed));
    let w = tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("/".into()))
        .title("Rift")
        .decorations(false)
        .inner_size(1600.0, 1000.0)
        .min_inner_size(900.0, 600.0)
        .visible(false)
        .build()
        .map_err(|e| format!("open_new_window: {e}"))?;
    crate::center_in_work_area(&w);
    let _ = w.show();
    let _ = w.set_focus();
    Ok(())
}

/// #37 cross-window sync — broadcast that the on-disk conversation store changed
/// (a save/delete/rename in `origin_label`'s window) to every OTHER window so it
/// can re-pull `assistant_list_conversations`. Origin is skipped: it already
/// refreshed its own list locally, and re-firing it would loop. Two windows
/// share one disk store but separate in-memory lists, so without this a chat
/// created in window-2 never shows up in window-1's sidebar until a reload.
#[tauri::command]
pub fn broadcast_convos_changed(app: tauri::AppHandle, origin_label: String) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    for (label, _) in app.webview_windows() {
        if label == origin_label {
            continue;
        }
        let _ = app.emit_to(label.as_str(), "convos-changed", ());
    }
    Ok(())
}
