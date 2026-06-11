# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.24 — 2026-06-11 — Enhance wand v2 + dictation uncensored

> **Why.** The enhance wand was stateless (mid-thread drafts enhanced blind), refines re-rolled instead of iterating, Discard left the CLI spawn running and billing, and dictation bleeped profanity — the recognition engine (Azure-backed Web Speech) masks swears server-side and nothing ever restored them.

**Enhance wand (`oneshot.rs` + `EnhanceBar`/`Composer`):**
- Conversation awareness — recent chat tail rides a `<context>` block so "fix that same thing" resolves to real names.
- Iterative refine — directive chips + new freeform "steer the rewrite…" input edit the previous rewrite (`<previous>` block) instead of re-rolling; Regenerate still re-rolls.
- Editable preview (pencil), Ctrl+E enhance→accept loop, 12s restore-draft undo after accept.
- **Discard now kills the spawn** — `ENHANCE_PIDS` registry + `assistant_enhance_cancel` tree-kill; also swept by `kill_all_session_children` (update-apply lock safety). Cost/duration footer from the result frame; grounded passes stream live "Reading src/…" status; ground toggle persists + auto-enables on code-anchored drafts.

**Dictation (`stt/` + `stt.svelte.ts`):**
- **Cussing uncensored, three layers** — deterministic `decensor()` restores letter-masked words (`f***ing`→`fucking`) on interim+final text; Haiku polish now also runs on Web Speech finals (was Whisper-only) with an explicit restore-masked-profanity instruction; Whisper `initial_prompt` biases verbatim profanity.
- Voice commands (toggle, default on): "send it" fires, "new line"/"new paragraph" break, "scratch that" deletes the last phrase.
- Hold-Space push-to-talk (empty composer, ≥300ms hold; tap inert), CC-CLI style.
- Polish shimmer + 15s "Show raw" undo chip; auto-stop on silence (Off/3s/5s/10s).

**Carried from cont.114:** ask_user stale-nudge toast (60s, suppressed when on-screen) · zero-tool spend stat in Harness session summary · system-addendum/`/tools` refresh (TaskCreate/TaskUpdate wording, native-tool guidance, edit-retry tactic).

**Verify.** cargo check clean · svelte-check 0/0 (4093 files) · vitest 122/122. Live mic/voice pass pending (needs real audio).

## Older versions

v0.8.23 Activity panel polish (MCP steps humanized w/ per-tool icons + payload targets, turn separators, Sources section, last-turn recap, opaque spine icons) · v0.8.22 multi-tab stream survival (live TabState authoritative over disk; pointer-switch tabs) + Harness mission control (active-sessions cell, turn drill-down, health alerts) + `/history` fix + dead-code sweep (−331L) + poison-safe CACHE locks · v0.8.21 self-aware Rift — loopback UI bridge resurrected (`bridge.rs`: ask_user card round-trip / open_browser dock / notify toast) + per-turn env snapshot + localhost links open in-app · v0.8.20 live plan limits — cost-cockpit "Plan limits" card + `/usage` popover via undocumented OAuth usage endpoint (CLI token read-only, 60s cache) · v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping, slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips + `turn.rs` overlapping-turn registry race fix · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8) · v0.8.15 hot-file splits + honest Settings update chip · v0.8.14 update-dialog render crash fix + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model · v0.8.12 pill `×` = 24h snooze · v0.8.11 Settings redesign + Harness one-viewport · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
