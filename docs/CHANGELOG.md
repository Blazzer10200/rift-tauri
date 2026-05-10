# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.12-alpha-test — 2026-05-10 — Drop top update banner

Sidebar pill (TabRail bottom-left) is the only "update available" affordance now — top banner felt heavy in the test build. Dialog still opens via the pill or Settings → About → Check for updates.

### Landed

- Removed `UpdateBanner.svelte` + the `banner-slot` grid track in `AppShell.svelte` (rows: `32px 44px auto 1fr 22px` → `32px 44px 1fr 22px`).
- Dropped `bannerDismissed` + `dismissBanner()` from `updates.svelte.ts` — no other consumers.

### Verify

- `cargo check`: clean. `svelte-check`: 0 errors.

v0.2.11-alpha-test archived.
