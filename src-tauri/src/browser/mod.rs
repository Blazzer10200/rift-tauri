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

use std::sync::atomic::{AtomicU64, Ordering};
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
        other => {
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::Log,
                crate::diagnostics::DiagLevel::Warn,
                Some("browser"),
                Some(file!()),
                "blocked non-web URL scheme",
                serde_json::json!({ "scheme": other }),
            );
            Err(format!("blocked URL scheme '{other}:' — only http/https are allowed"))
        }
    }
}

/// The (host, port) to TCP-probe when `u` targets a loopback host; None for
/// anything else (remote hosts are never probed — extra latency + an extra
/// pre-connection for no benefit).
fn loopback_target(u: &Url) -> Option<(String, u16)> {
    let raw = u.host_str()?;
    // `host_str` keeps IPv6 brackets ("[::1]") — strip for parse/connect.
    let host = raw.trim_start_matches('[').trim_end_matches(']');
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host.parse::<std::net::IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false);
    if !is_loopback {
        return None;
    }
    Some((host.to_string(), u.port_or_known_default()?))
}

/// Monotonic navigation generation — a newer `open_probed` supersedes any
/// older one still parked in its loopback probe, so a stale probed URL can't
/// clobber a navigation the user issued in the meantime.
static OPEN_GEN: AtomicU64 = AtomicU64::new(0);

/// Serializes the probe's final generation check + `open()` against the
/// OPEN_GEN bumps in `hide`/`close`. Without it a dismiss landing between the
/// generation load and `open()`'s `show()` is silently undone (check-then-act
/// gap in the very mechanism meant to prevent that reopen).
static NAV_GATE: Mutex<()> = Mutex::new(());

fn nav_gate() -> std::sync::MutexGuard<'static, ()> {
    match NAV_GATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Dev-server race: the assistant opens http://localhost:5173 moments after
