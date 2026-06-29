# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.2 — Maintenance: internal cleanup, no behavior change

### What you'll notice
- **Nothing should change in how Rift behaves** — this is a housekeeping release. Same features, same speed; the work was all under the hood to keep the codebase healthy.

### Under the hood
- **Refactored the turn-spawn path for readability.** The single largest function behind every chat turn (~740 lines that resolved your model/effort/thinking/permission settings and assembled the CLI command) was split into a small orchestrator plus a focused `resolve_spawn` step. Behavior is byte-identical — verified by the full test suite (123/123) — it's just far easier to maintain and reason about now.
- **Cleared every compiler lint** (14 → 0) and **removed dead code** (a pair of no-op methods and their call sites left over from an earlier refactor).
- **Reconciled the internal issue trackers to reality** — several items marked "open" were verified already shipped, so the docs no longer point future work at ghosts.
- **Verified.** cargo test 123/123 · clippy 0 warnings · svelte-check clean (4134) · 386/386 frontend unit tests.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.1** — Hotfix: the assistant could go passive on short messages — a benign plan-usage note in front of a brief message made the model give clipped/"winding down" replies. The note now appears only when usage is genuinely high, so short messages get a normal response again.

- **v0.71.0** — First-run onboarding rework: lighter welcome, a clearer Connect step when the CLI is missing, a scratch-space hint, and the Defaults step collapsed into one Cautious/Balanced/Fast choice.

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
