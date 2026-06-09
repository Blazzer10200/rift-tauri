# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.13 — 2026-06-09 — Claude Fable 5: limited-run model in the picker

> **What.** Anthropic's new **Claude Fable 5** (`claude-fable-5`, GA today) is selectable in Rift — for its limited run. It's gone after **June 22**; Rift enforces that cutoff itself so nothing breaks when the model retires.

- **Picker.** Fable 5 sits at the top of the model menu with an accent-tinted name and an **"UNTIL JUN 22"** badge — visually distinct, like the Legacy treatment on Opus 4.7. Effort tiers up to Ultra; no fast mode (Fable's adaptive thinking is always-on). Keyboard shortcuts renumber automatically.
- **Three-layer sunset, no stranded users.** After Jun 22: the row disappears from the picker, a stored Fable preference self-heals to Sonnet on load, and the backend falls back to Opus for any stale pref or pinned-Fable session (Anthropic route only — custom providers untouched).
- **Cost cockpit knows the price.** $10 / $50 per MTok (+ cache 12.5 / 1.0) added to the pricing table, so spend tracking is accurate from the first turn.

**How to verify.** Open the model picker → Fable 5 on top with the badge; select it → composer pill reads "Fable 5"; send a turn → usage panel prices it at Fable rates. After Jun 22 the row is gone and existing sessions keep working on Opus/Sonnet.

**Verify.** `npm run check` 0/0 (4070) · `cargo check` green · CDP pixel-verified (picker, selection, persistence, 0 console errors).

## Older versions

v0.8.12 fix: an update can never go invisible again — pill `×` is a 24h snooze (`{version,until}`), never a permanent dismissal; snooze-proof accent dot on the Settings gear; `backdrop-filter` stripped from dialog/toasts (WebView2 mis-composite) · v0.8.11 Settings redesign (single-column titled cards) + Harness one-viewport overhaul (Telemetry · Cost · Swarm) · v0.8.10 fix: update button no longer 50/50 — stable singleton `UpdatePill` replaces the sticky toast that slid out from under the cursor (`animate:flip` move-target) + WebView2 backdrop-filter garbage fixed on the pill · v0.8.9 first tag-driven CI release (Actions builds + packs + publishes to `rift-releases` end-to-end; `release.ps1` strips a non-ASCII `RELEASES_TOKEN` that was corrupting the upload `Authorization` header) · v0.8.8 updater end-to-end test (clean version bump post toast fix) · v0.8.7 fix: update toast was unclickable — host z-index raised 60→2000 above transient overlays + download self-heal + bisection logging · v0.8.6 in-app updater apply-path test · v0.8.5 fix: corrupted install no longer masquerades as "up to date" (`check()` surfaces a broken Velopack layout w/ reinstall card + self-heal retry, instead of faking "up to date") · v0.8.4 updater delivery test · v0.8.3 fix: updater can no longer hang forever (mutex released before network + 30s check timeout + 90s download stall watchdog) · v0.8.2 live update-path validation release · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast w/ [Get it on GitHub]) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list + "Rift noticed…" insights · v0.6.5 custom-provider escape hatch · v0.6.4 collaborator-401 install-selection fix · v0.6.3 auto-update hotfix verify · v0.6.2 in-app-update child-lock fix · v0.6.1 CLI multi-install awareness · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
