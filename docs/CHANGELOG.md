# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.12 — 2026-06-09 — fix: an update can never go invisible again

> **Why.** v0.8.11 sat "available" on every launch but never showed: one stray click on the update pill's `×` (flush against "View") had written that version to localStorage **permanently** — no pill, no hint, no recovery short of digging into Settings. Prod logs confirmed the backend found the update on 3 consecutive launches while the UI stayed silent.

**What's fixed.**
- **Snooze is now 24 hours, never forever.** The pill `×` stores `{version, until}`; it expires on a timer mid-session and on next launch. Stale permanent dismissals from older versions are discarded automatically. A newer release always supersedes a snooze immediately.
- **Snooze-proof titlebar dot.** Whenever an update is waiting — snoozed or not — an accent dot sits on the Settings gear (tooltip: "Update available — vX"). An available update is never invisible.
- **"UI scuffed out" class removed from every update surface.** `backdrop-filter` stripped from the dialog overlay/shell and all toasts — WebView2 mis-composites it on fixed elements (garbage rects, collapsed click targets; same measured bug fixed on the pill in v0.8.10). Dialog shell is now fully opaque.
- Dialog button renamed **"Remind me tomorrow"** to match the real behavior.

**How to verify.** With an update available: pill `×` hides the pill but the gear dot stays; pill returns within 24h or when a newer version ships; update dialog renders clean and both buttons click first try. Verified live via CDP — snooze→dot→expiry→dialog→download chain.

**Verify.** `npm run check` 0/0 (4070).

## v0.8.11 — 2026-06-09 — Settings redesign + Harness one-viewport overhaul

Harness sub-pages (Telemetry · Cost · Swarm) reorganized to one viewport, zero scroll — session browsing via `Live · recent-4 · All 40→` instead of the hidden 4193px pill river. Settings rebuilt as a single centered column of titled cards (header bands inside cards); Assistant tab regrouped into **Cost guard** / **Model & routing** (w/ precedence note) / **Compression proxy (advanced)**. Settings audit: every control traced end-to-end; imperceptible Accent-presence toggle + dead `data-ligatures` write removed. `npm run check` 0/0.

## Older versions

v0.8.10 fix: update button no longer 50/50 — stable singleton `UpdatePill` replaces the sticky toast that slid out from under the cursor (`animate:flip` move-target) + WebView2 backdrop-filter garbage fixed on the pill · v0.8.9 first tag-driven CI release (Actions builds + packs + publishes to `rift-releases` end-to-end; `release.ps1` strips a non-ASCII `RELEASES_TOKEN` that was corrupting the upload `Authorization` header) · v0.8.8 updater end-to-end test (clean version bump post toast fix) · v0.8.7 fix: update toast was unclickable — host z-index raised 60→2000 above transient overlays + download self-heal + bisection logging · v0.8.6 in-app updater apply-path test · v0.8.5 fix: corrupted install no longer masquerades as "up to date" (`check()` surfaces a broken Velopack layout w/ reinstall card + self-heal retry, instead of faking "up to date") · v0.8.4 updater delivery test · v0.8.3 fix: updater can no longer hang forever (mutex released before network + 30s check timeout + 90s download stall watchdog) · v0.8.2 live update-path validation release · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
