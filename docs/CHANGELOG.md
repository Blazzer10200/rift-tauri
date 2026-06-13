# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.1 — 2026-06-13 — UI polish arc (cleaner, less sloppy, more organized)

> **Why.** Post-buddy-release tightening pass — finish and polish what's already there, not new features. Six user-facing fixes across the assistant UI. Frontend-only; backend identical to v0.9.0.

**Fixed:**
- **Live token counter** now actually climbs mid-turn. It sat frozen because the CLI streams no incremental output-token counts (just `1` at the start, the final total at the end); rewritten to climb from a streamed-character estimate and snap to the exact count at each message boundary — spinner line + composer pill.
- **Notifications consolidated** — ~22 confirmation/error/status messages that fought over one banner slot now stack as severity-tuned toasts. Workspace paths are prettified (no more `\\?\C:\…`); Settings errors that silently went to an off-screen composer banner are now visible toasts. The inline banner is reserved for multi-line slash output (`/tools`, `/help`, `/stats`).
- **Image lightbox** — clicking an image opens an in-app full-screen overlay (click anywhere / Esc to close) instead of the dead `window.open` (a no-op in WebView2).
- **Drag-and-drop** — a file dropped outside the composer no longer makes the window navigate away; stray image drops attach to the active chat, and non-image files get a clear toast instead of vanishing silently.
- **Activity panel** — decluttered: deduped triplicated timestamps (verb / duration / "ago" each have one home), quoted unreadable raw-regex search targets, category-coloured step icons, session cost promoted to its own line, live token count in the Now strip.
- **Streaming latency** — tightened the two stacked text pacers so rendered text keeps closer to live on fast bursts; the signature blur reveal is unchanged.

**Verify.** svelte-check 0/0 · vitest 129/129 · backend unchanged from v0.9.0 (frontend-only) · CDP-verified each fix against the live dev UI.

## Older versions

v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
