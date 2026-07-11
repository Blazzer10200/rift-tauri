//! Administrator-elevation commands (#20 domain split). The elevation
//! mechanics live in `crate::elevation`; these are the thin Tauri surface the
//! Settings page + the status-bar badge call.

use serde::Serialize;

/// Snapshot of the current elevation state for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ElevationStatus {
    /// Elevation is a Windows concept — false everywhere else (controls hidden).
    pub supported: bool,
    /// This process currently holds an elevated (admin) token.
    pub elevated: bool,
    /// The always-elevated launcher is fully set up (preference on AND the
    /// Scheduled Task exists — the task is the real source of truth).
    pub always_elevated: bool,
    /// Raw preference. Differs from `always_elevated` only in the brief mid-setup
    /// window (enabled, but the task hasn't been registered by the elevated
    /// instance yet).
    pub pref_on: bool,
}

/// Result of toggling the always-elevated preference.
#[derive(Debug, Clone, Serialize)]
pub struct ElevationApply {
    pub always_elevated: bool,
    /// True when the app is about to relaunch elevated to finish enabling (a
    /// one-time UAC prompt) — the UI shows a "Relaunching as administrator…"
    /// state instead of a normal toggle flip.
    pub relaunching: bool,
}

/// Current elevation state — cheap synchronous reads (token query + a
/// `schtasks /Query`). Drives the Administrator badge + the Settings section.
#[tauri::command]
pub fn elevation_status() -> ElevationStatus {
    let pref_on = crate::assistant::get_always_elevated();
    let task = crate::elevation::task_exists();
    ElevationStatus {
        supported: cfg!(windows),
        elevated: crate::elevation::is_elevated(),
        always_elevated: pref_on && task,
        pref_on,
    }
}

/// Relaunch Rift as administrator for this session (one UAC prompt). On success
/// the elevated instance is launched and this one exits shortly after, so only
/// the elevated instance remains. On cancel/failure the app keeps running
/// non-elevated and the error surfaces in the UI.
#[tauri::command]
pub fn elevation_relaunch_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    crate::elevation::relaunch_as_admin()?;
    exit_after_handoff(app);
    Ok(())
}

/// Enable or disable "always run Rift as administrator".
///
/// **Enable** persists the preference first (so intent survives a relaunch),
/// then: if already elevated, registers the prompt-free launcher task now; if
/// not, does a one-time UAC relaunch — the elevated instance's boot
/// reconciliation registers the task, and every launch after that is
/// prompt-free. **Disable** removes the task and clears the preference (the app
/// keeps running as-is).
#[tauri::command]
pub fn elevation_set_always(app: tauri::AppHandle, enabled: bool) -> Result<ElevationApply, String> {
    if !enabled {
        crate::elevation::delete_task()?;
        crate::assistant::set_always_elevated(false)?;
        return Ok(ElevationApply { always_elevated: false, relaunching: false });
    }

    // Persist intent up front so a relaunch doesn't lose it.
    crate::assistant::set_always_elevated(true)?;

    if crate::elevation::is_elevated() {
        crate::elevation::create_task()?;
        return Ok(ElevationApply { always_elevated: true, relaunching: false });
    }

    // Not elevated — registering a HighestAvailable task needs admin. One-time
    // UAC relaunch; the elevated instance reconciles the task on boot.
    crate::elevation::relaunch_as_admin().map_err(|e| {
        // Roll the preference back on cancel/failure — otherwise every launch
        // would keep re-attempting the relaunch with no task ever created.
        let _ = crate::assistant::set_always_elevated(false);
        e
    })?;
    exit_after_handoff(app);
    Ok(ElevationApply { always_elevated: true, relaunching: true })
}

/// Tear down this instance shortly after handing off to an elevated one — the
/// short delay lets the command's response reach the UI before the window dies.
/// `app.exit(0)` runs the normal exit path (warm-pool drain + child reap +
/// mcp-config scrub), so no children are orphaned.
fn exit_after_handoff(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        app.exit(0);
    });
}
