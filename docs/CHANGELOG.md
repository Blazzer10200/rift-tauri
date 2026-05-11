# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.16-alpha-test — 2026-05-11 — Manual transfer activity events

Manual `download_paths` / `upload_paths` were silent — no Activity rows, no toasts. Buddy ran an initial pull and had no way to tell if it was working, half-failed, or hung. Auto-sync ops already emitted `autosync://activity`; manual ops now do too, reusing the same event stream + existing Activity tab + ActivityToast.

### Landed

- `download_paths`: emits 3 `ActivityRow` events — `download started` (before job expansion), `downloading N files` (after expansion, before transfer), and `download complete N/N` or `download partial N/M` after completion. Cancellation emits `download cancelled`.
- `upload_paths`: emits `upload started` + `upload complete N/N` (or `upload partial`).
- Events flow into the existing Activity tab + ActivityToast — no Svelte changes needed.

### Verify

- `cargo check --release`: clean (3.83s incremental).
- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated.

Future: per-batch progress during long transfers — current pass is start/end only, but the Activity row + Pending counter on the Browser page already give meaningful "is it still going" feedback.

v0.2.15-alpha-test archived to git log.
