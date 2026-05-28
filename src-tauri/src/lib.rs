//! Rift v14 — Tauri + Svelte + russh backend.
//!
//! Auto-update via the GH-release-API path (v0.4.34+, see
//! `commands/update.rs`). Frontend polls the latest GitHub release on launch,
//! offers a one-click "Download" that opens the Setup.exe asset URL via
//! `tauri-plugin-opener`; NSIS handles install over the running binary.
//! No signing key, no `latest.json`, no plugin runtime dependency — the
//! prior `tauri-plugin-updater` path bricked all clients on key loss
//! (2026-05-27 incident → v0.4.34 rebuild).
//!
//! Tauri command surface at the bottom of this file (`run()`'s
//! `invoke_handler!`). Command fns live under `commands/` — one file per
//! domain (#20).

pub mod assistant;
pub mod bootstrap;
pub mod bridge;
pub mod browser;
pub mod commands;
pub mod diagnostics;
pub mod edit;
pub mod local_fs;
pub mod path_guard;
pub mod profile;
pub mod secrets;
pub mod sftp;
pub mod state;
pub mod sync;
pub mod stt;
pub mod transport;
pub mod tunnel;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use crate::sync::AutoSyncEngine;

/// Tauri-managed handle to the active AutoSync engine. None until start_autosync.
pub struct AutoSyncState(pub AsyncMutex<Option<Arc<AutoSyncEngine>>>);

/// Tauri-managed handle to the active SSH local-port-forward (Phase 1g). Some
/// only when a profile w/ `bridge_port` is active. Lifecycle bound to AutoSync.
pub struct TunnelState(pub AsyncMutex<Option<tunnel::SshTunnel>>);

/// Phase 4: per-server EditInPlaceManager. Keyed by server_key.
pub struct EditInPlaceState(
    pub AsyncMutex<std::collections::HashMap<String, Arc<edit::in_place::EditInPlaceManager>>>,
);

/// M13: holds the CancellationToken for the currently in-flight download_paths.
pub struct DownloadState(pub AsyncMutex<Option<CancellationToken>>);

/// #106: cancellation handle for the `diag_state_pump` background task. Fired
/// on `RunEvent::Exit` so the pump exits before Tauri tears the runtime down.
pub struct DiagPumpCancel(pub CancellationToken);

/// #249: subscriber refcount for `diag_state_pump`. Pump short-circuits the
/// 500ms collect+emit when count == 0 (no Diagnostics tab mounted). Refcount
/// (not bool) because the Diagnostics surface can be mounted twice — as a
/// page workspace AND as `<Diagnostics embedded />` inside ActivityFeed.
/// `diag_subscribe_state`/`diag_unsubscribe_state` increment/decrement.
pub struct DiagPumpSubscribers(pub std::sync::atomic::AtomicU64);

