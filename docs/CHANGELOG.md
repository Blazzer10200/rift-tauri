# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.34-alpha-test — 2026-05-12 — 12-hour time format everywhere

Blazzer can't stand 24-hour timestamps. Audited all date/time render paths in the UI and forced explicit `hour12: true` on every `toLocaleTimeString` and `toLocaleString` call. Previously three callsites passed `hour12: false` (24-hour), and four used `toLocaleString()` with no options (locale-default — would be 24-hour on non-US machines).

### Landed
- **Forced 12-hour on explicit 24-hour sites** ([ActivityFeed](src/lib/components/activity/ActivityFeed.svelte#L83), [Diagnostics](src/lib/components/diagnostics/Diagnostics.svelte#L63), [StatusHero](src/lib/components/StatusHero.svelte#L11)). Flipped `hour12: false` → `hour12: true`.
- **Pinned 12-hour on locale-default sites** ([RemotePane](src/lib/components/browser/RemotePane.svelte#L85), [LocalPane](src/lib/components/browser/LocalPane.svelte#L86), [DriftReview](src/lib/components/drift/DriftReview.svelte#L260), [ConflictResolver](src/lib/components/conflicts/ConflictResolver.svelte#L34)). `toLocaleString()` → `toLocaleString([], { hour12: true })` so non-US locales still display 12-hour.

### Coverage
All display surfaces affected: live Activity feed times, Diagnostics event log, StatusHero last-activity badge, browser pane mtimes (local + remote), DriftReview entry mtimes, ConflictResolver side-by-side mtimes. Internal `toISOString()` storage (SyncModal activity rows, diagnostics export `generated_at`) is untouched — those are wire format, not display.

### Verify
- `svelte-check`: 0 errors, 2 pre-existing warnings. No backend changes → cargo not re-run.

v0.2.33 archived to git log.