/// spawning the dev server — navigating immediately lands on WebView2's
/// connection-refused page. For loopback targets, wait (bounded, ~8s) for the
/// port to accept a TCP connection before navigating. Fail-open: when the
/// window expires, navigation proceeds and the error page is the honest
/// outcome.
pub async fn open_probed(app: &AppHandle, url: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    let u = parse_url(url)?;
    let generation = OPEN_GEN.fetch_add(1, Ordering::AcqRel) + 1;
    if let Some((host, port)) = loopback_target(&u) {
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        loop {
            match tokio::time::timeout(
                Duration::from_millis(600),
                tokio::net::TcpStream::connect((host.as_str(), port)),
            )
            .await
            {
                Ok(Ok(_)) => break,
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                crate::diagnostics::emit_with_fields(
                    crate::diagnostics::DiagStage::Log,
                    crate::diagnostics::DiagLevel::Info,
                    Some("browser"),
                    Some(file!()),
                    "loopback probe timed out — navigating anyway",
                    serde_json::json!({ "host": host, "port": port }),
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    let _gate = nav_gate();
    if OPEN_GEN.load(Ordering::Acquire) != generation {
        return Ok(()); // superseded by a newer navigation/dismiss while probing
    }
    open(app, url, x, y, w, h)
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
        // Console capture rides every document this webview ever loads —
        // read_console / console_counts drain the buffer it maintains.
        .initialization_script(CONSOLE_CAPTURE_JS)
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
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                spawn_icon_probe(webview.app_handle().clone());
            }
        });
    let wv = match window.add_child(
        builder,
        LogicalPosition::new(x, y),
        LogicalSize::new(w, h),
    ) {
        Ok(wv) => wv,
        // TOCTOU: a concurrent open() may have created the webview between our
        // get_webview check above and this add_child. Duplicate-label is then
        // not a failure — adopt the existing webview and reposition, matching
        // the early-return idempotent path. (RR10)
        Err(e) => match app.get_webview(LABEL) {
            Some(wv) => {
                wv.navigate(u).map_err(|e| format!("navigate: {e}"))?;
                let _ = wv.set_position(LogicalPosition::new(x, y));
                let _ = wv.set_size(LogicalSize::new(w, h));
                let _ = wv.show();
                return Ok(());
            }
            None => {
                crate::diagnostics::emit_with_fields(
                    crate::diagnostics::DiagStage::Log,
                    crate::diagnostics::DiagLevel::Warn,
                    Some("browser"),
                    Some(file!()),
                    "embedded webview create failed",
                    serde_json::json!({
                        "scheme": u.scheme(),
                        "host": u.host_str().unwrap_or(""),
                        "error": e.to_string(),
                    }),
                );
                return Err(format!("create embedded webview: {e}"));
            }
        },
    };
    // The builder's initial External URL doesn't reliably load on the child
    // webview (WebView2 starts it at about:blank); navigate explicitly.
    wv.navigate(u).map_err(|e| format!("navigate: {e}"))?;
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
    // A hide supersedes any `open_probed` still parked in its loopback probe —
    // without this the probe resolves up to 8s later and open()'s unconditional
    // show() reopens the dock the user just dismissed. The gate makes the bump
    // atomic with the probe's final check+open.
    let _gate = nav_gate();
    OPEN_GEN.fetch_add(1, Ordering::AcqRel);
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
    // Same probe-supersede rule as hide() — a parked open_probed must not
    // recreate the dock after an explicit close.
    let _gate = nav_gate();
    OPEN_GEN.fetch_add(1, Ordering::AcqRel);
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

    let json = eval_json(&wv, &js, Duration::from_secs(5)).await?;

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
    // RR8 + RR11 both live in current_dock_url — see its doc.
    let url = current_dock_url(app);
    Ok(PageContent {
        title: raw.title.chars().take(1024).collect(),
        url,
        text: raw.text,
        truncated: raw.truncated,
        full_len: raw.full_len,
    })
}

/// Run `js` in the dock webview and await its JSON-serialized result (wry
/// hands `eval_with_callback` a JSON string). The callback wants `Fn` (may be
/// retained), but a oneshot sender is single-use — guarded in a
/// Mutex<Option<_>>, taken on first fire. Shared by read_page, the console
/// readers, and the favicon probe.
async fn eval_json(wv: &tauri::Webview, js: &str, timeout: Duration) -> Result<String, String> {
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
    tokio::time::timeout(timeout, rx)
        .await
        .map_err(|_| "timed out waiting for the page to answer".to_string())?
        .map_err(|_| "page eval was cancelled".to_string())
}

/// The dock webview's URL as reported by the webview itself (trusted
/// navigation state — a hostile page can spoof `location.href` to a
/// `javascript:`/`data:` string, so never read it from page JS). Re-fetches
/// the webview by label so a close+recreate during a caller's prior await
/// can't leave a stale handle (RR11). RR8: capped at 2048 chars — a hostile
/// page can drive navigation to a multi-megabyte data:/blob: URL that would
/// bloat every snapshot consumer (frontend, prompt context).
fn current_dock_url(app: &AppHandle) -> String {
    app.get_webview(LABEL)
        .and_then(|w| w.url().ok())
        .map(|u| u.to_string())
        .map(|s| {
            if s.len() > 2048 {
                let mut end = 2048;
                while !s.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}…", &s[..end])
            } else {
                s
            }
        })
        .unwrap_or_default()
}

// ---- Console capture -------------------------------------------------------

