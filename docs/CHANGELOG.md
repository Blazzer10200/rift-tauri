# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.8-alpha — 2026-05-17 — Hot-fix: dyslexia toggle "stuck on" + Appearance shell-switch "disappears"

Two UX bugs reported immediately after the v0.4.7 ship.

**1. Dyslexia mode wouldn't turn off.** Flipping the master toggle off correctly cleared the system-prompt addendum on the next turn, but the *visual* effects (Lexend font, increased line-height) stayed because the sub-dials in [src/lib/state/accessibility.svelte.ts](src/lib/state/accessibility.svelte.ts) wrote their CSS attributes unconditionally. User correctly perceived "I can't disable it." Fix: `apply()` now gates the font + line-height dials on the master flag — when dyslexia mode is off, the `data-a11y-font` attribute snaps to `"system"` and `data-a11y-line-height` snaps to `"off"`, regardless of the persisted sub-dial values. Persisted sub-dial state is preserved so re-enabling the master restores whatever the user last picked. Warm reading tint stays independent (some users want glare reduction without the dyslexia bundle). Settings UI also disables the sub-dial buttons via `disabled={!accessibility.dyslexiaMode}` so the relationship is visually obvious.

**2. Appearance → v0.3 shell toggle made Settings "disappear."** AppShell mounts a fundamentally different layout depending on `uiPrefs.useV03Shell`: the v0.2 shell renders Settings as a routed page (`active = "settings"`), the v0.4.1 shell renders Settings as a modal (`settingsModalOpen = true`). Live-flipping mid-Settings reparented the panel into a structure where it effectively had no mount point — user saw it vanish and assumed the toggle had reverted. Fix: [src/lib/state/ui-prefs.svelte.ts](src/lib/state/ui-prefs.svelte.ts) `setUseV03Shell()` now calls `window.location.reload()` (with a 120ms delay so the localStorage write commits first), re-mounting the whole shell with the new flag from the start. This matches the long-standing hint text *"Restart Rift after toggling — some components only read the flag at mount,"* now automated. Hint copy updated to *"Toggling reloads the window so the new shell mounts cleanly."*

3-file bump 0.4.7 → 0.4.8-alpha. Auto-verifier clean.

Verify post-install: (a) toggle Dyslexia-friendly mode on → confirm Lexend + spacing apply, toggle off → confirm both snap back to system defaults. (b) Appearance → flip v0.3 shell toggle → window reloads cleanly, Settings re-opens via the right path for the new shell.
