# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.115.0 — One slab, honest toasts, a context heads-up

- **The sidebar and the main surface are now cut from the same slab.** Identical fill, grounded foot, machined bevel, film grain — the sidebar's old per-page frosted variant is gone, so the two docked islands finally read as one material on every page.
- **Toasts center on the main island, not the window.** With the sidebar open they used to sit visibly off-axis; the stack now tracks the island and glides with it when the rail collapses.
- **Rift warns you before context runs out.** At 85% full a toast offers one-click Compact, so you pick the compaction point at a natural break instead of having one forced mid-thought. One warning per conversation; it re-arms only after usage actually drops.
- **Quieter, better-fitted screens everywhere.** Page exits are snappier (no old-page ghosting under the new one) · AI Health shows loading skeletons instead of a blank stats row · the active project card stopped glowing (the pill and Continue already say it) · the chat greeting sits weighted-center with up to six recent chats · the sidebar resize handle blends until you actually drag it.
- **Clearer accent settings copy** — the Appearance card now says what the accent touches and that the whole app follows.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
