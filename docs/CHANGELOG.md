# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.2 — 2026-06-08 — chore: live update-path validation release

> **Why.** v0.8.1 made update failures visible + always recoverable, but that fix can only be proven by watching a real update flow through the live Velopack feed. This release is a clean version bump cut for exactly that: a genuine higher version published to `rift-releases` so a client on v0.8.1 exercises the full check → download → apply → relaunch path against GitHub. No functional code change — the v0.8.1 logging/recovery surface is what's under test.

**What's new.** Version bump only (0.8.1 → 0.8.2). The embedded version string differs, so this is a distinct binary the updater treats as a real upgrade — not a byte-identical no-op.

**How to verify the updater.** On v0.8.1, trigger an update check: the dialog should report 0.8.2, download with progress, then apply-on-exit and relaunch onto 0.8.2. If anything fails, the new rotating `rift.log` (Settings → Help & diagnostics → Logs) now captures the Velopack internals — grab it to root-cause.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0.

## Older versions

v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
