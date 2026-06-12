//! Rift — pure local-workspace coding assistant (Tauri + Svelte).
//! The Claude CLI integration + local MCP server live under `assistant/`.
//!
//! Auto-update via Velopack (v0.4.47+, restored 2026-06-04, see
//! `update_service.rs` + `commands/update.rs`). Frontend checks the latest
//! GitHub release on launch + every 6h; one-click download streams progress,
//! then Velopack applies on exit and relaunches the new version unattended.
//! `VelopackApp::build().run()` runs early in `run()` to handle install/update
//! hooks. (Replaced the GH-release-API browser-handoff path; that in turn
//! replaced `tauri-plugin-updater`, which bricked clients on key loss.)
//!
//! Tauri command surface at the bottom of this file (`run()`'s
//! `invoke_handler!`). Command fns live under `commands/` — one file per
//! domain (#20).

pub mod assistant;
pub mod browser;
pub mod commands;
pub mod diagnostics;
pub mod secrets;
pub mod state;
pub mod stt;
pub mod update_service;
pub mod usage;

use tauri::Manager;

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
        let payload_raw = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        let payload = diagnostics::scrub_log_message(payload_raw);
        let location = diagnostics::scrub_log_message(&location);
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
    // MUST precede VelopackApp::build() — in stdio mode nothing may touch
    // stdout (it would corrupt the JSON-RPC framing), and the Velopack
    // installer never launches us with RIFT_MCP_SERVER set.
    if std::env::var_os("RIFT_MCP_SERVER").is_some() {
        assistant::mcp_server::run_stdio();
        return;
    }

    // Velopack: install/update/uninstall hooks run FIRST on normal launches
    // (the installer passes `--veloapp-*` args). In all other cases this is a
    // near-instant no-op. Must run before Tauri spins up. See update_service.rs.
    velopack::VelopackApp::build().run();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(std::sync::Arc::new(assistant::AskUserRegistry::new()))
        .manage(std::sync::Arc::new(assistant::PermissionRegistry::new()))
        .manage(std::sync::Arc::new(update_service::UpdateService::new()))
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
            // Assistant UI bridge (ask_user / open_browser / notify): bind the
            // loopback listener before the first turn can spawn an MCP child.
            // Failure is non-fatal — write_mcp_config skips the env injection
            // and the MCP child simply doesn't list the bridge tools.
            let bridge_app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = assistant::bridge::start(bridge_app).await {
                    log::error!("assistant bridge failed to start: {e}");
                }
            });
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
            commands::open_in_vscode,
            commands::check_for_updates,
            commands::download_update,
            commands::apply_pending_update,
            commands::assistant_auth_probe,
            commands::assistant_open_login,
            commands::assistant_update_cli,
            commands::assistant_get_api_key_present,
            commands::assistant_set_api_key,
            commands::assistant_get_use_full_config,
            commands::assistant_set_use_full_config,
            commands::assistant_get_max_budget_usd,
            commands::assistant_set_max_budget_usd,
            commands::assistant_get_trust_level,
            commands::assistant_set_trust_level,
            commands::environment_check,
            commands::assistant_send,
            commands::assistant_enhance_prompt,
            commands::assistant_enhance_cancel,
            commands::assistant_generate_title,
            commands::assistant_stop,
            commands::assistant_steer,
            commands::assistant_answer_ask_user,
            commands::assistant_answer_permission,
            commands::assistant_list_conversations,
            commands::assistant_load_conversation,
            commands::assistant_session_cwd,
            commands::assistant_export_save,
            commands::assistant_save_conversation,
            commands::assistant_delete_conversation,
            commands::assistant_get_workspace,
            commands::assistant_set_root,
            commands::assistant_clear_root,
            commands::assistant_remove_recent_root,
            commands::assistant_list_workspace_files,
            commands::assistant_workspace_branch,
            usage::limits::usage_rate_limits,
            commands::browser_open,
            commands::browser_navigate,
            commands::browser_set_bounds,
            commands::browser_show,
            commands::browser_hide,
            commands::browser_current_url,
            commands::browser_close,
            commands::browser_back,
            commands::browser_forward,
            commands::browser_reload,
            commands::browser_read_page,
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
        .run(|_app_handle, event| {
            // Scrub the on-disk bridge token from `~/.rift/assistant/mcp-config.json`
            // on app exit. The token becomes stale the instant the process exits.
            if let tauri::RunEvent::Exit = event {
                assistant::cleanup_mcp_config_on_exit();
            }
        });
}
