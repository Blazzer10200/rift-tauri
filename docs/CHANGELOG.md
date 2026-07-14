# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.105.0 — One rhythm

- **Every hover and small animation now moves at the same speed.** Buttons, menus, chips, and cards had drifted onto a dozen slightly-different timings; they all run on the app's two motion speeds now, so interactions feel consistent everywhere.
- **Status colors are exactly on-palette everywhere.** A few success/warning/danger tints in odd corners (timeline bullets, menus, error pills) had drifted from the design system's real values — all corrected.
- **Leaner under the hood:** dead styles from retired designs removed; the floating panels (jump pill, agent/plan HUDs) share one shadow definition; dev builds no longer self-promote to the installed app when "Always run as administrator" is on (dev-machine safeguard — the installed app is unchanged).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
