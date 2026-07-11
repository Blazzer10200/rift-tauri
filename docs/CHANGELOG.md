# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.101.0 — One terminal, everywhere

- **Every tool block now shares one anatomy** — the live stream and saved history render commands, file reads, searches, and results through the same header and output components, so the whole transcript finally reads as one family instead of two drifting designs.
- **Real terminal colors** — shell output renders its actual ANSI colors (cargo greens, npm warnings, red errors) instead of keyword-guessed tinting. Plain uncolored lines still get a conservative ok/warn/error tint.
- **Shell blocks feel alive** — the command line is syntax-highlighted, a live timer ticks with a blinking cursor while a command runs, output prints in with a subtle cascade, and a truthful exit receipt closes it out (`✓ ok · 9s · 17 lines`). The old pill that claimed "exit 1" on every failure now honestly says `failed`.
- **Long output folds around the middle** — first lines + last lines with an "N lines hidden" divider, so the summary or error at the END stays visible without expanding. Click the divider to unfold.
- **Test and lint runs are first-class** — vitest / cargo test / pytest / svelte-check / eslint commands render with real pass/fail count pills and auto-expand their failure output. (Previously a dead-end pill with no way to see *which* test failed.)
- **Search results are navigable** — grep/glob output parses into `path:line` rows with the match highlighted; click a row to open the file in your editor, reveal it, or copy the path.
- **File reads are readable** — Read results render syntax-highlighted with real line numbers matching the file, like a proper code block.
- **The model's thinking renders as Markdown** in the live stream (history already did — the live view was the outlier).
- **A light rail line** now runs down each working turn so multi-step work reads as one connected sequence.
- **Run Rift as administrator (opt-in)** — Settings → Administrator access: relaunch elevated for one session (single UAC prompt), or "always run as administrator" via a per-user scheduled task (prompt-free launches). The assistant's tools then inherit admin rights — no more per-action UAC walls. Off by default; a status-bar "Admin" badge shows when elevated.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Voice profanity on Web Speech:** fully-masked words (`******`) can't be recovered from Azure's servers — real fix is the on-device **Whisper** engine (built, not yet shipped).
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
