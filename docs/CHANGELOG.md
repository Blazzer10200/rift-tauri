# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.73.0 — Front-end polish pass: diff/code-block fixes + dashboard motion

### Fixed / improved
- **File-creation diffs no longer show a phantom deleted line.** When the assistant created a new file, the create card's diff opened with a stray `−` row (an empty "removed" line) above the real additions — an artifact of diffing the new content against an empty string. A brand-new file has nothing to remove, so creations now render as pure additions and the line numbers line up with the `+N` count. (Frontend-only; CDP-verified live: a 4-line create now shows 4 add rows, 0 phantom del rows.)
- **The "show more" fade on long code blocks now blends seamlessly.** A collapsed code block's bottom dissolve was painted with a stale hard-coded graphite color left over from before the v0.72 block redesign, so it faded to a slightly different shade than the block's own emerald-tinted background — a faint seam under the "Show N more lines" pill. The fade now derives from the same design-token expression as the block itself, so it dissolves cleanly. (Frontend-only.)
- **The activity-stats error state matches the rest of the app.** If the Workspace dashboard couldn't load your usage stats it showed a bare red sentence — no icon, no way to retry. It now uses the same centered card treatment as the loading and empty states, with a warning icon, the error detail, and a one-tap **Retry**.
- **Dashboards assemble with a gentle top-down motion.** The AI Health cards and the Settings section cards now rise in with a short staggered entrance (same cadence across both) instead of flashing in all at once, and the right-edge usage bars in the status bar animate to their new width instead of snapping. All reduced-motion-respecting. (Frontend-only; svelte-check 0/0 · CDP-verified across every page.)

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.72.0** — Plan-mode unfreeze (the native "exit plan mode" step no longer hangs the turn for minutes, #75), terminal-grade work habits (the model now batches tool calls + skips redundant re-reads like it does in the CLI, #76), and a modern unified look for every chat block — code, terminal, file-read, and create/edit diff cards now share one emerald-tinted glassy Rift surface with a top accent glow and rise-in entrance (#77).

- **v0.71.8** — Maintenance (no behavior change): finished the path-helper de-dup — the path-string helpers had drifted into ~7 copies across the codebase; consolidated into one canonical, unit-tested home (`src/lib/utils/path.ts`). `leafName` (folder/file name on pane headers, file menus, conversation list, tool captions) and `rootKey` (the "same folder?" comparison key) each had multiple drifted copies — two even disagreed on trailing-slash handling; now uniform.

- **v0.71.7** — Maintenance (no behavior change): first half of the path-helper de-dup — consolidated `leafName`/`shortPath`/`prettyPath` into `utils/path.ts` (v0.71.8 folded in the `rootKey` half).

- **v0.71.6** — Split-pane stays in its lane: per-pane sub-agent panel (each pane shows its own agents, with a "N steps · Ms" summary, per-tool-type icons, and a blended-glass card), steer now tells you when a missed steer became a queued message, closing a background tab no longer drops its unsaved tail, and background-pane notifications/browser-dock stop popping in the pane you're looking at. Root cause: nearly everything read off the single global focused-tab (16 of 31 audit findings confirmed; backend session-isolation was already clean).

- **v0.71.5** — Fix: split-pane crosstalk (the inactive pane mirrored the active one's thinking timer + context readout) and dragging a project chip onto a pane (a copy/move drag-effect mismatch made WebView2 reject the drop).

- **v0.71.4** — Fix: a reused warm CLI process could fire an instant, off-topic reply (stale pipe frame) after an `ask_user` round-trip, plus a permission-prompt pairing race and broken per-turn latency telemetry (Issues #72 + #73).

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
