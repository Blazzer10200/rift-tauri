# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.135.0 — Stream redesign

**The whole assistant transcript got a visual overhaul — every block now shares one quiet, cohesive look.**

- **Terminal blocks**: live commands type themselves in, long-running ones show an honest wait state with a riding cursor, and every settled shell gets an ok/failed pill. Output sits in a proper inset well under a flat header.
- **Diff cards** (Edited/Created): the loud green slabs are gone — quiet washes, monochrome-dimmed deleted lines, and creates no longer paint every row green.
- **Ask / plan / agent cards**: "Rift needs your input" is now a proper island card with a breathing live dot and tinted option tiles; plan and agent boxes match the same family, with done plan items striking through.
- **Tool drilldown**: the expanded work-line is a scannable index — each read/search row unfolds its result on demand into a clean inset panel, instead of dumping everything translucent at once.
- **Mid-turn notes**: redesigned as a neutral dashed pill — no more stray blue.
- **Smooth autoscroll**: the transcript glides to the bottom instead of teleporting (instant for reduced-motion and tab switches).
- Turn receipts on every settled turn (time · tokens · cost), and code fences get quieter headers with hover-only copy.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
