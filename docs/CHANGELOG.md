# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.21 — 2026-06-11 — self-aware Rift: assistant drives the app UI + sees app state

> **Why.** The assistant lived inside Rift but couldn't touch it or see it: it printed localhost links the user had to open elsewhere, had no way to flag attention, and — the real find — was being *steered to a tool that didn't exist*: `mcp__rift__ask_user` had been unregistered since the pure-assistant conversion ripped the loopback bridge, while the deny-handler, allowlist, and the entire frontend card UI all still pointed at it.

- **Loopback UI bridge resurrected** (`assistant/bridge.rs`, new): NDJSON over `127.0.0.1:<random>` with a per-launch token, parent ↔ MCP-child, UI-presentation ops only. Bound at boot; `write_mcp_config` injects `RIFT_BRIDGE_PORT/TOKEN` per turn (missing env → tools degrade to unlisted, never error).
- **`mcp__rift__ask_user` works again** — interactive multiple-choice card in the chat, 10-min answer window, dismiss → clean `cancelled` fallback. The dormant frontend stack (FIFO binding, card, `assistant_answer_ask_user`) needed zero changes.
- **`mcp__rift__open_browser`** (new): the model shows any http/https page in the in-app browser dock — dev-server previews land next to the chat instead of as a bare link. Scheme-allowlisted in the bridge; frontend owns dock visibility + stage rect.
- **`mcp__rift__notify`** (new): toast in Rift's corner (severity-allowlisted, length-capped) for "long work finished / needs attention" moments.
- **Per-turn "Rift environment snapshot"** rides the user message (cache-prefix stays stable): browser dock's current page + live plan-usage gauges (5-hour/weekly %) via a cache-only read (`limits::cached_snapshot`, ≤5 min) — zero TTFT cost; a fire-and-forget background refresh keeps it warm. The system addendum teaches all three tools + snapshot semantics; ghost sync-tool entries purged from the allowlist.
- **Localhost links open in-app**: clicking a `localhost`/`127.0.0.1` link in chat markdown routes to the browser dock, not the system browser.
- Deps: `rand 0.10` + `base64 0.22` re-added (bridge token + request-id generation).

**How to verify.** In a chat with a workspace open: ask the model to "open http://localhost:&lt;port&gt; in the browser" → dock opens itself; ask it to "notify me when done" → corner toast; ask it something ambiguous → interactive question card instead of a wall of text. Click any localhost link in a reply → opens in the dock.

**Verify.** svelte-check 0/0 (4093 files) · cargo check clean (zero warnings, forced recheck) · live CDP pass with real turns: open_browser opened + navigated the dock, warn toast pixel-confirmed, ask_user full round-trip (card → Yes → model echoed "Yes"), link intercept with 0 console errors, session JSONL carried the snapshot ("dock open at …; 5-hour window 38% used").

## Older versions

v0.8.20 live plan limits — cost-cockpit "Plan limits" card + `/usage` popover via undocumented OAuth usage endpoint (CLI token read-only, 60s cache) · v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping, slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips + `turn.rs` overlapping-turn registry race fix · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8) · v0.8.15 hot-file splits + honest Settings update chip · v0.8.14 update-dialog render crash fix + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model · v0.8.12 pill `×` = 24h snooze · v0.8.11 Settings redesign + Harness one-viewport · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
