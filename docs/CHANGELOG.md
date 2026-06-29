# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.5 — Fix: split-pane crosstalk + drag-a-project-onto-a-pane

### What you'll notice
- **Split panes are truly independent again.** When two chats were open side-by-side, the *inactive* pane mirrored the active one's "thinking" timer and live token/context readout — both panes showed the same "Still waiting… · 109s" even though only one was working. Each pane now shows only its own turn state, context %, and `/usage` context bar.
- **Dragging a project onto a pane works.** Dropping a project chip onto a split pane did nothing — the cursor showed "no-drop" and the drop never registered. Fixed; the project now opens in the pane you drop it on.

### Under the hood
- The shared-state leak was a read-path bug, not a state bug: per-pane components (`StreamTurn`, `MessageBubble`, the composer context ring, and the `/usage` popover) read the store's bare `streaming`/`activity`/`ctx*` getters, which all delegate to the single focused tab. Each now reads its own pane's `tab` via the `*For(tab)` helpers.
- The drag failure was a drag-effect mismatch: project chips start the drag with `effectAllowed="copy"`, but the pane's dragover handler hard-coded `dropEffect="move"`. A copy-source + move-target pair makes Chromium/WebView2 reject the drop outright, so the `drop` event never fired. The handler now matches the effect to the drag type (copy for projects, move for tabs); the underlying drop → open-in-pane logic was already correct.
- Ruled out (investigated, not a bug): warm-pool CLI session collision — each tab keys its CLI session off its own unique conversation id, so there's no actual conversation bleed.
- **Verified.** svelte-check clean (4134) · 387/387 frontend unit tests.

### Where these came from
A live split-pane session: the user saw both panes share one "thinking" timer (screenshot-confirmed) and a project chip refuse to drop onto pane 2. Root-caused inline, then a same-class sweep of every per-pane component.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.4** — Fix: a reused warm CLI process could fire an instant, off-topic reply (stale pipe frame) after an `ask_user` round-trip, plus a permission-prompt pairing race and broken per-turn latency telemetry (Issues #72 + #73).

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
