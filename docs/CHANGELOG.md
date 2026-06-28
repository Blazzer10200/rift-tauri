# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.1 — Hotfix: the assistant could go passive on short messages

### What you'll notice
- **The assistant answers your short messages properly again.** A bug could make Claude reply to brief messages ("hi", "ok", "what's up") with vague one-liners — or, worse, act like there was nothing to respond to and even redo work it had already done. That's fixed. Every message, long or short, now gets a normal, engaged response.

### Under the hood
- **Root cause:** Rift attached a per-turn "environment snapshot" (including your Claude plan-usage gauges) to *every* message, and a system instruction told the model to "wrap up gracefully when plan usage runs hot." On a perfectly healthy plan a benign "weekly 67% used" note, sitting in front of a short message, read to the model as *"the user is winding down"* — so it gave clipped, passive answers. (Diagnosed from a real session: the model was receiving the message fine; it was the framing that misled it.)
- **Fix:** the plan-usage note is now shown only when usage is genuinely high (≥90% of the 5-hour window or ≥95% weekly) — so on a normal turn your message reaches the model with nothing prepended. The system instruction was reworded so a snapshot can never make the model treat a short message as low-priority or go passive.
- **Verified.** New regression tests lock the threshold (cargo test 123/123); svelte-check clean (4134); live-verified in the running app — a short message now returns a direct reply with no snapshot attached.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.71.0** — First-run onboarding rework: lighter welcome, a clearer Connect step when the CLI is missing, a scratch-space hint, and the Defaults step collapsed into one Cautious/Balanced/Fast choice.

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
