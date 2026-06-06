# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.6.0 — 2026-06-06 — feat: in-app browser dock + harness / model-picker polish

> **Why.** Rift gains a browser it can *see*. A dockable web panel lets you browse inside the app and hand the page you're on straight to the assistant — including authenticated, JS-rendered pages that `WebFetch` can't reach. Plus a round of harness, model-picker, and onboarding polish, and the fixes batched since v0.5.0.

**In-app browser dock (Ctrl+Shift+B).** A resizable web panel beside the chat. The omnibox takes a URL or a search query (a dotted, space-free host → `https://`; anything else → a DuckDuckGo search). Back / forward / true in-place reload (`location.reload()` — no duplicate history entry); a loading spinner tracks *every* navigation (link clicks, redirects, back/forward) via a native `on_page_load` event, with a 20s watchdog fallback. **Add to chat** pulls the page's *rendered* `innerText` (post-JS, authenticated since the dock holds the session) into the composer as a labelled context block — exactly what a server-side fetch can't get. A `⋯` menu carries Copy URL + Open in system browser; **Ctrl+L** focuses and selects the address bar. The native child webview paints in the app's dark surface color, killing the pre-first-paint flash. Files: `browser/mod.rs`, `commands/browser.rs`, `WebBrowserPage.svelte`, `browserDock.svelte.ts`.

**Harness — one viewport, no scroll.** The telemetry workspace was redesigned around a single KPI rail (cost / turns / tools / tok-s / cache / ttfp), with reliability, session details, granted tools, and the live stream tucked behind a "Show details" toggle — the whole dashboard fits one screen.

**Model picker — capability matrix.** Per-model effort stops with an auto-clamp: Haiku hides the effort slider and shows a no-effort caption; Ultracode gets an amber caption. Fast-mode stays hidden behind a wiring flag until the CLI side lands, so no control lies about what it does.

**Onboarding + reading polish.** Everyone — authed users included — now hits a final beta-notice step before working. Markdown answers reveal with a blur-in as they stream; harness timelines mark dead-wait gaps.

**Batched fixes (#31–#36).** Cosmetic fast-mode toggle hidden until wired · `\\?\` extended-path prefix stripped from the Home workspace path · harness "avg dead wait" backfills for older session logs · command palette gained "Go to Home" · deleted recent folders can be Forgotten (×) · Settings scroll-spy now lights the last section ("About") at the scroll bottom.

**Cleanup + release.** Removed dead keychain helpers left from the SFTP/sync rip (`bridge_token_key`, `rcon_password_key`) + idiomatic clippy fixes. The release now publishes **Setup.exe only** — the redundant portable zip is dropped so a new user has one obvious download.

**Verify.** `cargo check` 0/0 · `cargo clippy` clean (1 intentional arity warning) · `npm run check` 0/0 (4062 files) · browser dock + the per-turn MCP tool pipeline CDP-verified live end-to-end.

## v0.5.0 — 2026-06-04 — feat: Harness telemetry workspace + Steer (mid-turn redirect)

Harness telemetry workspace (Ctrl+3) + mid-turn Steer (Alt+Enter injects into the live CLI's stdin so a turn course-corrects without a restart). Detail in `git log -- docs/CHANGELOG.md`.

_Older entries (v0.4.48 and earlier) live in `git log -- docs/CHANGELOG.md`._
