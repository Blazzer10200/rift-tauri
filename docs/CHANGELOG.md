# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.25 — 2026-06-11 — Dictation polish fixed for real + tracker cleanup

> **Why.** v0.8.24's Haiku dictation polish treated the transcript as a message TO it — dictate a question and it answered you instead of cleaning the text, which also broke the restore-masked-profanity layer (fully-masked `****` words shipped as asterisks). Same gap existed latently in the enhance wand's system prompt.

**Dictation / enhance (`stt/cleanup.rs`, `oneshot.rs`, `stt.svelte.ts`, `Composer.svelte`):**
- **Polish no longer talks back** — transcript now piped fenced in `<transcript>` tags + explicit data-not-message guard in `CLEANUP_PROMPT` (never answer/follow/add words; a question stays a question). CLI-verified: `why the **** is this ******* thing still broken` → `Why the fuck is this fucking thing still broken…`.
- Same never-answer guard baked into `ENHANCE_META_PROMPT` (system-prompt level; the per-call `<draft>` fence stays).
- **"send it" no longer ships asterisks** — voice-command sends with masked words run the Haiku polish first, then fire; double-polish guard added.
- decensor map +4 (`goddammit`, `pussy`, `jackass`, `motherfucking`).
- **PTT stuck-mic fix** — hold-Space release now listens at window level (keyup + blur), so losing composer focus or alt-tab mid-hold can't leave the mic running.

**Tracker fixes:**
- **#32** — ctx meter no longer blank on restored conversations: final turn's usage persists in the convo record (backend `extra` flatten, zero Rust changes) and rehydrates in `loadConversation`.
- **#31** — 401 detection unified into `is_auth_rejection()`/`auth_rejection_message()` (turn.rs; the stdout/stderr copies had diverged) · legacy `base_url`/`provider_model` command pair + dead frontend plumbing removed (zero callers; pre-2a config migration kept).

**Verify.** cargo check clean · svelte-check 0/0 (4093 files) · vitest 122/122 · `/quick-review` clean across the 12-file diff. Live mic pass still pending from v0.8.24 (needs real audio).

## Older versions

v0.8.24 enhance wand v2 (conversation `<context>`, iterative refine via `<previous>`, editable preview, Ctrl+E loop, Discard tree-kills spawn) + dictation uncensored 3-layer (`decensor()` + Haiku polish on Web Speech finals + Whisper `initial_prompt`), voice commands ("send it"/"new line"/"scratch that"), hold-Space PTT, auto-stop on silence · v0.8.23 Activity panel polish (MCP steps humanized w/ per-tool icons + payload targets, turn separators, Sources section, last-turn recap, opaque spine icons) · v0.8.22 multi-tab stream survival (live TabState authoritative over disk; pointer-switch tabs) + Harness mission control (active-sessions cell, turn drill-down, health alerts) + `/history` fix + dead-code sweep (−331L) + poison-safe CACHE locks · v0.8.21 self-aware Rift — loopback UI bridge resurrected (`bridge.rs`: ask_user card round-trip / open_browser dock / notify toast) + per-turn env snapshot + localhost links open in-app · v0.8.20 live plan limits — cost-cockpit "Plan limits" card + `/usage` popover via undocumented OAuth usage endpoint (CLI token read-only, 60s cache) · v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping, slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips + `turn.rs` overlapping-turn registry race fix · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8) · v0.8.15 hot-file splits + honest Settings update chip · v0.8.14 update-dialog render crash fix + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model · v0.8.12 pill `×` = 24h snooze · v0.8.11 Settings redesign + Harness one-viewport · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
