# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.70.0 — Workspace + projects UI overhaul (beta-test release)

### What you'll notice
- **Open a project in one click.** New project chips in the sidebar — click to open, drag onto a pane, or right-click to split. Opening a project into a split pane went from a ~5-step dance to a single click. The Workspace page gained Split buttons on project cards and rows too.
- **The dashboard is now "Workspace" everywhere.** The home/dashboard surface is consistently called **Workspace** (sidebar, command palette, title bar); the chat surface is consistently **Chat**. Fixed a latent bug where "Go to Workspace" in the command palette landed you on an empty chat instead of the dashboard.
- **The context gauge is honest on big-context models.** On 1M-context models the gauge no longer caps the readout at 200K — it reflects your real plan/model window. There's a Plan setting (defaults to the largest window) so the gauge matches what you actually have.
- **Cleaner Workspace + AI Health.** The Workspace page lost its double-scroll (one scroll now), News collapses into a strip, and projects get a full-width hero with compact rows. AI Health stops contradicting itself — when slowness is on Anthropic's side it says so (amber "API is slow right now") instead of blaring red "action needed," and tiny <1% rows are dropped from the usage breakdown.

### Under the hood
- **Local-LLM TLS fix.** The local-LLM probe/test/optimize commands now route through the shared corporate-CA client like every other outbound call, so a local-LLM endpoint behind a TLS-inspecting proxy no longer silently fails its HTTPS handshake. (Surfaced by a pre-release security sweep — additive-only, no change off-proxy.)
- **Cleanup.** Removed superseded duplicate CDP stress scripts; no source-tree dead code (clean audit).
- **Verified.** `cargo test` 121/121, svelte-check clean (4134 files), vitest 386/386; each surface live-CDP-verified during the arc.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