/// Application entry point. Registers managed state + Tauri commands and
/// blocks on the event loop. Update flow lives in `commands/update.rs` —
/// frontend pulls GitHub release metadata, opens Setup.exe URL on confirm.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Logger init early so panic hook + plugin init events surface in stderr.
    // RUST_LOG controls level; default = info.
    diagnostics::LogForwarder::install();

    // #219: install a global panic hook so async-task panics don't die silently.
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        log::error!("panic at {location}: {payload}");
        diagnostics::emit_with_fields(
            diagnostics::DiagStage::System,
            diagnostics::DiagLevel::Error,
            None,
            None,
            format!("panic at {location}: {payload}"),
            serde_json::json!({ "location": location, "payload": payload }),
        );
    }));

    // Center the main window inside the primary monitor's WORK AREA (screen
    // minus the taskbar / dock). Tauri's built-in `center: true` uses full
    // monitor size, which shifts the window visually low when the taskbar eats
    // the bottom band. SPI_GETWORKAREA returns the taskbar-excluded rect.
    #[cfg(target_os = "windows")]
    fn center_in_work_area(window: &tauri::WebviewWindow) {
        #[repr(C)]
        #[derive(Default)]
        struct Rect { left: i32, top: i32, right: i32, bottom: i32 }
        extern "system" {
            fn SystemParametersInfoW(
                ui_action: u32,
                ui_param: u32,
                pv_param: *mut std::ffi::c_void,
                f_win_ini: u32,
            ) -> i32;
        }
        const SPI_GETWORKAREA: u32 = 0x0030;
        let mut wa = Rect::default();
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                &mut wa as *mut _ as *mut std::ffi::c_void,
                0,
            )
        };
        if ok == 0 { return; }
        let Ok(size) = window.outer_size() else { return; };
        let work_w = wa.right - wa.left;
        let work_h = wa.bottom - wa.top;
        let x = wa.left + (work_w - size.width as i32) / 2;
        let y = wa.top + (work_h - size.height as i32) / 2;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    #[cfg(not(target_os = "windows"))]
    fn center_in_work_area(_window: &tauri::WebviewWindow) {}

    // Assistant α2: when launched by the Claude CLI as an MCP server (env
    // RIFT_MCP_SERVER=1), serve JSON-RPC on stdio and skip Tauri entirely.
    if std::env::var_os("RIFT_MCP_SERVER").is_some() {
        assistant::mcp_server::run_stdio();
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(std::sync::Arc::new(assistant::AskUserRegistry::new()))
        .manage(std::sync::Arc::new(assistant::PermissionRegistry::new()))
        .manage(AutoSyncState(AsyncMutex::new(None)))
        .manage(TunnelState(AsyncMutex::new(None)))
        .manage(EditInPlaceState(AsyncMutex::new(std::collections::HashMap::new())))
        .manage(DownloadState(AsyncMutex::new(None)))
        .manage(stt::DownloadCancel(std::sync::Mutex::new(None)))
        .manage(stt::WhisperCache(tokio::sync::Mutex::new(None)))
        .manage(stt::WhisperSession(tokio::sync::Mutex::new(None)))
        .setup(|app| {
            // Window starts hidden (`visible: false` in tauri.conf.json) so we
            // can position it before the user sees it.
            if let Some(main) = app.get_webview_window("main") {
                center_in_work_area(&main);
                let _ = main.show();
                let _ = main.set_focus();
            }
            // Diagnostics: stream bus events to the frontend (`diag://event`)
            // and emit a periodic pipeline-state snapshot (`diag://state`)
            // every 500ms. Both run for the life of the process.
            let app_handle = app.handle().clone();
            diagnostics::spawn_frontend_pump(app_handle.clone());
            // #106: stash the pump's cancel token in managed state so
            // RunEvent::Exit can cancel it cleanly.
            let pump_cancel = CancellationToken::new();
            app.manage(DiagPumpCancel(pump_cancel.clone()));
            app.manage(DiagPumpSubscribers(std::sync::atomic::AtomicU64::new(0)));
            tauri::async_runtime::spawn(commands::sync::diag_state_pump(app_handle, pump_cancel));
            // Phase E4: best-effort sweep of CLI JSONLs whose sessions were
            // retired by compaction >30 days ago.
            tauri::async_runtime::spawn_blocking(|| {
                let deleted = assistant::cleanup_retired_jsonls();
                if deleted > 0 {
                    log::info!("assistant: startup sweep deleted {} retired JSONL(s)", deleted);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::scan_drift,
            commands::start_autosync,
            commands::stop_autosync,
            commands::get_autosync_status,
            commands::enqueue_for_flush_batch,
            commands::resolve_conflict,
            commands::resolve_conflicts_bulk,
            commands::retry_failed,
            commands::list_servers,
            commands::get_last_selected,
            commands::set_last_selected,
            commands::save_server,
            commands::delete_server,
            commands::local_list_dir,
            commands::remote_list_dir,
            commands::upload_paths,
            commands::download_paths,
            commands::cancel_download,
            commands::remote_rename_path,
            commands::remote_delete_paths,
            commands::local_rename_path,
            commands::local_delete_paths,
            commands::detect_bootstrap,
            commands::bootstrap_list_files,
            commands::generate_ssh_key,
            commands::generate_default_ssh_key,
            commands::default_ssh_key_exists,
            commands::default_ssh_key_path,
            commands::read_default_ssh_pub_key,
            commands::probe_server_fingerprint,
            commands::set_server_fingerprint,
            commands::clear_server_fingerprint,
            commands::rotate_bridge_token,
            commands::validate_ssh_key_file,
            commands::check_for_updates,
            commands::begin_edit_in_place,
            commands::save_edit_in_place,
            commands::close_edit_in_place,
            commands::list_watched_edits,
            commands::diag_get_state,
            commands::diag_snapshot_path,
            commands::diag_log_frontend_error,
            commands::diag_subscribe_state,
            commands::diag_unsubscribe_state,
            commands::sync_reconcile,
            commands::sync_cancel,
            commands::sync_pull_pending,
            commands::sync_push_pending,
            commands::sync_get_drift_snapshot,
            commands::list_watched_folders,
            commands::sync_get_aborted_shrunk,
            commands::sync_rebaseline_folder,
            commands::sync_apply_selected,
            commands::sync_sweep_stale_locks,
            commands::sync_set_mirror_mode,
            commands::sync_get_mirror_mode,
            commands::diag_ignored_breakdown,
            commands::assistant_auth_probe,
            commands::assistant_get_api_key_present,
            commands::assistant_set_api_key,
            commands::assistant_get_use_full_config,
            commands::assistant_set_use_full_config,
            commands::assistant_get_max_budget_usd,
            commands::assistant_set_max_budget_usd,
            commands::assistant_get_allow_remote_shell,
            commands::assistant_set_allow_remote_shell,
            commands::assistant_get_thinking_effort,
            commands::assistant_set_thinking_effort,
            commands::assistant_get_permission_mode,
            commands::assistant_set_permission_mode,
            commands::assistant_get_auto_compact_threshold,
            commands::assistant_set_auto_compact_threshold,
            commands::assistant_get_compact_model,
            commands::assistant_set_compact_model,
            commands::assistant_summarize_session,
            commands::assistant_remint_session,
            commands::assistant_send,
            commands::assistant_enhance_prompt,
            commands::assistant_stop,
            commands::assistant_answer_ask_user,
            commands::assistant_answer_permission,
            commands::assistant_list_conversations,
            commands::assistant_load_conversation,
            commands::assistant_save_conversation,
            commands::assistant_delete_conversation,
            commands::assistant_get_workspace,
            commands::assistant_set_root,
            commands::assistant_clear_root,
            commands::assistant_remove_recent_root,
            commands::assistant_list_workspace_files,
            commands::browser_open,
            commands::browser_navigate,
            commands::browser_set_bounds,
            commands::browser_show,
            commands::browser_hide,
            commands::browser_current_url,
            commands::browser_close,
            stt::stt_get_config,
            stt::stt_set_config,
            stt::stt_set_engine,
            stt::stt_start_recording,
            stt::stt_stop_recording,
            stt::stt_get_input_devices,
            stt::stt_backend_available,
            stt::stt_list_models,
            stt::stt_download_model,
            stt::stt_cancel_download,
            stt::stt_delete_model,
            stt::stt_clean_transcript,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            // Scrub the on-disk bridge token from `~/.rift/assistant/mcp-config.json`
            // on app exit. The token becomes stale the instant the process exits.
            if let tauri::RunEvent::Exit = event {
                // #106: cancel the diag pump before cleanup so it stops emitting
                // events into a tearing-down Tauri runtime.
                if let Some(c) = app_handle.try_state::<DiagPumpCancel>() {
                    c.0.cancel();
                }
                assistant::cleanup_mcp_config_on_exit();
            }
        });
}
