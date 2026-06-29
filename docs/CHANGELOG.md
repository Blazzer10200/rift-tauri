# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.6 — Split-pane stays in its lane: per-pane sub-agents, steer feedback, no lost messages

### What you'll notice
- **Sub-agents now belong to their own pane.** With two chats side-by-side, the sub-agent activity panel showed the *focused* pane's agents in both panes. Now each pane has its own panel — a sub-agent running in the background pane shows in *that* pane. The panel also looks better: each finished agent shows a "3 steps · 408ms" summary, every tool step has a type icon (read / edit / search / shell / web / git), and the card is now a softer glass surface that blends with the chat instead of a hard floating panel.
- **Steering tells you what happened.** If you Alt+Enter to steer a reply but the turn finishes a split-second before it lands, your message used to silently vanish into the queue with no feedback — it read as "steer ignored." Now it clears the box and says "Turn finished — message queued" so you always know whether it injected or queued.
- **Closing a background tab no longer loses its last messages.** Closing a chat you weren't looking at could drop its unsaved tail. It's now flushed to History before the tab is retired, same as the active tab.
- **Background-pane notifications and the browser dock stay put.** A notification or `open_browser` from a background pane's turn no longer pops in the pane you're actually looking at.

### Under the hood
- All of the above trace to one root cause: nearly everything keyed off a single global "focused tab", so any operation meant for a *specific* pane leaked to the focused one. Diagnosed with a multi-agent audit (5 parallel finders → 24 adversarial verifiers → synthesis): 31 candidate findings, 16 confirmed, 15 refuted — including several plausible-but-wrong hypotheses the verifiers killed. The backend session isolation was already clean (everything keys by session id); the fixes are all on the frontend read path.
- The sub-agent panel moved from one page-level float to one per pane (scoped to that pane's tab); its expand/collapse + auto-reveal moved into each panel instance, leaving the global singleton as just the Settings on/off switch.
- Small cleanup: the "is auth ready to send?" check was copy-pasted in four places — now a single `authReady` getter.
- **Verified live.** Drove the running app over CDP through real sub-agent turns and split-pane navigation. svelte-check clean (4134) · 390/390 frontend unit tests (+3 split-pane regressions).

### Where these came from
A live multi-session test: the user confirmed two-folder isolation working, then asked for sub-agents to "stay in their lanes" per pane and to show more detail / blend in better. Caught and reverted one self-inflicted reactive-loop freeze mid-session (per-pane file-list rework) before it shipped.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.5** — Fix: split-pane crosstalk (the inactive pane mirrored the active one's thinking timer + context readout) and dragging a project chip onto a pane (a copy/move drag-effect mismatch made WebView2 reject the drop).

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