/// Injected at document creation on every navigation (main frame). Wraps
/// console.* + uncaught errors + unhandled rejections into a capped ring
/// buffer that the drain/counts evals read. This is page-world JS — a hostile
/// page can tamper with its own buffer — so the drain script AND the Rust
/// side both re-cap and re-validate everything on the way out.
const CONSOLE_CAPTURE_JS: &str = r#"(function () {
  if (window.__riftConsole) return;
  var st = { buf: [], dropped: 0 };
  window.__riftConsole = st;
  var MAX = 300, MAXLEN = 2000;
  function push(level, text) {
    try {
      text = String(text);
      if (text.length > MAXLEN) text = text.slice(0, MAXLEN) + "…";
      if (st.buf.length >= MAX) { st.buf.shift(); st.dropped++; }
      st.buf.push({ level: level, text: text, ts: Date.now() });
    } catch (e) {}
  }
  function fmt(args) {
    var out = [];
    for (var i = 0; i < args.length; i++) {
      var a = args[i];
      if (typeof a === "string") { out.push(a); continue; }
      if (a instanceof Error) { out.push(a.stack || String(a)); continue; }
      try { out.push(JSON.stringify(a)); } catch (e) { out.push(String(a)); }
    }
    return out.join(" ");
  }
  ["log", "info", "warn", "error", "debug"].forEach(function (level) {
    var orig = console[level];
    console[level] = function () {
      push(level, fmt(arguments));
      if (orig) { try { orig.apply(console, arguments); } catch (e) {} }
    };
  });
  window.addEventListener("error", function (e) {
    var loc = e.filename ? " (" + e.filename + ":" + e.lineno + ")" : "";
    push("error", (e.message || "uncaught error") + loc);
  });
  window.addEventListener("unhandledrejection", function (e) {
    var r = e.reason, msg;
    try { msg = r && r.stack ? String(r.stack) : String(r); } catch (err) { msg = "(unserializable)"; }
    push("error", "Unhandled promise rejection: " + msg);
  });
})();"#;

/// Non-destructive drain of the capture buffer. Re-caps entry count + text
/// length in-page so a tampered buffer can't marshal an unbounded payload.
const CONSOLE_DRAIN_JS: &str = r#"(function () {
  var out = [], dropped = 0;
  try {
    var st = window.__riftConsole;
    if (st && st.buf) {
      dropped = Math.max(0, Math.floor(Number(st.dropped) || 0));
      var start = Math.max(0, st.buf.length - 300);
      dropped += start;
      for (var i = start; i < st.buf.length; i++) {
        var x = st.buf[i] || {};
        out.push({ level: String(x.level).slice(0, 8), text: String(x.text).slice(0, 2000), ts: Number(x.ts) || 0 });
      }
    }
  } catch (e) {}
  return { entries: out, dropped: dropped };
})()"#;

/// Error/warning tally only — polled by the frontend badge every ~1.2s, so it
/// must stay far lighter than a full drain.
const CONSOLE_COUNTS_JS: &str = r#"(function () {
  var errors = 0, warns = 0;
  try {
    var st = window.__riftConsole;
    if (st && st.buf) {
      for (var i = 0; i < st.buf.length; i++) {
        var l = st.buf[i] && st.buf[i].level;
        if (l === "error") errors++; else if (l === "warn") warns++;
      }
    }
  } catch (e) {}
  return { errors: errors, warns: warns };
})()"#;

#[derive(Serialize, serde::Deserialize)]
pub struct ConsoleEntry {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub ts: f64,
}

#[derive(Serialize)]
pub struct ConsoleSnapshot {
    pub url: String,
    pub entries: Vec<ConsoleEntry>,
    /// Entries evicted by the ring buffer before this read.
    pub dropped: u64,
}

#[derive(Serialize)]
pub struct ConsoleCounts {
    pub errors: u64,
    pub warns: u64,
}

