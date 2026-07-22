# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.139.0 — Dictation you can trust, smarter GitHub, sharper chrome

- **Speech-to-text overhaul.** Sending while words were still gray silently discarded them — the ghost tail now rides along with every send, so you never wait for text to "turn white." Held-mic silence no longer invents filler words (".yeah"): segment commits and the stop-time pass are gated on real voiced audio, and stray leading punctuation is scrubbed. The model pre-warms at launch and on engine/model switches, killing the first-press stall. Ghost text is brighter with a live blinking caret.
- **GitHub chip got honest.** Polls every 30s while CI runs, relative times tick live, finished runs toast pass/fail, and the popover offers one-click "Pull/Push N commits with Claude."
- **Autocorrect actually works** — Enter-send and Shift+Enter no longer skip the final word, right-click Auto-correct fixes the whole text, ~55 new common typos.
- **Composer fixes:** no longer creeps taller after long drafts; toolbar slimmed to attach + mic + enhance (Escape still clears the draft).
- **Sidebar:** shows Pinned + today's chats only; everything older folds behind "Show earlier."
- **Plan card + HUD:** long plans fold completed items behind one "N completed" row; the floating plan/activity bars got a glass upgrade (deeper shadow, top catch-light, accent hairline, staggered stack-in). The turn rail is a tapered hairline with a drifting live glint.
- **Self-diagnosis:** frontend errors + `console.error` now reach the backend event bus and a rotating `events.ndjson`; the assistant gains `read_events` + `crash_reports` tools.
- **Compaction you can see.** `/compact` shows a live progress card — estimated percent, how many messages/tokens are being summarized, and a time estimate. When the CLI auto-compacts mid-turn (which used to look like an unexplained 3-minute hang), Rift now detects it and says exactly what's happening.
- **Sharper, faster chrome.** Launch intro rebuilt as a proper warmup: slab material, ticking boot percentage, breathing light, staged readout. Sidebar open/close is snappier (240ms, fewer reflow frames), full-screen backdrop blurs that caused lag + fuzzy rendering are gone (WebView2 mis-composites them on fixed overlays), and frosted menus are crisper (opaque fill + lighter blur).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
