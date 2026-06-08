# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.0 — 2026-06-08 — feat: one-click 401 recovery + edit-swarm + opt-in context compression

> **Why.** A collaborator opened the updated app and hit a dead end: "Authentication failed (401) — open a terminal and run `claude login`." That's the wrong answer for a desktop app. Root cause: `claude auth status` can report `loggedIn:true` for a stale OAuth token the API then rejects, so the pre-send check passes but the turn still fails. This release turns that wall into one-click recovery, and ships the edit-swarm + context-compression work that's been soaking since v0.7.0.

**In-app sign-in recovery.** A 401 now renders an *actionable* banner instead of a wall of text. A rejected `claude login` session shows **[Sign in]** — Rift opens the CLI's own `auth login` in a console, the browser OAuth flow runs, and Rift polls until the session is live again, then clears the error and tells you to resend. A rejected API key shows **[Open Settings]**. Both carry **[Re-check]**, which drops the banner the moment auth is healthy. Credentials land in the CLI's own store, so a successful sign-in fixes the failure for real — no terminal, no manual retry.

**Edit-swarm (Harness → Swarm).** Hand one edit task to several parallel attempts, each in its own isolated git worktree and verified independently; Rift parses the verdicts and surfaces the best result. New `swarm/` backend module + a Harness → Swarm sub-tab, within the existing 4-workspace IA.

**Opt-in context compression.** A new Settings toggle routes turns through a local compression proxy via the existing `ANTHROPIC_BASE_URL` seam, to stretch context on long sessions. Off by default; the proxy runtime is a soft dependency Rift never bundles or spawns — you run it, Rift owns only the env seam and a reachability check. A custom provider still wins the seam.

**Hardening.** Test coverage went from ~9 to 61 Rust tests + 27 vitest: the MCP workspace-tool path-containment, git tools, trust gating, URL scheme allowlist, path sanitization, and cost-cockpit math now have regression coverage they lacked.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4067 files) · 61 Rust + 27 vitest green · auth-recovery banner (all three states + navigation) live-verified via CDP.

## Older versions

v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch + cont.66 hardening · v0.6.4 collaborator-401 install-selection fix + leaner releases · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
