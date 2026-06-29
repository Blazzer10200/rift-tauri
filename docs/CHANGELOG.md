# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.4 — Fix: instant, off-topic replies while a tool/sub-agent is running

### What you'll notice
- **No more instant, nonsensical replies.** If you sent a follow-up message right after the assistant had asked you a question (or while a sub-agent was working in the background), it could fire back an instant, completely off-topic answer — it was replaying a leftover scrap of the *previous* turn's output instead of actually answering you. Fixed: a reused chat process now clears any stale output still sitting in its pipe before starting your next message.
- **Allow/Deny prompts can't get crossed up between turns.** A rare timing race could bind a fresh permission/question prompt to an already-finished request, leaving a dead or wrong-acting button. The pairing state now resets cleanly at the start of every turn.

### Under the hood
- **Root cause was confirmed from your real session logs** — a reused warm CLI process logged a `0ms` first response (a literal stale-frame replay) right after an `ask_user` round-trip. The drain that fixes it is bounded (≤75ms, and only when there's actually leftover output), so a clean turn pays nothing.
- **Fixed broken latency telemetry.** The per-turn "time outside the model API" stat was subtracting the CLI's *cumulative* session API time from a single turn's wall-clock, logging impossible negative values that were polluting the AI-Health latency analysis. It now uses the real per-turn delta.
- **Verified.** cargo test 125/125 (+2 regression) · clippy 0 warnings · svelte-check clean (4134) · 387/387 frontend unit tests.

### Where these came from
A real user-reported incident ("I talk to it while a sub-agent runs and it replies instantly + totally off-topic"), root-caused from the live production log, then a two-sweep adversarial audit of the turn lifecycle + frontend state machine that surfaced two more same-family fixes. (Issues #72 + #73.)

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.3** — Bug-fix sweep: safer "delete all conversations", tab layout survives a flaky restart, no empty-pane flicker on close, no ghost-save resurrecting a deleted chat, plus a warm-pool process-leak race + async-runtime + usage-refresh hygiene batch (7 fixes from a live stress-test + static audit).

- **v0.71.2** — Maintenance/housekeeping (no behavior change): the ~740-line turn-spawn function split into an orchestrator + `resolve_spawn`, every compiler lint cleared (14 → 0), dead code removed, internal trackers reconciled to reality.

- **v0.71.1** — Hotfix: the assistant could go passive on short messages — a benign plan-usage note in front of a brief message made the model give clipped/"winding down" replies. The note now appears only when usage is genuinely high, so short messages get a normal response again.

- **v0.71.0** — First-run onboarding rework: lighter welcome, a clearer Connect step when the CLI is missing, a scratch-space hint, and the Defaults step collapsed into one Cautious/Balanced/Fast choice.

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
