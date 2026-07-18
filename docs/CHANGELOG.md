# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.126.0 — Split-pane bulletproofing + composer menu overhaul

Four owner-reported issues from real split-pane use, all fixed:

- **The mic only lights up in the pane that's actually dictating.** The speech-to-text engine is a single shared service, but every pane's mic button mirrored its state — both ends lit up no matter where you were talking. Buttons now check ownership: the dictating pane animates, the other pane's mic disables with a "Dictating in another pane" hint, and Ctrl+D / hold-Space can't hijack a recording that belongs to the other side.
- **A pane's model can no longer leak in from the other pane.** With two panes on two projects, clearing a chat (or opening a new tab, or setting a pane's folder) could silently adopt the sibling pane's model. Panes with their own folder now pin their model explicitly — clear/new-tab preserve it, per-pane folder picks stop rewriting the shared defaults, and model/effort choices save against the pane's folder instead of the global one.
- **The model menu stays glued to the composer.** The settings and permission menus positioned themselves once, so the composer's center↔docked flight animation could strand them floating mid-screen. They now follow their anchor every frame. Misclicking the menu's padding also no longer steals focus — which was what sent the composer flying back up and closed the menu mid-adjustment.
- **New effort range slider.** The effort control is now a real range slider — recessed groove, accent fill, draggable knob — replacing the tick-only rail. Drag anywhere, click a tick, or use arrow keys; it still snaps to the same four discrete gears (leftmost = thinking off).

Plus: expanding a running agent card now shows a live mini-transcript — the agent's own thinking, narration, and tool errors in arrival order, scroll-clamped so a chatty agent stays a card, not a wall.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
