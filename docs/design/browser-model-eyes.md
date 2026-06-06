# In-app browser — "the model's eyes"

> Status: **Slice 1 IMPLEMENTED 2026-06-05 (cont.59), live-verified via CDP, UNSHIPPED**.
> `cargo check` 0/0 (isolated target), `svelte-check` 0/0, end-to-end tested in
> the running dev app (read_page → composer on example.com; back/forward;
> omnibox search-vs-URL). Rides the next `/git-ship` with the cont.52–58 batch.

---

## 1. Direction

The browser dock was an island: a manual address bar + native child WebView2,
with **zero** connection to the assistant. The chosen direction is to make it a
**shared perception surface** — the model reads, and eventually acts on, the
exact page you're looking at. Conventional chrome (tabs/bookmarks/history) is
explicitly *not* the goal; the AI integration is.

**Why an embedded browser beats the model's own `WebFetch`:** `WebFetch` gets
raw pre-JS HTML and can't touch authenticated pages. The dock renders the page
(post-JS) and *is* you (logged in). So "read what I'm looking at" reaches
SPA docs, private dashboards, and gated content `WebFetch` structurally cannot.

## 2. Slice 1 — what shipped (this arc)

- **`read_page` → "Add to chat"** (the unlock). A toolbar button extracts the
  rendered `document.body.innerText` and drops it into the composer as a
  labelled, editable block: `[Page context: <title> — <url>] … [End page context]`.
  Capped at 40k chars with a truncation note. User-triggered + visible =
  full control, no surprise context.
- **History nav** (back/forward) + **omnibox** address bar (dotted token → URL,
  else → DuckDuckGo search).
- Button success/fail flash lives on the toolbar (the native webview floats over
  the stage and would hide stage-level feedback).

## 3. Architecture decisions

- **Extraction = `Webview::eval_with_callback`** (Tauri 2.11). The JS result
  returns as a JSON string to the callback; we await it over a oneshot channel
  with a 5s timeout. Chosen over `webview2-com`/`ICoreWebView2::ExecuteScript`
  FFI — no platform code, no `windows`-crate version skew, cross-platform.
- **It's a Tauri command, not an MCP tool — on purpose.** The MCP server
  (`mcp_server.rs`) runs as a *separate process* (`RIFT_MCP_SERVER=1`, stdio)
  with no `AppHandle`, so it cannot touch the live webview. A true autonomous
  `read_page`/`open_url` MCP tool needs a cross-process bridge back into the main
  app — deferred to a later slice (see §4).
- **Surface:** `browser/mod.rs` (`read_page`, `go_back`, `go_forward`) →
  `commands/browser.rs` wrappers → registered in `lib.rs` → driven from
  `WebBrowserPage.svelte`; composer injection via `assistant.composerDraft`.

## 4. Next slices (not built)

1. **Selection → chat** — highlight text in the dock, one gesture sends it.
2. **MCP `open_url` / `read_page`** — autonomous model access. Requires the
   main-app bridge (localhost IPC + the existing mcp-config bridge-token
   pattern). The model opens cited links in the dock and reads them back.
3. **Lightweight agentic** — model navigates/clicks for multi-step lookups
   (computer-use style), only after 1–2 prove out.

## 5. Gotcha logged this session

Running `npm run check` (→ `svelte-kit sync`) **while `tauri dev` is live**
briefly destabilized Vite; WebView2 cached one corrupt module response to disk
(`EBWebView-Dev`), which survived a full dev restart and showed as a stuck
`500 Internal Error` (curl looked healthy — it bypasses the cache). Fixed via
CDP `Network.clearBrowserCache` + cache-bypassing reload. **Never `/check`
against a running tauri-dev** (already a CLAUDE.md rule — confirmed the hard way).
