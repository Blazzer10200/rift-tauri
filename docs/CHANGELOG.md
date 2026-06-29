# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.3 — Bug-fix sweep: chat/tab persistence + warm-pool hygiene

### What you'll notice
- **Deleting all conversations is safer.** If the backend hiccups mid-purge, only the chats that actually deleted are cleared — surviving chats stay open instead of leaving broken tabs that could fail on your next message.
- **Your open tabs survive a flaky restart.** A transient load error during startup no longer wipes your saved tab layout.
- **No more empty-pane flicker** when you close a chat that's still streaming.
- **Closing a chat right after a reply can't resurrect a just-deleted conversation** — a stray background save is now cancelled the moment the tab closes.

### Under the hood
- **Fixed a rare background-process leak.** A race in the idle-eviction sweeper could orphan a live chat's helper process (~450 MB) instead of reaping it; the entry is now re-registered when a turn races in.
- **Process-kill sweeps no longer block the app's async runtime** (idle eviction + the CLI-update reap moved off the executor).
- **Capped concurrent usage-gauge refreshes at one**, so a burst of queued turns can't fan out duplicate background requests.
- **Verified.** cargo test 123/123 · clippy 0 warnings · svelte-check clean (4134) · 387/387 frontend unit tests (+1 regression).

### Where these came from
All seven fixes came from an autonomous live stress-test + an adversarially-verified static audit (36 raw findings → 10 confirmed → **7 fixed, 0 critical/high**). The other three were deliberately left as intentional design tradeoffs (documented in the issue tracker).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.2** — Maintenance/housekeeping (no behavior change): the ~740-line turn-spawn function split into an orchestrator + `resolve_spawn`, every compiler lint cleared (14 → 0), dead code removed, internal trackers reconciled to reality.

- **v0.71.1** — Hotfix: the assistant could go passive on short messages — a benign plan-usage note in front of a brief message made the model give clipped/"winding down" replies. The note now appears only when usage is genuinely high, so short messages get a normal response again.

- **v0.71.0** — First-run onboarding rework: lighter welcome, a clearer Connect step when the CLI is missing, a scratch-space hint, and the Defaults step collapsed into one Cautious/Balanced/Fast choice.

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
