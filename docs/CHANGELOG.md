# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.15 — 2026-06-09 — maintenance: hot-file splits (#20) + honest Settings update chip

> **Why.** The two largest files in the codebase were past the maintainability threshold. This release lands the structural split on both sides plus one visible fix. No feature changes; every move is verbatim code relocation under test cover.

- **Frontend split COMPLETE:** `assistant.svelte.ts` 2709 → 1700 lines. The stream pump (M8 → `assistant/streaming.ts`) and the send/queue/steer orchestrator (M9 → `assistant/send.ts`) moved to free-fn modules; `TabState`/store methods are thin thunks, so every call site and behavior is unchanged. The conversation-playback regression suite drives the moved pump end-to-end.
- **Backend split 5/8:** `assistant/mod.rs` 4331 → 2917 lines — `cli_install` (CLI discovery/ranking) · `convo_store` (conversation persistence + sidecars) · `auth_update` (auth probe + CLI updater) · `env_checks` (compression/host-tool probes) · `workspace` (root state + @-mentions) carved into sibling modules. All tauri command paths re-exported — IPC surface identical.
- **Fix (visible): Settings update chip no longer lies.** It hard-coded "· up to date" with a green check regardless of state. Now driven by one derived `updates.summary` — amber "vX available", spinner while checking, red on a failed check/broken install, green only when actually up to date.

**How to verify.** Settings hero chip reflects real update state (matches the pill/titlebar dot). Chat, steer, stop, /retry, History list/load/delete, auth pill, CLI update — all unchanged.

**Verify.** `npm run check` 0/0 (4072) · vitest 51/51 (playback suite) · fresh `cargo check` zero warnings · `cargo test --lib` 95/95 per extraction commit.

## Older versions

v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key on blank note lines — the real end of the "can't click update" saga) + swarm worktree-escape guard & 6 defensive fixes · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent; snooze-proof dot; blur stripped from dialog/toasts · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
