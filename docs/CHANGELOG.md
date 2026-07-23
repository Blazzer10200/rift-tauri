# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.140.1 — The long-turn freeze is dead

- **Fixed the recurring mid-turn UI freeze (blank `$` rows, stuck "Editing file" chips, frozen timers).** Root cause after 4 occurrences: command-output previews keyed their lines by *content*, so any command whose output repeated a line (`git push`, `gh run watch`, cargo progress) crashed the renderer's update loop — every chip after that moment rendered blank forever while the turn kept running fine underneath. Previews now key by position; same landmine also cleared from web-source chips, answer chips, and release notes. Your work was never lost — the transcript always healed on disk; only the live paint wedged.

## v0.140.0 — Split panes grew up

- **Split panes grew up.** A visible "Split" button now lives in the status bar (it was keyboard/drag-only before). Maximize any pane full-width and back — header button, double-click the header, or Alt+Enter; Alt+←/→ walks focus between panes. New panes start empty with a resume picker instead of grabbing a random open tab. Pane sizes survive opening/closing a pane instead of resetting, the width guard now accounts for the browser dock (no more sliver panes), and a background pane that finishes flashes "✓ done" so you don't miss it.
- **Pane corners are crisp now.** The rough/torn corners on split-pane cards are gone — real borders instead of shadow tricks, a focus rail that respects the rounded corners, and a smooth fade-in entrance instead of the scale pop that shimmered.
- **Background agents no longer fake "Done".** A turn that hands work to a background agent now shows a pulsing "Agent working in background — you can keep chatting" footer until the agent actually reports back.
- **Project setup, de-confused.** New Project is folder-first: pick the folder, the name fills itself, and 8 common junk patterns (node_modules, .git, build output…) are excluded by default. Include/exclude globs live behind a collapsed "File scope" row with a plain-English summary.
- **Workspace page uses big screens** — wider layout; fullscreen gets a 4-up project grid instead of dead gutters.
- **Code blocks:** one unified header (language · line count · Copy) on every block type, 6 new languages (Go, YAML, SQL, HTML, CSS, C#), tidier diff padding + faithful diff copy.
- **GitHub popover:** refresh spinner no longer loops forever; adds "checked Xm ago", a live elapsed timer while CI runs, and a green in-sync dot.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
