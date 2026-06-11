# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.17 — 2026-06-10 — Rail-v2: steer chips + overlapping-turn registry race fix

> **Why.** The pending rail gains the third tier: a queued message can now ride INTO the next turn instead of becoming its own. Live-verifying that exposed a real race silently breaking steer + Stop at the start of every drained turn — fixed.

- **Rail-v2:** every queued chip gets a ↳ mode toggle — steer chips (accent-tinted, caption "Steers next turn") skip the drain and inject into the next turn at its first stream line; the rail's accent sweep replays as the pulse-on-inject. An all-steer queue degrades its head to a normal send so the queue can never strand. "Send now" (immediate inject into the running turn) unchanged. Plumbing: `TabState.onTurnStarted` (once per turn, first stream line) → `flushSteerChips`; `drainQueue` picks the first queue-mode chip.
- **Fix — overlapping-turn registry race (`turn.rs`):** DONE fires on `result` before the child is reaped, so the next (drained) turn re-registers the session's PID + steer sender under the same key — and the finishing turn's tail then unconditionally wiped both. Symptoms: steer answered `no_active_turn` and Stop silently no-opped for up to ~5s (reap grace) into every drained follow-up turn. Clears are now identity-guarded (`clear_session_pid_if` by PID, `clear_steer_tx_if` by `same_channel`).

**How to verify.** Queue two messages during a long turn, click ↳ on the second → accent tint + "Steers next turn". When the first fires as the next turn, the second injects into it (inline "You steered" marker) instead of starting its own turn. Stop responds immediately at the start of a drained turn.

**Verify.** vitest 118/118 (2 new) · `npm run check` 0/0 · tauri-dev rebuild clean · live CDP end-to-end pre/post fix (pre-fix repro → post-fix inline steer marker).

## Older versions

v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8; IPC surface identical) · v0.8.15 hot-file splits (TS complete + mod.rs 5/8) + honest Settings update chip · v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key — the real end of the "can't click update" saga) + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
