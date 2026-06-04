//! In-app web browser backend — native embedded webview.
//!
//! Embeds a real child WebView2 *inside* Rift's main window via Tauri's
//! multiwebview API (`Window::add_child`, gated behind the `unstable` feature).
//! Unlike a CDP-spawned page, a child webview is part of the host window — no
//! separate taskbar entry — and gives native scroll / text-selection / clicks.
//!
//! The frontend renders an empty placeholder ("stage") in the chat's browser
//! dock and reports its rect; this module positions/sizes the native webview to
//! overlap it. The webview floats above the main webview, so the frontend must
//! `hide` it whenever the dock isn't actually visible (dock closed, or a
//! different workspace is active).

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Url, WebviewBuilder, WebviewUrl};

/// Label of the embedded webview child. Single instance — one browser dock.
const LABEL: &str = "rift-browser";
/// Host window the dock lives in.
const HOST_WINDOW: &str = "main";

fn parse_url(raw: &str) -> Result<Url, String> {
    let u = Url::parse(raw).map_err(|e| format!("invalid URL '{raw}': {e}"))?;
    // Scheme allowlist: only real web navigation. `file://` would expose the
    // local disk, `javascript:`/`data:` would execute attacker-controlled script
    // in the embedded webview — both reachable from AI-generated links.
    match u.scheme() {
        "http" | "https" | "about" => Ok(u),
        other => Err(format!("blocked URL scheme '{other}:' — only http/https are allowed")),
    }
}

/// Create the child webview at the given window-relative rect (or, if it
/// already exists, navigate + reposition + show it). Idempotent.
pub fn open(app: &AppHandle, url: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    let u = parse_url(url)?;
    if let Some(wv) = app.get_webview(LABEL) {
        wv.navigate(u).map_err(|e| format!("navigate: {e}"))?;
        let _ = wv.set_position(LogicalPosition::new(x, y));
        let _ = wv.set_size(LogicalSize::new(w, h));
        let _ = wv.show();
        return Ok(());
    }
    let window = app
        .get_window(HOST_WINDOW)
        .ok_or("main window not found")?;
    let wv = window
        .add_child(
            WebviewBuilder::new(LABEL, WebviewUrl::External(u.clone())),
            LogicalPosition::new(x, y),
            LogicalSize::new(w, h),
        )
        .map_err(|e| format!("create embedded webview: {e}"))?;
    // The builder's initial External URL doesn't reliably load on the child
    // webview (WebView2 starts it at about:blank); navigate explicitly.
    wv.navigate(u).map_err(|e| format!("navigate: {e}"))?;
    Ok(())
}

/// Navigate the existing dock webview. No-op (Ok) if it isn't open.
pub fn navigate(app: &AppHandle, url: &str) -> Result<(), String> {
    let u = parse_url(url)?;
    if let Some(wv) = app.get_webview(LABEL) {
        wv.navigate(u).map_err(|e| format!("navigate: {e}"))?;
    }
    Ok(())
}

/// Reposition/resize the dock webview to track the frontend placeholder rect.
pub fn set_bounds(app: &AppHandle, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        let _ = wv.set_position(LogicalPosition::new(x, y));
        let _ = wv.set_size(LogicalSize::new(w, h));
    }
    Ok(())
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.show().map_err(|e| format!("show: {e}"))?;
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.hide().map_err(|e| format!("hide: {e}"))?;
    }
    Ok(())
}

/// The dock webview's current URL (follows redirects/navigation). Empty if closed.
pub fn current_url(app: &AppHandle) -> Result<String, String> {
    match app.get_webview(LABEL) {
        Some(wv) => Ok(wv.url().map(|u| u.to_string()).unwrap_or_default()),
        None => Ok(String::new()),
    }
}

/// Destroy the dock webview. Safe to call when not open.
pub fn close(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.close().map_err(|e| format!("close: {e}"))?;
    }
    Ok(())
}
