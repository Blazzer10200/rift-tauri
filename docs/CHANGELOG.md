# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.6 — 2026-06-08 — chore: in-app updater apply-path test

> **Why.** The full in-app `download → apply → relaunch` chain has never executed on any machine — every prior "update" was a manual Setup.exe install. This is a clean version-only bump so a client on a *freshly-installed* v0.8.5 (clean Velopack layout, launched from the Start Menu shortcut) can finally exercise the real path end-to-end against the live feed. No functional code change.

**What's new.** Version bump only (0.8.5 → 0.8.6).

**How to verify.** On a clean v0.8.5: open the update dialog, click **Download installer**, and let it run untouched. Expect progress → app closes → Velopack swaps files → relaunches onto v0.8.6. The rotating `rift.log` should show `update download: starting` → `update apply: scheduling swap` for the first time.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0.

## v0.8.5 — 2026-06-08 — fix: corrupted install no longer masquerades as "up to date"

> **Why.** Diagnosing a real "it detects the version but won't update" report (via the v0.8.1 `rift.log`), the root cause was a corrupted Velopack layout: `VelopackApp — NotInstalled("Could not auto-locate app manifest")`, so `UpdateManager::new` failed and the service held no manager. The old `check()` then returned `Ok(None)` — which the UI rendered as **"You're up to date."** A dead updater was lying that everything was fine. (The corruption itself comes from manual file swaps / launching a loose exe outside the managed `current\` dir — but the updater must *report* it, not hide it.)

**What's fixed.**
- **`check()` surfaces a broken install instead of faking "up to date."** When no manager is available it now returns an actionable error ("Rift isn't properly installed for auto-update … reinstall from the latest Setup.exe"), and the service records the init failure reason.
- **Self-heal retry.** If the manager was absent at startup, `check()` re-resolves once — so an install that recovers no longer needs a relaunch to update.
- **Update dialog adapts.** A "not properly installed" error shows reinstall guidance + a **"Get the latest Setup.exe on GitHub"** link (works even when the failed check left no release info), distinct from the transient "feed unreachable, try again" path.

**How to verify.** On a *clean* install (Setup.exe, launched from the Start Menu shortcut), a check should find the newer version, download with progress, apply-on-exit, and relaunch. A hand-broken install now shows the reinstall card rather than a false "up to date."

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4068).

## Older versions

v0.8.4 updater delivery test (clean version bump to exercise the v0.8.3 hardening through the live feed) · v0.8.3 fix: updater can no longer hang forever (check() releases mutex before the network call + 30s check timeout + 90s download stall watchdog — every failure surfaces instead of hanging dark) · v0.8.2 live update-path validation release (version bump to exercise the v0.8.1 logging/recovery surface through the live feed) · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
