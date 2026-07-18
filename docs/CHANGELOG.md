# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.127.0 — Chat switcher unwedge (malformed folder-path fix)

**Clicking certain chats did nothing, and every button after went dead until restart.** When you set a project folder, Rift stored its path in a low-level Windows form (with a `\\?\` prefix) that the rest of the app didn't recognize. Chats saved with that path couldn't be matched to their folder, so opening one stalled mid-switch and jammed the whole chat switcher — buttons stopped responding, and startup crawled while the app tripped over every affected chat.

- **Root cause fixed** — folder paths are now normalized on the way in, so no new chat can pick up the bad prefix. Guarded by a regression test.
- **Existing data healed** — all previously-affected chats and your recent-folders list were cleaned in place. No history was lost; transcripts are byte-for-byte intact.

---

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
