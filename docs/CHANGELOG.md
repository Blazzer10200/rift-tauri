# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.16 — 2026-06-09 — maintenance: backend split COMPLETE (#20 R1-R8)

> **Why.** Finishes the structural work v0.8.15 started: the backend's god-file is gone. No feature changes; every move is verbatim code relocation under test cover.

- **Backend split COMPLETE:** `assistant/mod.rs` 4331 → 303 lines across R1-R8. This release lands the final three: `config` (AssistantConfig + provider profiles + all config get/set commands + validation), `oneshot` (enhance/title/summarize/remint headless spawns), and `turn` (session registry + steer channels + permission plumbing + `assistant_send`/`stop`/`steer`). mod.rs is now a pure module hub; all tauri command paths re-exported — IPC surface identical.
- `kill_all_session_children` (load-bearing for the Velopack update apply) kept path-stable via explicit `pub(crate)` re-export.
- Clippy back to fully clean (one auto-deref nit in `mcp_server`).
- Docs: stale cross-refs fixed repo-wide, spent run docs retired, `composer-split.md` brief added (next #20 target).

**How to verify.** Pure refactor — everything behaves identically: chat turn (stream/tools/thinking), steer, stop, /retry, queue drain, prompt enhance, title gen, summarize/compact, History list/load/delete, Settings config get/set, provider CRUD, update check/apply.

**Verify.** `cargo check` + `cargo clippy --all-targets` zero warnings · `cargo test --lib` 95/95 per extraction commit · `npm run check` 0/0 (4072) · vitest 51/51.

## Older versions

v0.8.15 hot-file splits (TS complete + mod.rs 5/8) + honest Settings update chip · v0.8.14 fix: update dialog crashed on render (duplicate `{#each}` key — the real end of the "can't click update" saga) + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model (self-heals to Sonnet/Opus after Jun 22) · v0.8.12 pill `×` = 24h snooze, never permanent · v0.8.11 Settings redesign + Harness one-viewport overhaul · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
