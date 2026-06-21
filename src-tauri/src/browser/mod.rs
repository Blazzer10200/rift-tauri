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

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, WebviewBuilder, WebviewUrl};

/// Label of the embedded webview child. Single instance — one browser dock.
const LABEL: &str = "rift-browser";
/// Host window the dock lives in.
const HOST_WINDOW: &str = "main";

pub(crate) fn parse_url(raw: &str) -> Result<Url, String> {
    let u = Url::parse(raw).map_err(|e| format!("invalid URL '{raw}': {e}"))?;
    // Scheme allowlist: only real web navigation. `file://` would expose the
    // local disk, `javascript:`/`data:` would execute attacker-controlled script
    // in the embedded webview — both reachable from AI-generated links.
    match u.scheme() {
        "http" | "https" => Ok(u),
        // `about:` is allowed ONLY for the exact blank page. Broader `about:`
        // targets (`about:srcdoc`, `about:newtab`, …) can render attacker HTML
        // or privileged browser UI in the embedded webview.
        "about" if u.as_str() == "about:blank" => Ok(u),
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
    // Emit page-load phase + URL to the frontend so the address bar + loading
    // spinner track real navigations (link clicks, redirects, back/forward),
    // not just the explicit `go()` path. Fires for the webview's whole lifetime.
    let builder = WebviewBuilder::new(LABEL, WebviewUrl::External(u.clone()))
        // Match the app's page bg (oklch(0.142) → #090a0b) so the pre-first-paint
        // flash reads as the app's dark surface, not a white/black blast.
        .background_color(tauri::webview::Color(9, 10, 11, 255))
        .on_page_load(|webview, payload| {
            let phase = match payload.event() {
                tauri::webview::PageLoadEvent::Started => "started",
                tauri::webview::PageLoadEvent::Finished => "finished",
            };
            // emit_to HOST_WINDOW, not a global emit: the dock only lives on
            // `main`, so a global broadcast drives a second window's address bar
            // + spinner off navigations it doesn't own (multi-window state bleed).
            let _ = webview.app_handle().emit_to(
                HOST_WINDOW,
                "browser://load",
                serde_json::json!({ "phase": phase, "url": payload.url().to_string() }),
            );
        });
    let wv = window
        .add_child(
            builder,
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

/// History navigation in the dock webview. Fire-and-forget — no-op at history
/// ends (the browser ignores `history.back()` with no prior entry).
pub fn go_back(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.eval("history.back()").map_err(|e| format!("back: {e}"))?;
    }
    Ok(())
}

pub fn go_forward(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.eval("history.forward()").map_err(|e| format!("forward: {e}"))?;
    }
    Ok(())
}

/// True reload of the dock page. `location.reload()` re-fetches in place —
/// unlike a fresh `navigate()` to the same URL, it preserves history (no
/// duplicate Back entry) and re-runs SPA/hash state.
pub fn reload(app: &AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview(LABEL) {
        wv.eval("location.reload()").map_err(|e| format!("reload: {e}"))?;
    }
    Ok(())
}

/// Readable snapshot of the dock page — feeds "Add page to chat". Captures the
/// *rendered* `innerText` (post-JS, and authenticated since the webview holds
/// the session), which is exactly what `WebFetch` can't reach.
#[derive(Serialize)]
pub struct PageContent {
    pub title: String,
    pub url: String,
    pub text: String,
    /// True when the page exceeded the extraction cap and `text` is a prefix.
    pub truncated: bool,
    /// Full `innerText` length before capping (chars).
    pub full_len: u64,
}

/// Extraction cap (chars). ~10k tokens — a hefty doc page — while keeping the
/// injected composer block from ballooning.
const EXTRACT_CAP: usize = 40_000;

/// Pull the dock page's rendered text via `eval_with_callback` (the result is
/// JSON-serialized by wry and handed to the callback). Returns an error when no
/// page is open or the webview doesn't answer within the timeout.
pub async fn read_page(app: &AppHandle) -> Result<PageContent, String> {
    let wv = app
        .get_webview(LABEL)
        .ok_or("no page open — enter a URL first")?;

    // IIFE returns an object; wry serializes it to a JSON string for the
    // callback. Capping happens in-page so we never marshal a huge string.
    let js = format!(
        r#"(function(){{
            var b = document.body ? document.body.innerText : "";
            var full = b.length;
            var cap = {EXTRACT_CAP};
            return {{
                title: document.title || "",
                text: full > cap ? b.slice(0, cap) : b,
                truncated: full > cap,
                fullLen: full
            }};
        }})()"#
    );

    // `eval_with_callback` wants `Fn` (may be retained), but a oneshot sender is
    // single-use — guard it in a Mutex<Option<_>> and take on first fire.
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let slot = Mutex::new(Some(tx));
    wv.eval_with_callback(js, move |json| {
        if let Ok(mut g) = slot.lock() {
            if let Some(tx) = g.take() {
                let _ = tx.send(json);
            }
        }
    })
    .map_err(|e| format!("eval: {e}"))?;

    let json = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| "timed out reading page".to_string())?
        .map_err(|_| "page read was cancelled".to_string())?;

    #[derive(serde::Deserialize)]
    struct Raw {
        title: String,
        text: String,
        truncated: bool,
        #[serde(rename = "fullLen")]
        full_len: u64,
    }
    let raw: Raw =
        serde_json::from_str(&json).map_err(|e| format!("decode page snapshot: {e}"))?;
    // Use the webview-reported URL (trusted navigation state), NOT the page's
    // own `location.href` — a hostile page can spoof the latter to a
    // `javascript:`/`data:` string that downstream consumers might treat as a
    // real navigable URL.
    let url = wv.url().map(|u| u.to_string()).unwrap_or_default();
    Ok(PageContent {
        title: raw.title,
        url,
        text: raw.text,
        truncated: raw.truncated,
        full_len: raw.full_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_allows_only_web_schemes() {
        assert!(parse_url("https://example.com").is_ok());
        assert!(parse_url("http://example.com/path?q=1").is_ok());
        assert!(parse_url("about:blank").is_ok());
    }

    #[test]
    fn parse_url_blocks_dangerous_schemes() {
        // file:// (disk exposure), javascript:/data: (script execution in the
        // embedded webview) — all reachable from AI-generated links, all blocked.
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
        ] {
            assert!(parse_url(bad).is_err(), "{bad} should be blocked");
        }
        // Not a URL at all → parse error, never silently accepted.
        assert!(parse_url("not a url").is_err());
    }
}
