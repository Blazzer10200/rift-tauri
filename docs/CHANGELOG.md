# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.71.0 — First-run onboarding rework

### What you'll notice
- **A clearer first-run setup.** The welcome screen is lighter (the essentials, not a wall of text), and the final "Defaults" step is now a single **"How should Rift work?"** choice — pick **Cautious**, **Balanced** (recommended), or **Fast** instead of juggling three separate permission/thinking/git toggles. You can still fine-tune everything later from the composer and Settings.
- **Better Connect step when the CLI is missing.** If the `claude` CLI isn't found, the setup now spells out exactly what to do — install it, then click Re-check (or relaunch Rift if a fresh PATH hasn't reached it yet). The most common fresh-machine snag is called out directly.
- **Scratch-space hint.** The "open a project" step now mentions you can skip it and work in a private scratch space until you pick a folder.

### Under the hood
- Collapsed the onboarding Defaults controls into preset cards that fan out to the same settings; removed the now-dead thinking-dial logic. Frontend-only.
- **Verified.** svelte-check clean (4134 files), vitest 386/386; all four onboarding steps live-CDP-verified (render clean, presets switch correctly, zero console errors).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
