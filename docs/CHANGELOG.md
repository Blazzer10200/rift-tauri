# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.148.0 — Quicker, and it stops "fixing" words you spelled right

- **Autocorrect leaves your real words alone.** It judged words against a 50k-word list that holds base words only — no regular plurals or endings — so anything it hadn't heard of got rewritten to the nearest common word. "greps" became "grips". It now recognises inflected and prefixed forms of words it already knows, only corrects toward genuinely common words, and treats Rift's own vocabulary (opus, sonnet, shiki, velopack) as real.
- **Faster startup.** Syntax highlighting loaded all seventeen language grammars while the app was still painting its first screen. That now waits for the first idle moment instead of competing with the window you're waiting on.
- **Faster workspace search.** The assistant's file search now checks whether a file matches at all before walking it line by line. Most files in a project don't match, and those now cost almost nothing.
- **Smaller download on every update.** Release builds were being compiled as sixteen separate chunks with no optimization across them. They're now optimized as one unit and stripped of debug symbols — a faster backend and a smaller update to pull down.

## v0.147.1 — Tighter locks, honest labels

- **First-run setup said "Opus 4.8" when the picker said "Opus 5".** Onboarding kept its own copy of the model list, and it had quietly drifted out of date. It now reads the same list the composer does, so the two can't disagree again.
- **The in-app browser can no longer reach app-level permissions.** Permissions were granted per *window*, and the browser dock lives inside the main window — so a grant meant for Rift's own UI technically extended to whatever page you had open. They're now granted per *webview*, which leaves the browser out by construction. Nothing was exploitable in practice; this closes the door before it matters.
- **Workspace file access is scoped correctly with more than one folder open.** The include/exclude rules the assistant's file tools respect only applied to your first workspace folder — anything under a second folder fell through unfiltered. Every folder is now checked against its own rules, and anything outside them is denied rather than allowed.
- **"Open in VS Code" no longer misreads a path as a command flag.**

## v0.147.0 — Opus 5 is here

- **Claude Opus 5.** Anthropic's newest, most capable Opus is now the default Opus in the picker — same price as 4.8, stronger at checking its own work and pushing a hard task through to done. Opus 4.8 moves into "More models" so anything pinned to it keeps running exactly as before.
- **Find on a page — `Ctrl+F`.** A real find bar in the in-app browser: type to jump between matches, `Enter` / `Shift+Enter` to walk them, `Esc` to close.
- **Zoom a page** from the browser's ⋯ menu, separately from the app-wide UI zoom, and it sticks as you click around.
- **Links stop vanishing.** `target="_blank"` pages used to spawn an invisible popup that read as a dead click; they now open in the dock you're looking at.
- **A friendlier empty browser** — the blank panel explains what it's for, with a search box and the key shortcuts.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
