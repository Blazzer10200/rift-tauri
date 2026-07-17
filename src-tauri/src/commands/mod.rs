//! Tauri command surface — split per domain (#20).
//!
//! lib.rs's `invoke_handler!` references these via `commands::*`. Each domain
//! file owns its #[tauri::command] fns + private helpers. Cross-domain helpers
//! (path-traversal guard, fingerprint pin guard, log-basename) live here.

pub mod assistant;
pub mod browser;
pub mod elevation;
pub mod update;

pub use assistant::*;
pub use browser::*;
pub use elevation::*;
pub use update::*;

/// Open a path in VS Code, optionally at a line (`-g path:line`). The opener
/// plugin's `openWith` can't launch VS Code on Windows (its CLI is `code.cmd`,
/// which `Command::new("code")` won't resolve without a shell) — so spawn
/// through `cmd /C` on Windows, direct `code` elsewhere.
#[tauri::command]
pub fn open_in_vscode(path: String, line: Option<u32>) -> Result<(), String> {
    let target = match line {
        Some(l) if l > 0 => format!("{path}:{l}"),
        _ => path.clone(),
    };
    // The Windows path runs through `cmd /C`, which re-parses metacharacters
    // *after* Rust's arg quoting — so `&`/`|`/`<`/`>`/`^`/`%`/`"` (and control
    // chars) could chain a second command. None are needed to open a file; reject
    // them rather than try to escape cmd.exe's quoting (F19).
    #[cfg(windows)]
    if target.bytes().any(|b| matches!(b, b'&' | b'|' | b'<' | b'>' | b'^' | b'%' | b'"') || b < 0x20) {
        return Err("path contains characters that can't be passed safely to the shell".into());
    }
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "code", "-g", &target]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = std::process::Command::new("code");
        c.args(["-g", &target]);
        c
    };
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't launch VS Code (is `code` on PATH?): {e}"))
}

/// Shared resolution for clickable workspace paths: resolve `path` against
/// `root`, refuse anything that escapes it. A bare filename (no separator)
/// gets a bounded workspace search so `turn.rs` in prose still lands on the
/// file. Filesystem walk + canonicalize run off the async worker.
async fn resolve_in_workspace(
    root: Option<String>,
    path: String,
) -> Result<std::path::PathBuf, String> {
    let raw = path.trim().to_string();
    if raw.is_empty() {
        return Err("empty path".into());
    }
    let root = root
        .filter(|r| !r.trim().is_empty())
        .ok_or("No workspace folder is open")?;

    tokio::task::spawn_blocking(move || -> Result<std::path::PathBuf, String> {
        let root = std::path::PathBuf::from(&root)
            .canonicalize()
            .map_err(|_| "workspace root not found".to_string())?;
        let p = std::path::Path::new(&raw);
        let candidate = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
        let has_sep = raw.contains('/') || raw.contains('\\');
        let resolved = match candidate.canonicalize() {
            Ok(c) => Some(c),
            // Bare filename that doesn't sit at the root — bounded search.
            Err(_) if !has_sep => find_by_name(&root, &raw),
            Err(_) => None,
        };
        let resolved = resolved.ok_or_else(|| format!("Not found in this workspace: {raw}"))?;
        // Containment AFTER canonicalize so a symlink can't escape the root.
        if !resolved.starts_with(&root) {
            return Err("path is outside the open workspace".into());
        }
        Ok(resolved)
    })
    .await
    .map_err(|e| format!("resolve: {e}"))?
}

/// Clickable file paths in chat (Markdown `.md-path` spans). Resolves `path`
/// against the focused tab's workspace `root` and hands back the absolute,
/// containment-checked path — the frontend feeds it to `FilePathMenu` (open in
/// VS Code / default app / reveal in the system file manager / copy), the same
/// action set every other file-path surface in the app already offers.
#[tauri::command]
pub async fn resolve_workspace_path(root: Option<String>, path: String) -> Result<String, String> {
    let resolved = resolve_in_workspace(root, path).await?;
    Ok(resolved.display().to_string())
}

/// Bounded workspace search for a bare filename — first match wins, heavy
/// build/dep dirs skipped, hard entry cap so a giant tree can't stall the click.
fn find_by_name(root: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    const SKIP: [&str; 8] =
        ["node_modules", ".git", "target", ".svelte-kit", "build", "dist", ".next", ".venv"];
    let mut seen = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && e.file_name().to_str().map(|n| SKIP.contains(&n)).unwrap_or(false))
        })
    {
        let Ok(e) = entry else { continue };
        seen += 1;
        if seen > 20_000 {
            return None;
        }
        if e.file_type().is_file()
            && e.file_name().to_str().map(|n| n.eq_ignore_ascii_case(name)).unwrap_or(false)
        {
            return e.path().canonicalize().ok();
        }
    }
    None
}

/// B2 — AI Health turn-performance aggregate. Reads the persisted `turns.ndjson`
/// (p50/p90 latency, cache-hit rate, cost-by-day) off the async executor so the
/// file parse never stalls a Tauri worker. Returns a zero-filled aggregate when
/// no turns have been recorded yet (first launch) rather than erroring.
/// `window_hours` narrows the aggregate to the AI Health range picker's window
/// (24/168/720); absent = full log (backward compatible).
#[tauri::command]
pub async fn query_turn_perf(
    window_hours: Option<u32>,
) -> Result<crate::diagnostics::perf::TurnPerfStats, String> {
    tokio::task::spawn_blocking(move || {
        crate::diagnostics::perf::query_turn_perf_sync(window_hours)
    })
    .await
    .map_err(|e| format!("query_turn_perf: {e}"))
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
    // #37: launch-scoped nonce. A plain `window-<seq>` reset every launch, so
    // launch 2's first secondary window re-used launch 1's label and inherited
    // its persisted pane/split layout (labels namespace localStorage keys —
    // see persistence.ts). Secondary windows are ephemeral by design; the
    // nonce guarantees a fresh key, and the FE prunes orphaned keys at boot.
    static LAUNCH_NONCE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let nonce = LAUNCH_NONCE.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    });
    let label = format!("window-{nonce:x}-{}", WINDOW_SEQ.fetch_add(1, Ordering::Relaxed));
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
