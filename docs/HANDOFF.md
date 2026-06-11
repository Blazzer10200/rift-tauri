# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.116) — Dictation polish data-fence + tracker cleanup → SHIPPED v0.8.25

**Shipped v0.8.25** (feature `d9ff368`, release `0ccd441`, tag pushed → CI run 27359206587 queued at handoff — **verify green**). v0.8.24 release CI confirmed green. Verified: cargo clean · svelte-check 0/0 · vitest 122/122 · `/quick-review` clean.

- **Root cause of "dictation talks back + asterisks never fixed":** `cleanup.rs` piped the raw transcript bare on stdin → to Haiku the speech WAS the message; it answered questions instead of cleaning, which also killed the restore-masked layer (fully-masked `****` has no leading letter for `decensor()`). Fix: `<transcript>` fence + data-not-message guard in `CLEANUP_PROMPT`. **CLI-verified live** (masked input → fully restored, question stayed a question). Same guard added to `ENHANCE_META_PROMPT`.
- **"send it" gap closed:** voice-send with `\*{2,}` in finalText now awaits `polishWebSpeechFinal()` then fires; `polishing` re-entry guard prevents double-spawn from `onEnd`. decensor map +4.
- **PTT stuck-mic fixed** (the cont.115 known edge): release via `<svelte:window onkeyup onblur>` in Composer; textarea keyup now only refreshes mentions.
- **#32 resolved:** `lastTurnUsage` rides `ConversationRecord` (backend `Conversation.extra` flatten — zero Rust) · hydrated in `loadConversation` after `resetUsage()`.
- **#31 partials resolved:** 401 helpers `is_auth_rejection()`/`auth_rejection_message()` in turn.rs · legacy `base_url`/`provider_model` commands + frontend plumbing REMOVED (zero callers; config-struct fields + first-load migration KEPT for pre-2a configs).

### RESUME HERE

- **Verify CI release 27359206587 green** → user installs v0.8.25 in-app → live-test: dictate a question (must stay a question), mic cussing incl. fully-masked, "send it" w/ cusswords, hold-Space + alt-tab mid-hold, enhance Ctrl+E, ctx meter on a restored old chat.
- **ISSUES remaining:** #31 leftovers (blocking-fs deferred-by-design · **Fable dead-branch sweep after Jun 22**) · #30 cwd badge · #29 CSP-nonce (app-wide verify) · CR-UX trust enum (**needs user sign-off**) · Auth-Rec live-verify (needs logged-out machine).
- Chat-page arc candidates: collapsible Activity sections, tooltip `.tip` glass transparency (app-wide).
- Carried: `browser_screenshot` MCP arc · #7 charts · #12 chip affordance · #11/#13 · Settings checklist · POLISH tier · SEC-1.

## Prior arcs — detail in `git log` + CHANGELOG

cont.115 enhance wand v2 + dictation uncensored + voice cmds/PTT → v0.8.24. cont.113 Activity panel polish → v0.8.23 (MCP steps humanized, turn seps, Sources, recap, opaque spine icons; ISSUES #32 filed). cont.112 UI/UX arc (Home Mission-Control bento + `targetHarnessSubtab` deep-link · chat revamp w/ right-aligned user bubbles + merged thought rows + 1100px col · Activity idle recap, DOCK_DEFAULT 340). cont.111 full-codebase audit → shipped v0.8.22. cont.110 multi-tab stream-kill fix + Harness mission control. cont.109 bridge.rs loopback v0.8.21. cont.108 live plan limits v0.8.20. cont.106 custom context menus + Fable 1M fix. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175. **Spine-node icons stay opaque** (no `-soft` fills in ActivityPanel rows).
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.25 stands.**
- **`turn.rs::kill_all_session_children` re-export** (now also sweeps `oneshot::ENHANCE_PIDS`) + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
