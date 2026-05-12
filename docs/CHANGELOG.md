# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.30-alpha-test — 2026-05-12 — Codebase sweep: clippy clean + svelte warnings down 5→2

Whole-codebase audit pass — no functional changes, no behavioral risk. Sets a clean baseline for future feature work.

### Landed
- **`paths.rs`:** `std::io::Error::new(ErrorKind::Other, msg)` → `std::io::Error::other(msg)`. Picks up the `clippy::io_other_error` lint suggestion (Rust 1.95 idiom).
- **`Settings.svelte`:** `let section = $state<Section>(initialSection)` raised `state_referenced_locally` because the prop read happens inside `$state(...)`. Wrapped the prop in `untrack(() => initialSection)` — captures once on mount, doesn't shadow the prop name. Warning gone.
- **`LocalPane.svelte` + `RemotePane.svelte` ctxmenu:** added `onkeydown` handler on the `role="menu"` wrappers that fires `closeMenu()` on Escape. Removes the `a11y_click_events_have_key_events` warning AND gives users a real keyboard escape from the right-click menu — UX win in passing.

### Audit results (whole repo)
- **`cargo clippy --no-deps`:** clean, 0 warnings (was 1).
- **`svelte-check`:** 0 errors, **2 warnings** (was 5). Both remaining are the `<section tabindex>` non-interactive-element warnings on the file-browser panes; the `svelte-ignore` directive does not suppress `a11y_no_noninteractive_element_interactions` in current svelte-check despite the documented syntax. Tracked as a known svelte-check quirk, not a real defect.
- **`TODO/FIXME/HACK/XXX` grep:** zero hits across all `src/` and `src-tauri/src/`.
- **`#[allow(dead_code)]`:** one occurrence in `sftp/mod.rs` — the SSH session-keeper, intentional and documented.
- **Module wiring:** all `src-tauri/src/**/*.rs` files reachable through `lib.rs` `pub mod` declarations; no orphan modules.

### Verify
- `cargo check`: clean. `cargo clippy --no-deps`: clean. `svelte-check`: 0 errors / 2 warnings.

v0.2.29 archived to git log.
