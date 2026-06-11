# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.115) — Enhance wand v2 + dictation uncensored → SHIPPED v0.8.24

**Shipped v0.8.24** (feature `ba1f7dc`, release `5b01284`, tag pushed → CI run 27352950716; **verify CI green** if not yet confirmed). Verified: cargo clean · svelte-check 0/0 · vitest 122/122. Live mic/voice + new-panel pixel pass NOT done (prod app was running; no dev session).

- **Enhance wand v2** (`oneshot.rs` + `Composer`/`EnhanceBar`): conversation `<context>` block (8 msgs / 3K cap) · iterative refine via `<previous>` (chips + freeform steer input; Regenerate re-rolls) · editable preview (pencil) · Ctrl+E enhance→accept · 12s accept-undo · **Discard tree-kills the spawn** (`ENHANCE_PIDS` + `assistant_enhance_cancel`; swept by `kill_all_session_children` — update-apply lock safety) · cost/duration footer from result frame (result-frame text wins over deltas on grounded multi-turn) · live "Reading src/…" ground status · ground toggle in localStorage `rift.enhanceGround` + auto-on for code-anchored drafts.
- **Dictation uncensored** — root cause: Azure Web Speech masks profanity server-side, and web_speech path never got Haiku polish. 3 layers: `decensor()` regex (letter+stars→word, `stt.svelte.ts`) on interim/finals/partials · Haiku polish now runs on Web Speech finals (`stt_clean_transcript` wired) w/ restore-masked instruction in `CLEANUP_PROMPT` · Whisper `initial_prompt` biases verbatim profanity.
- **Dictation upgrades**: voice commands (`send it`→`stt.sendRequested`→Composer `$effect`→`fire()`; `new line/paragraph`; `scratch that` pops segment) · hold-Space PTT (300ms, empty composer only, repeat-swallow) · polish shimmer + 15s "Show raw" chip (`polishUndo`) · auto-stop on silence (`auto_stop_secs` 0/3/5/10, frontend interval; web_speech needs show_interim).
- **Carried cont.114 work committed**: ask_user 60s stale-nudge toast (`healthAlerts.askUserStaleNudge` + `streaming.ts` timer) · zero-tool spend stat (telemetry + Harness row) · turn.rs system-addendum TaskCreate/TaskUpdate + native-tools + edit-retry wording.

### RESUME HERE

- **Verify CI release 27352950716 green** + user installs v0.8.24 via in-app update; then live-test: mic cussing (web speech), "send it", hold-Space, enhance Ctrl+E loop.
- Known edge (in report, acceptable): PTT keyup missed if focus leaves composer mid-hold → window-level listener if it bites.
- **ISSUES #31** (legacy cmds · 401-dup · blocking-fs · **Fable dead-branch sweep after Jun 22**) + **#30** (reload init race) + **#32** (ctx meter on restore).
- Chat-page arc candidates: collapsible Activity sections, tooltip `.tip` glass transparency (app-wide).
- Carried: `browser_screenshot` MCP arc · #7 charts · #12 chip affordance · #11/#13 · Settings checklist · POLISH tier · SEC-1 · #29 CSP-nonce · CR-UX.

## Prior arcs — detail in `git log` + CHANGELOG

cont.113 Activity panel polish → v0.8.23 (MCP steps humanized, turn seps, Sources, recap, opaque spine icons; ISSUES #32 filed). cont.112 UI/UX arc (Home Mission-Control bento + `targetHarnessSubtab` deep-link · chat revamp w/ right-aligned user bubbles + merged thought rows + 1100px col · Activity idle recap, DOCK_DEFAULT 340). cont.111 full-codebase audit → shipped v0.8.22. cont.110 multi-tab stream-kill fix + Harness mission control. cont.109 bridge.rs loopback v0.8.21. cont.108 live plan limits v0.8.20. cont.106 custom context menus + Fable 1M fix. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175. **Spine-node icons stay opaque** (no `-soft` fills in ActivityPanel rows).
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.24 stands.**
- **`turn.rs::kill_all_session_children` re-export** (now also sweeps `oneshot::ENHANCE_PIDS`) + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
