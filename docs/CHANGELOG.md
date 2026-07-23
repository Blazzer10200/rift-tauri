# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.141.0 — Every model, honest compaction radar

- **The model picker now carries the full Claude lineup.** Opus 4.6 and 4.5, Sonnet 4.6 and 4.5 join the legacy flyout (correct context windows + effort ceilings each — the old "Sonnet 4.6 can't do X-High" limit is gone upstream, so the dial goes all the way up), and Haiku 4.5 is back (Anthropic reinstated it). Plus an **Opus 5 "Coming soon"** row — it becomes selectable the moment the release lands.
- **Auto-compaction detection finally respects your CLI settings.** If you tuned auto-compact (custom window, trigger percent, or turned it off), Rift now reads the same config the CLI does — the "Auto-compacting conversation…" card and the "context getting full" heads-up fire at *your* real trigger instead of assuming compaction happens near a full window. Manual and auto compaction now both land accurately.
- **Sub-agent cards show the agent's actual work.** On CLI 2.1.211+, background agents stream their text and reasoning into their inline card — not just tool calls.
- **Smarter long-tool handling on new CLIs:** heartbeat frames from slow tool calls are recognized (no more stray "unknown event" breadcrumbs), and the EndConversation safety tool renders honestly. Recommended CLI is now 2.1.214.
- **Split/chrome polish:** cleaner pane divider + calmer focus border, the empty pane offers project quick-picks, the status bar got re-zoned, and the composer's orbiting light is retired for a calm breathing ring. Fixed the folder chip above the composer showing a stale folder after switching workspaces in a split.

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