/// Console messages logged by the current dock page since it loaded (the
/// capture buffer resets per document). Feeds both the `read_browser_console`
/// MCP tool and the dock's "add console to chat" action.
pub async fn read_console(app: &AppHandle) -> Result<ConsoleSnapshot, String> {
    let wv = app
        .get_webview(LABEL)
        .ok_or("no page open — enter a URL first")?;
    let json = eval_json(&wv, CONSOLE_DRAIN_JS, Duration::from_secs(5)).await?;
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        entries: Vec<ConsoleEntry>,
        #[serde(default)]
        dropped: f64,
    }
    let raw: Raw =
        serde_json::from_str(&json).map_err(|e| format!("decode console snapshot: {e}"))?;
    // Page-world data — re-validate levels + re-cap counts/text server-side.
    let entries = raw
        .entries
        .into_iter()
        .take(300)
        .map(|mut e| {
            if !matches!(e.level.as_str(), "log" | "info" | "warn" | "error" | "debug") {
                e.level = "log".into();
            }
            if e.text.chars().count() > 2000 {
                e.text = e.text.chars().take(2000).collect();
            }
            e
        })
        .collect();
    Ok(ConsoleSnapshot {
        url: current_dock_url(app),
        entries,
        dropped: raw.dropped.max(0.0) as u64,
    })
}

/// Error/warning tally for the dock badge. A closed dock is zeros, not an
/// error — the frontend polls this blindly.
pub async fn console_counts(app: &AppHandle) -> Result<ConsoleCounts, String> {
    let Some(wv) = app.get_webview(LABEL) else {
        return Ok(ConsoleCounts { errors: 0, warns: 0 });
    };
    let json = eval_json(&wv, CONSOLE_COUNTS_JS, Duration::from_secs(3)).await?;
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        errors: f64,
        #[serde(default)]
        warns: f64,
    }
    let raw: Raw = serde_json::from_str(&json).map_err(|e| format!("decode console counts: {e}"))?;
    Ok(ConsoleCounts { errors: raw.errors.max(0.0) as u64, warns: raw.warns.max(0.0) as u64 })
}

// ---- Favicon ----------------------------------------------------------------

/// The page's declared icon (or origin /favicon.ico fallback), resolved
/// relative to the document so relative hrefs work.
const ICON_PROBE_JS: &str = r#"(function () {
  try {
    var sel = "link[rel~='icon'], link[rel='shortcut icon'], link[rel='apple-touch-icon']";
    var l = document.querySelector(sel);
    var href = l && l.href ? String(l.href) : new URL("/favicon.ico", location.href).href;
    return href.slice(0, 65536);
  } catch (e) { return ""; }
})()"#;

/// Cap on a fetched favicon body (or an inline data: URI). Generous for any
/// real icon while bounding what a hostile page can push at the UI.
const ICON_MAX_BYTES: usize = 256 * 1024;

/// After a page finishes loading, resolve its favicon to a `data:` URI and
/// emit `browser://icon`. The app webview's CSP (`img-src 'self' … data:`)
/// blocks external image URLs, so the bytes must arrive inline: a page's
/// inline data: icon passes through, an http(s) href is fetched here (same
/// host the user is already browsing — never a third-party favicon service,
/// which would leak browse history). Fire-and-forget: any failure just means
/// the address bar keeps its fallback glyph.
fn spawn_icon_probe(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Some(wv) = app.get_webview(LABEL) else { return };
        let Ok(json) = eval_json(&wv, ICON_PROBE_JS, Duration::from_secs(4)).await else {
            return;
        };
        let Ok(serde_json::Value::String(href)) = serde_json::from_str(&json) else { return };
        if let Some(data_uri) = resolve_icon(&href).await {
            let _ = app.emit_to(
                HOST_WINDOW,
                "browser://icon",
                serde_json::json!({ "page": current_dock_url(&app), "icon": data_uri }),
            );
        }
    });
}

