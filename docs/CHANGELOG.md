# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.68.0 — Reliability + safety hardening (full-codebase audit pass)

### What you'll notice
- **Stop actually stops — instantly.** Hitting Stop now cancels a pending permission prompt and reaps the underlying process right away, so a turn can no longer get "stuck" for minutes after you've stopped it. (A real case had a turn wedged 9+ minutes; that path is closed.)
- **Updates are more dependable.** The release pipeline now refuses to publish a build that would leave you on a stale update feed, so "the app says it's up to date but it isn't" can't happen from a misconfigured release.
- **Everything else feels the same** — this is a stability release, not a visible-features one. Replies are still fast-by-default (thinking off), and the warm process still keeps turns snappy.

### Under the hood
A self-directed four-sweep audit of the whole codebase (static bug hunt · runtime/telemetry forensics · architecture review · perf baseline from real turns) surfaced 153 findings; the high-priority, fully-verified ones ship here. Full report under `docs/audit/cont228/`.
- **Stop / wedge fix.** `assistant_stop` now cancels pending `PermissionRegistry` entries (registry tagged by `session_id` + `cancel_all_for_session`), so a parked Allow/Deny await resolves immediately instead of parking the full 120s. `TurnOutcome::Stalled` now tree-kills the wedged child by PID — `loop_cleanup` only `start_kill`'d the direct handle, leaving the CLI's subprocess tree alive. (Fixes a reproduced prod incident.)
- **Crypto provider cleanup.** `reqwest` now uses `rustls-no-provider` so `aws-lc-rs` is fully out of the dependency tree (`cargo tree -i aws-lc-rs` is empty) — `ring` is the sole TLS provider, installed once as the process default in `run()`. Runtime-verified with a live HTTPS handshake.
- **Release safety.** `release.ps1` fails fast in CI if R2 secrets are missing (the live update feed is R2-only); `release.yml`'s verify step queries the correct repo instead of relying on a GitHub rename redirect.
- **Effort-flag test coverage + cleanups.** The thinking-off → `--effort low` override and the effort-tier → CLI-flag mapping are extracted to tested functions (silent regression on either would reinstate the slow "hello"). Five verbatim-duplicated helpers collapsed (`tree_kill`, `ctxWindowFor`, `dirs_home`, `read_body_capped`, `strip_unc`).
- **Verified.** `cargo test` 121/121, svelte-check clean (4134 files), vitest 78/78; live-CDP-verified the app boots, turns stream, Stop recovers cleanly, and every page renders with zero console errors.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
