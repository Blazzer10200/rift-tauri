# rift-tauri — Handoff Archive

> Retired session entries from `docs/HANDOFF.md`. Newest first. Pre-archive history also available via `git log -- docs/HANDOFF.md`.

## Session 90 — 2026-05-17 — v0.4.4-alpha source ship + stress-test fix-ups

Autonomous CDP-driven stress test across every UI surface: ActivityBar (Ctrl+1..7 / Ctrl+0 / drag-reorder + persistence), chat tabs (Ctrl+T/W, Ctrl+Tab, Alt+1..9), right-pane × 7 (lazy-mount latch + width clamp 320..1200 + dblclick-snap), Settings × 7 sections (v0.3↔v0.2 shell round-trip, all STT/Assistant/Terminal toggles, language picker, diagnostic copy), Sync (drift scanner caught 1 pull in `[endure]`), Files (TwoPane nav + remote ctx menu), Terminal (PTY echo verified), Velopack ("Up to date" vs released 0.4.3). Status bar `isHandshaking` invariant held across reconnect.

Two latent UX bugs caught + fixed: (a) `right-pane.svelte.ts::init()` clamped state.width but didn't re-persist, so OOB localStorage values survived launches — now writes back. (b) Composer mic button rendered unconditionally; clicking it with STT disabled silently set `stt.lastError`. Gated on `stt.config.enabled && stt.supported` w/ `onMount(() => stt.init())` so the gate reflects backend config without a Settings visit. Tooling: `scripts/cdp/serve.cjs` `KEY_DEFS` gained Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight (drives `Ctrl+,`, `Ctrl+\`` directly).

Bumped 3-file version 0.4.3 → 0.4.4-alpha; Cargo.lock auto-synced. CHANGELOG v0.4.4 extended w/ S90 fix-ups. Source committed + pushed.

## Session 89 — 2026-05-17 — TTS rollback + workspace clean-out

TTS reversed (only STT wanted): removed `src-tauri/src/tts/`, `tts.svelte.ts`, speaker UI, `msedge-tts` chain. Settings `Voice` → `Speech` (id `"speech"`). 6 dead npm deps + stale `cdp/smoke-v04.sh` + orphan branches dropped. svelte-check **0/0**. Folded into v0.4.4-alpha (S90).

## Sessions S57–S68 (one-liners, retired 2026-05-16)

- **S68** — v0.2.56-alpha SHIPPED (`687edb8`): TabRail polish (slim kbd hints, BETA chip, pin-open w/ `--rail-w` reflow), Files-tab drag-reorder (pointer events, live shuffle via `animate:flip`), Sync collapsible shrink-banner + empty-state guard, About page Paths+Diagnostics w/ privacy-scrubbed copy.
- **S60-66** — AI Assistant page (auth, MCP tools, TodoWrite dock, conversation history, slash cmds, markdown renderer, workspace decoupling, state-aware EmptyState). Tab at Ctrl+3.
- **S67** — Canonical page skeleton (PageHeader/PageToolbar/PageFooter/EmptyState primitives). 5 pages converted. Titlebar declutter, TabRail 3-group rework + RIFT wordmark.
- **S59** — v0.2.55-alpha: one-button Sync, auto-rescan, keep-alive tabs (no flash on switch).
- **S58** — Terminal UI overhaul (Settings panel, search Ctrl+F, themes).
- **S57** — v0.2.54-alpha: fresh-install bootstrap hotfix for Trey onboarding.