/// Icon href → `data:` URI the CSP will render, or None to keep the fallback
/// glyph. data:image/* passes through; http(s) is fetched (5s timeout, size
/// cap, image content-type or magic-byte sniff); every other scheme is out.
async fn resolve_icon(href: &str) -> Option<String> {
    if href.is_empty() || href.len() > ICON_MAX_BYTES {
        return None;
    }
    if href.starts_with("data:image/") {
        return Some(href.to_string());
    }
    if !href.starts_with("http://") && !href.starts_with("https://") {
        return None;
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(href).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    // favicon.ico served as octet-stream (or with no type at all) is everywhere
    // — fall through to the magic-byte sniff rather than rejecting.
    if !(ct.starts_with("image/") || ct == "application/octet-stream" || ct.is_empty()) {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > ICON_MAX_BYTES {
        return None;
    }
    let mime: String = if ct.starts_with("image/") {
        ct
    } else {
        sniff_image_mime(&bytes)?.to_string()
    };
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{mime};base64,{b64}"))
}

/// Magic-byte sniff for icon bodies served without an image/* content-type.
/// None = not a recognizable image, don't render it.
fn sniff_image_mime(b: &[u8]) -> Option<&'static str> {
    if b.len() >= 4 && b[..4] == [0x00, 0x00, 0x01, 0x00] {
        return Some("image/x-icon");
    }
    if b.len() >= 8 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some("image/png");
    }
    if b.len() >= 3 && b[..3] == [0xFF, 0xD8, 0xFF] {
        return Some("image/jpeg");
    }
    if b.len() >= 6 && (&b[..6] == b"GIF87a" || &b[..6] == b"GIF89a") {
        return Some("image/gif");
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if let Ok(s) = std::str::from_utf8(&b[..b.len().min(256)]) {
        if s.trim_start().starts_with("<svg") || s.contains("<svg") {
            return Some("image/svg+xml");
        }
    }
    None
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
    fn loopback_target_detects_dev_hosts() {
        let t = |s: &str| loopback_target(&Url::parse(s).unwrap());
        assert_eq!(t("http://localhost:5173/x"), Some(("localhost".into(), 5173)));
        assert_eq!(t("http://127.0.0.1:3000"), Some(("127.0.0.1".into(), 3000)));
        assert_eq!(t("http://[::1]:8080"), Some(("::1".into(), 8080)));
        assert_eq!(t("http://localhost"), Some(("localhost".into(), 80)));
        assert_eq!(t("https://example.com"), None);
        assert_eq!(t("http://192.168.1.10:3000"), None);
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

    #[test]
    fn sniff_image_mime_detects_common_formats() {
        assert_eq!(sniff_image_mime(&[0x00, 0x00, 0x01, 0x00, 0x01]), Some("image/x-icon"));
        assert_eq!(
            sniff_image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
            Some("image/png")
        );
        assert_eq!(sniff_image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff_image_mime(b"GIF89a......"), Some("image/gif"));
        assert_eq!(sniff_image_mime(b"RIFF\x00\x00\x00\x00WEBP"), Some("image/webp"));
        assert_eq!(sniff_image_mime(b"<svg xmlns='x'/>"), Some("image/svg+xml"));
        assert_eq!(sniff_image_mime(b"MZ not an image"), None);
        assert_eq!(sniff_image_mime(b""), None);
    }

    #[tokio::test]
    async fn resolve_icon_passes_data_uris_and_rejects_junk() {
        // Inline data: icons pass through untouched (CSP renders them as-is).
        let ok = resolve_icon("data:image/png;base64,iVBORw0KGgo=").await;
        assert_eq!(ok.as_deref(), Some("data:image/png;base64,iVBORw0KGgo="));
        // Non-image / non-web schemes never reach a fetch.
        assert!(resolve_icon("javascript:alert(1)").await.is_none());
        assert!(resolve_icon("file:///c/icon.png").await.is_none());
        assert!(resolve_icon("data:text/html,<script>1</script>").await.is_none());
        assert!(resolve_icon("").await.is_none());
        // Oversized inline icon is dropped, not truncated into a broken image.
        let huge = format!("data:image/png;base64,{}", "A".repeat(ICON_MAX_BYTES));
        assert!(resolve_icon(&huge).await.is_none());
    }
}
