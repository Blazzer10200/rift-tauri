# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.72.0 — Plan-mode unfreeze, terminal-grade work habits, unified modern blocks

### Fixed / improved
- **Plan mode no longer freezes (#75).** When the model finished planning and called the native "exit plan mode" step, the turn could hang on a multi-minute "Working…" (observed: 223 seconds) and never continue. Rift now auto-approves that step instead of waiting on an approval prompt that had no surface — the plan you see streamed is the plan, and Rift just lets the work proceed. (Backend-only; `cargo check` clean. Verified against the CLI: the same scenario that hung for 223s now completes in ~10s.)
- **The assistant now works the way it does in the terminal/VS Code (#76).** Rift's per-turn instructions to the model already said "batch independent tool calls, don't re-read files, act first" — but they were buried mid-paragraph in a ~14 KB wall of prose, so the model ignored them (a real session ran 350 turns with *zero* batched tool calls + 80 redundant re-reads). The five load-bearing work-habits now lead in a tight, scannable block, so the model batches its reads/greps and skips the re-reads. Live-verified: a fresh multi-file turn now fires 3 reads + 2 searches in one batch (was one-at-a-time). (Backend-only `--append-system-prompt`; rides the cached prefix so zero per-turn cost.)
- **Code, terminal, and edit/create blocks got a modern, unified look (#77).** Every block in the chat — fenced code, shell/terminal output, file-read results, and the create/edit diff cards — now shares one Rift-native surface: rounded corners, a soft emerald-tinted glassy panel built from the design tokens (no more the bare GitHub-gray rectangle), a hairline accent glow along the top edge, a gentle drop-shadow, a language/shell pill with a glowing dot, and a subtle rise-in entrance animation (reduced-motion respected). The create/edit diff previews — which used to bleed frameless into the chat — now sit in the same card as everything else. (Frontend-only; svelte-check 0/0 · vitest 394/394 · CDP-verified live.)

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
