# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.3 — 2026-06-08 — fix: updater can no longer hang forever

> **Why.** Update checks could spin indefinitely on other users' machines — "click Check, it just loads" — and a stalled download had the same dead-end. Root cause: `check()` held the update mutex across the blocking GitHub network call (which has no timeout), so one slow/hung check wedged the lock and every later command deadlocked behind it. The failure was invisible: the UI sat on a spinner instead of surfacing an error.

**What's fixed.**
- **`check()` releases the mutex before the network call** — mirrors `download()`/`apply()`, which already cloned the manager out. Kills the compounding deadlock where one hung check blocks all later commands.
- **30s hard timeout on `check_for_updates`** — reqwest-under-Velopack has no default timeout; now an unreachable/blocked GitHub surfaces an error instead of an infinite spinner.
- **90s stall watchdog on `download_update`** — a wedged transfer (half-open socket, dead proxy) now aborts with an error instead of stranding the UI on "downloading".

Every updater failure mode now **terminates and surfaces** (and writes Velopack internals to the rotating `rift.log`) instead of hanging dark — so the next real-world failure is finally diagnosable.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0.

## Older versions

v0.8.2 live update-path validation release (version bump to exercise the v0.8.1 logging/recovery surface through the live feed) · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
