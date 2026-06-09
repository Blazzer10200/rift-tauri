# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.11 — 2026-06-09 — Settings redesign + Harness one-viewport overhaul

**Harness.** All three sub-pages reorganized to fit one viewport with zero scroll. The Telemetry strip's hidden 4193px horizontal pill river is gone — replaced by `Live · recent-4 · All 40→`, with full session browsing in the All view. The archived-session hero splits its lone cost figure into a left identity block + a 2×2 stats grid. SwarmPage was brought in line with the shared visual system (title/card sizing, radius, icon buttons, empty state, dotted-grid background); CostPage tightened to a single-viewport bento.

**UI system.** Settings moved from a ragged 12-col bento to a **single centered column of titled cards** — section titles are now header bands *inside* each card (sentence-case), not labels floating over flat slabs. Inline code in descriptions de-boxed to a calm wash; descriptions capped at 60ch so they no longer collide with their controls.

**Assistant tab reorganized.** "Budget & billing" → **Cost guard** (per-turn cap only); API-key fallback + custom providers merged into one **Model & routing** card with an explicit precedence note (API key → custom provider → compression proxy); "Context compression" → **Compression proxy (advanced)**.

**Audit + cleanup.** Traced every setting end-to-end — all wired, nothing decorative. Removed the **Accent presence (Calm/Bold)** toggle (imperceptible: only nudged one ghost-fill opacity) incl. its store field/CSS/persistence, plus a dead `data-ligatures` DOM write. Fixed code-preview copy to its true scope ("diffs/previews/file browser" → "code blocks in chat replies").

**Verify.** `npm run check` 0/0 (4070).

## v0.8.10 — 2026-06-09 — fix: update button no longer 50/50 — stable pill replaces the flaky toast

> **Why.** The "Update available" affordance was a *sticky toast* in the shared toast stack, and it accepted clicks only about half the time — the long-running "update button won't click" bug that v0.8.7's z-index raise only partly tamed. Real cause: the toast host is bottom-anchored and grows upward, so the launch-pushed update toast sat at the top of the stack and **slid** every time any other toast appeared or expired (`animate:flip`, 180ms). You were clicking a target that kept moving out from under the cursor.

**What's fixed.**
- **Dedicated `UpdatePill` replaces the update toast.** A singleton fixed pill that never reflows — stable click target, opaque background, scoped class. The toast stack now carries only the transient install-*failure* notice (which can't move-target: it opens the dialog simultaneously, hiding the pill).
- **Two render bugs caught in live verification.** The pill's generic `.pill` class collided with a global status-pill rule (`height: 20px`), collapsing it to a 20px sliver; and `backdrop-filter` on a bottom-anchored fixed element triggers WebView2 compositing garbage. Both fixed (unique `.upd-pill` class + solid background).
- **Snooze persists** — dismissing a version hides the pill until a newer one ships (localStorage), unchanged.

**How to verify.** With an update available, the bottom-right pill is clickable on the first try, every time; **View** opens the dialog; **×** snoozes. Verified live via CDP — render, click→dialog, snooze→persist.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4070).

## Older versions

v0.8.9 first tag-driven CI release (Actions builds + packs + publishes to `rift-releases` end-to-end; `release.ps1` strips a non-ASCII `RELEASES_TOKEN` that was corrupting the upload `Authorization` header) · v0.8.8 updater end-to-end test (clean version bump post toast fix) · v0.8.7 fix: update toast was unclickable — host z-index raised 60→2000 above transient overlays + download self-heal + bisection logging · v0.8.6 in-app updater apply-path test · v0.8.5 fix: corrupted install no longer masquerades as "up to date" (`check()` surfaces a broken Velopack layout w/ reinstall card + self-heal retry, instead of faking "up to date") · v0.8.4 updater delivery test · v0.8.3 fix: updater can no longer hang forever (mutex released before network + 30s check timeout + 90s download stall watchdog) · v0.8.2 live update-path validation release · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
