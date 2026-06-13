# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.2 — 2026-06-13 — Concept-D tool-group cards + composer auto-correct

> **Why.** Continue the UI-polish arc — make multi-tool turns read as clean, scannable step cards instead of a wall of rows ("quality over quantity"), plus a small composer quality-of-life add. Frontend-only; backend identical to v0.9.0.

**Added:**
- **Tool-group step cards.** A run of 3+ tools collapses into one status-railed card (left rail green = done / accent = running / red = error): quiet + collapsed on success, loud + auto-open on failure. Quick (<3s) inter-tool thoughts are now absorbed into the run, so interleaved-thinking turns actually group — previously a thought between every tool broke every run and nothing grouped. Per-group wall-clock duration shows in the head; the number moves into the card; collapsed bodies hide the absorbed thoughts (tools only on expand). Error groups stay loud-by-default but are now collapsible, not force-open.
- **Auto-correct** context-menu item on text fields — a deterministic local pass (common typos, sentence-start + standalone "i" capitalization, space collapse) that corrects the selection or the whole field. Undoable, no model round-trip.

**Verify.** svelte-check 0/0 · vitest 52/52 (helpers + playback) · backend unchanged from v0.9.0 · CDP-verified live: grouping across thoughts, collapse/expand, per-group duration, error-collapse, and the auto-correct correction.

## Older versions

v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
