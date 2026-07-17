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
pub mod certs;
pub mod commands;
pub mod diagnostics;
pub mod elevation;
pub mod job_object;
pub mod secrets;
pub mod state;
pub mod stt;
pub mod update_service;
pub mod usage;

use tauri::Manager;

// Center a window inside the primary monitor's WORK AREA (screen minus the
// taskbar / dock). Tauri's built-in `center: true` uses full monitor size,
// which shifts the window visually low when the taskbar eats the bottom band.
// SPI_GETWORKAREA returns the taskbar-excluded rect. Shared by the main-window
// setup + the `open_new_window` command.
#[cfg(target_os = "windows")]
pub(crate) fn center_in_work_area(window: &tauri::WebviewWindow) {
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
pub(crate) fn center_in_work_area(_window: &tauri::WebviewWindow) {}

/// Application entry point. Registers managed state + Tauri commands and
/// blocks on the event loop. Update flow lives in `commands/update.rs` —
/// frontend pulls GitHub release metadata, opens Setup.exe URL on confirm.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Logger init early so panic hook + plugin init events surface in stderr.
    // RUST_LOG controls level; default = info.
    diagnostics::LogForwarder::install();

    // cont.228: reqwest uses `rustls-no-provider` (so aws-lc-rs stays out of the
    // dep tree — we ship ring only). With no provider compiled into reqwest, it
    // calls `CryptoProvider::get_default()` and PANICS if none is installed. Set
    // ring as the process default here, before any TLS client is built. Idempotent
    // and racy-safe: `install_default` returns Err if already set — we ignore it.
    let _ = rustls::crypto::ring::default_provider().install_default();

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
        // Field crash observability: persist a dedicated, non-rotating
        // crash-<ts>.txt. Survives a second crash + captures startup panics
        // that fire before the frontend pump exists. (RR-2)
        diagnostics::write_crash_report(&location, &payload);
        diagnostics::emit_with_fields(
            diagnostics::DiagStage::System,
            diagnostics::DiagLevel::Error,
            None,
            None,
            format!("panic at {location}: {payload}"),
            serde_json::json!({ "location": location, "payload": payload }),
        );
    }));

    // Assistant α2: when launched by the Claude CLI as an MCP server (env
    // RIFT_MCP_SERVER=1), serve JSON-RPC on stdio and skip Tauri entirely.
    // MUST precede VelopackApp::build() — in stdio mode nothing may touch
    // stdout (it would corrupt the JSON-RPC framing), and the Velopack
    // installer never launches us with RIFT_MCP_SERVER set.
    if std::env::var_os("RIFT_MCP_SERVER").is_some() {
        assistant::mcp_server::run_stdio();
        return;
    }

    // NOTE: a Windows Job Object (KILL_ON_JOB_CLOSE) was prototyped here to
    // structurally reap all children on abnormal exit — but it's UNSAFE with
    // Velopack self-update: Velopack spawns `Update.exe` via CreateProcessW with
    // NO CREATE_BREAKAWAY_FROM_JOB (verified in velopack 1.2.0 process_win.rs),
    // so it would inherit the kill-on-close job and be terminated the instant the
    // old Rift exits — bricking the swap+relaunch. Simple job flags can't keep
    // WebView2/claude children IN the job while letting Update.exe OUT. See
    // src/job_object.rs (kept, not called) + docs for the safe designs
    // (JobObjectAssociateCompletionPort watcher process, or CREATE_BREAKAWAY on
    // our own Update.exe wrapper). Flagged for owner — do NOT wire in blindly.

    // Velopack: install/update/uninstall hooks run FIRST on normal launches
    // (the installer passes `--veloapp-*` args). In all other cases this is a
    // near-instant no-op. Must run before Tauri spins up. See update_service.rs.
    velopack::VelopackApp::build().run();

    // Administrator elevation reconciliation. When "always run as administrator"
    // is on, a non-elevated launch hands off to an elevated instance (via the
    // per-user Scheduled Task, no UAC prompt) and exits here; the elevated
    // instance continues. Off (the default) = an immediate no-op. MUST run after
    // the RIFT_MCP_SERVER early return above (an MCP child must never relaunch)
    // and after Velopack (so update/install hooks are handled first). See
    // `elevation.rs`.
    if elevation::bootstrap() == elevation::Boot::Exit {
        return;
    }

    // Prime the corporate-root PEM before the first claude spawn so the file
    // exists on disk by the time any cli_install::claude_command() is called.
    // Additive-only: the PEM carries the user's own Windows-store roots; Node
    // still trusts its built-ins, reqwest still trusts webpki — on a non-proxied
    // machine this changes nothing observable. Never disables verification.
    let _ = certs::corp_pem_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(std::sync::Arc::new(assistant::AskUserRegistry::new()))
        .manage(std::sync::Arc::new(assistant::PermissionRegistry::new()))
        .manage(std::sync::Arc::new(update_service::UpdateService::new()))
        .manage(stt::DownloadCancel(std::sync::Mutex::new(None)))
        .manage(stt::EngineCache(tokio::sync::Mutex::new(None)))
        .manage(stt::SttSession(tokio::sync::Mutex::new(None)))
        .setup(|app| {
            // Window starts hidden (`visible: false` in tauri.conf.json) so we
            // can position it before the user sees it.
            if let Some(main) = app.get_webview_window("main") {
                center_in_work_area(&main);
                // show() makes the window visible (taskbar entry + a flash) but
                // does NOT steal foreground. Deliberately NO set_focus() here:
                // an auto-update/dev relaunch firing set_focus() yanks the user
                // out of a fullscreen game (focus loss = minimized game). The
                // user clicks the taskbar when THEY want Rift. (2026-06-25)
                let _ = main.show();
            }
            // Diagnostics: stream bus events to the frontend (`diag://event`).
            // Event-driven (parks on the bus, no polling); runs for the life
            // of the process.
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
            // No-think loopback shim (local-LLM mode only): injects
            // thinking:disabled into /v1/messages so Ollama-class models skip
            // their forced reasoning dump. Replaces the external Node proxy.
            // Non-fatal — turn.rs falls back to the raw base URL if it can't bind.
            tauri::async_runtime::spawn(async move {
                if let Err(e) = assistant::nothink::start().await {
                    log::error!("assistant nothink shim failed to start: {e}");
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
            // Warm the CLI-capabilities cache at boot. The first turn calls
            // CliCaps::active(), which on a cold cache shells out to probe
            // `claude --version` (up to a 5s block) — that cost landed on the
            // user's first message's TTFT. Priming it here moves the probe off
            // the hot path so the first turn hits the cached value.
            tauri::async_runtime::spawn_blocking(|| {
                let _ = assistant::cli_caps::CliCaps::active();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::open_in_vscode,
            commands::open_new_window,
            commands::broadcast_convos_changed,
            commands::query_turn_perf,
            commands::list_mcp_servers,
            commands::check_for_updates,
            commands::download_update,
            commands::apply_pending_update,
            commands::repair_install,
            commands::assistant_auth_probe,
            commands::assistant_open_login,
            commands::assistant_update_cli,
            commands::cli_native_latest,
            commands::assistant_get_api_key_present,
            commands::assistant_set_api_key,
            commands::assistant_get_use_full_config,
            commands::assistant_set_use_full_config,
            commands::assistant_get_max_budget_usd,
            commands::assistant_set_max_budget_usd,
            commands::assistant_get_trust_level,
            commands::assistant_set_trust_level,
            commands::assistant_get_local_llm_config,
            commands::assistant_local_model_context,
            commands::assistant_optimize_local_model,
            commands::assistant_list_providers,
            commands::assistant_upsert_provider,
            commands::assistant_delete_provider,
            commands::assistant_activate_provider,
            commands::assistant_set_provider_key,
            commands::assistant_test_provider,
            commands::assistant_list_provider_models,
            commands::environment_check,
            commands::install_local_tool,
            commands::assistant_send,
            commands::assistant_prewarm,
            commands::assistant_enhance_prompt,
            commands::assistant_enhance_cancel,
            commands::assistant_generate_title,
            commands::assistant_analyze_usage,
            commands::assistant_fetch_ai_news,
            commands::assistant_summarize_ai_news,
            commands::assistant_stop,
            commands::assistant_kill_shell,
            commands::assistant_answer_ask_user,
            commands::assistant_answer_permission,
            commands::assistant_list_conversations,
            commands::assistant_stats,
            commands::assistant_load_conversation,
            commands::assistant_session_cwd,
            commands::assistant_save_conversation,
            commands::assistant_delete_conversation,
            commands::assistant_get_workspace,
            commands::assistant_set_root,
            commands::assistant_set_tab_root,
            commands::assistant_remove_recent_root,
            commands::assistant_list_workspace_files,
            commands::assistant_list_custom_commands,
            commands::assistant_workspace_branch,
            assistant::gh_remote::gh_branch_status,
            commands::assistant_local_scratch_path,
            commands::assistant_list_projects,
            commands::assistant_save_project,
            commands::assistant_delete_project,
            usage::limits::usage_rate_limits,
            commands::browser_open,
            commands::browser_set_bounds,
            commands::browser_show,
            commands::browser_hide,
            commands::browser_current_url,
            commands::browser_close,
            commands::browser_back,
            commands::browser_forward,
            commands::browser_reload,
            commands::browser_read_page,
            commands::browser_read_console,
            commands::browser_console_counts,
            commands::resolve_workspace_path,
            commands::elevation_status,
            commands::elevation_relaunch_as_admin,
            commands::elevation_set_always,
            stt::stt_get_config,
            stt::stt_set_config,
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
            if let tauri::RunEvent::Exit = event {
                // Reap every live `claude` child (+ its `rift-tauri.exe` MCP
                // grandchild) before the process dies. Idle children EOF-exit on
                // their own once the stdin pipes close, but a MID-TURN child keeps
                // executing its in-flight turn headless — lingering in Task
                // Manager and burning CPU + API tokens with no window to stop it.
                // Same primitives as the update-apply path; idempotent when that
                // path re-enters here (registries already drained → no-op). The
                // apply-only IMAGENAME sweep is deliberately NOT mirrored — on a
                // normal exit it could hit a second running Rift instance.
                assistant::warm_pool::drain_all_for_shutdown();
                assistant::kill_all_session_children();
                // Scrub the on-disk bridge token from
                // `~/.rift/assistant/mcp-config.json` — stale the instant we exit.
                assistant::cleanup_mcp_config_on_exit();
            }
        });
}
