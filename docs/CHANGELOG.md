# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.4 — 2026-06-08 — chore: updater delivery test (post-fix)

> **Why.** v0.8.3 hardened the updater so a check can no longer hang forever — but that fix can only be *proven* by watching the fixed code actually pull a release. This is a clean version-only bump published to `rift-releases` so a client now running v0.8.3 (with the released mutex + 30s timeout + 90s stall watchdog) exercises the full check → download → apply → relaunch path against the live GitHub feed. No functional code change — the v0.8.3 updater hardening is what's under test.

**What's new.** Version bump only (0.8.3 → 0.8.4). The embedded version string differs, so this is a distinct binary the updater treats as a real upgrade — not a byte-identical no-op.

**How to verify.** On v0.8.3, trigger an update check: the dialog should report 0.8.4, download with progress, then apply-on-exit and relaunch onto 0.8.4. If anything stalls or errors, it now surfaces (no infinite spinner) and the rotating `rift.log` (Settings → Help & diagnostics → Logs) captures the Velopack internals — grab it before relaunch to root-cause.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0.

## Older versions

v0.8.3 fix: updater can no longer hang forever (check() releases mutex before the network call + 30s check timeout + 90s download stall watchdog — every failure surfaces instead of hanging dark) · v0.8.2 live update-path validation release (version bump to exercise the v0.8.1 logging/recovery surface through the live feed) · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
