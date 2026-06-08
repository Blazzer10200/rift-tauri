# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.1 — 2026-06-08 — fix: make app-update failures visible + always recoverable

> **Why.** A user on v0.7.0 reported the "update available" toast (and every other update entry point) doing nothing when clicked — the app stayed on the old version. Investigation cleared the whole update infrastructure: the v0.8.0 release is byte-perfect (size + SHA1 + SHA256 match the feed), the download engine and apply both work, and the front-/back-end update code is correct. The real defect is that the in-process Velopack check/download/apply logs only to stderr — which is `/dev/null` in a GUI build — so when a download fails on a given machine it leaves **zero trace**, and the click can look like "nothing happened." This release makes update failures impossible to miss and always recoverable, and gives us the on-machine logs to root-cause the next one.

**Persistent update log.** Every `log` record (including Velopack's detailed check/download/apply internals) now writes to a rotating `rift.log` in the app log dir — the path already shown under Settings → Help & diagnostics → Logs. No more debugging blind.

**No silent failures.** A failed download now forces the update dialog open with the error AND raises a sticky toast — an update click can never again look like a no-op.

**Always recoverable.** That failure toast carries a **[Get it on GitHub]** action that opens the release page, so even if the in-app Velopack path breaks on some machine, you can always grab the installer manually and never get stuck.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4067 files). Update infrastructure (feed/hash/engine/apply) independently verified healthy end-to-end.

## Older versions

v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
