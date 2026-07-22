# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.137.0 — A real launch intro

- **New power-on intro** — launching Rift now plays a branded splash: a vertical shaft of accent light draws in, the RIFT wordmark assembles letter by letter, and a small terminal-style readout tracks the *real* boot stages (checking claude cli → starting session → opening workspace). The veil is opaque and holds until the app is genuinely ready, with a 10s hard cap so it can never trap the window.
- **No more half-loaded flash at boot** — the old splash lifted on a fixed timer while boot was still in flight, exposing placeholder skeleton bars stacked over the live composer. The overlay is gone entirely; the intro above now owns that window.
- **Fixed the doubled maximize icon** — when maximized, the restore control drew two full squares side by side (read as two maximize icons). It's now a proper restore glyph: front square + just the exposed corner of the back one.
- **Tool activity blocks redesigned** — the collapsed "Searched X · Read Y" rows no longer stretch full width; they hug their content like a quiet receipt chip, with a count badge + total duration on the right, and grow smoothly into a full-width ledger (hairline spine down the tool list) when opened.
- **Security: bumped DOMPurify** past a low-severity sanitizer advisory (GHSA-c2j3-45gr-mqc4) — this also fixes the failing `audit` workflow on GitHub.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
