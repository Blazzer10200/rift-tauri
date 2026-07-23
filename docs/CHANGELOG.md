# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.142.0 — Agents you can watch, autocorrect that thinks

- **Launching feels intentional now.** The splash hands off to the shell instead of blinking away — the sidebar slides in and the main island rises as the veil lifts. Faster minimum splash, and the boot % readout only appears when boot is genuinely slow.
- **Live agent cards grew up.** Long tool steps show a heartbeat dot + elapsed time, agent results render as real markdown, every card carries its token cost, child agents nest inside their parent's card, and finished cards persist into chat history instead of vanishing. A live "now" line surfaces what each agent is thinking mid-run.
- **Split panes stopped sharing clothes.** Model, effort, and the context ring are per-pane now — picking a model in one pane never restyles the other. Per-chat effort is remembered just like the model.
- **Autocorrect actually corrects now.** Beyond the known-misspellings list, it fixes *any* typo against a 50,000-word frequency-ranked dictionary (edit-distance, most-common-wins) — `wrok`→work, `responsibilty`→responsibility. It still refuses to touch real words, code identifiers, ALL-CAPS, names mid-sentence, or dev/chat slang.
- **The effort slider got dressed.** A slim pill track in the same language as the toggle switches, an accent fill that trails the thumb live while you drag, and crisper dots/thumb all around.
- **Steer markers show attached images** as thumbnails right on the marker, with a lightbox.

## v0.141.0 — Every model, honest compaction radar

- The model picker carries the full Claude lineup (Opus/Sonnet 4.6+4.5, Haiku 4.5 back, Opus 5 "Coming soon" teaser), auto-compaction detection reads your real CLI settings, sub-agent cards stream agent text on CLI 2.1.211+, and split/chrome polish landed. Recommended CLI: 2.1.214.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
