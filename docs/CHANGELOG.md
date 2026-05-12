# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.48-alpha — 2026-05-12 — FiveM web/build false-deletes + new-resource debounce

### Bug 7 — Phantom `ToDelete-local` on FiveM `web/build/` + `web/dist/` trees

Endure RP stress-test 2026-05-12 surfaced 45 false "remote deleted — removing local" rows covering ox_lib/web/build (32 fonts + index.html + assets), ox_inventory/web/build (index + LICENSE + assets), oxmysql/web/build (index + vite.svg + assets), illenium-appearance/web/dist (index + assets/index.b8e72b46.js). User SSH-verified every flagged file existed on prod intact — the diff was wrong, and applying would have erased local ui_page bundles + propagated the erasure on next Push.

Root cause: asymmetric filter between local + remote walkers. `sftp/list.rs::list_via_exec` builds `find ... -type d ( -name build -o -name dist ... ) -prune -o -type f ...` from `ignored_directory_names()` — server-side `find` can't see path context, so it prunes every `build/` and `dist/` dir regardless of whether it's the FiveM-special `<resource>/web/build/`. Local walker uses path-aware `classify()` which correctly bypasses the FiveM trees, so local_map has the files, remote_map has zero — drift = ToDelete.

Fix in `sync/ignore.rs::ignored_directory_names()`: exclude `build` and `dist` from the server-prune list. Path-aware `should_ignore(rel)` in drift_scanner's remote_map construction still filters generic `app/build/foo` paths client-side. Cost: ~32 fonts of extra `find` traffic per FiveM web build, trivial. New unit assertions in `ignored_dir_names_excludes_brackets`.

### Bug 5 — Created+Dir → 500 ms-delayed coalesced reconcile

v0.2.47's immediate `kick_drift_reconcile` on Create(Dir) was firing BEFORE Windows had finished writing the files inside the new dir — scan walked an empty tree, surfaced nothing. v0.2.48 wraps the kick in `tokio::spawn` + `tokio::time::sleep(Duration::from_millis(500))` + `AtomicBool pending_dir_reconcile` coalesce flag. 50 rapid Create(Dir) events inside one new subtree collapse to one delayed reconcile that fires after the file writes land. compare_exchange owns the dispatch slot; lost-race events skip (the winner picks up their files).

### Verify

`cargo check` clean 2.89s · `cargo test --lib sync::ignore` 11 pass / 0 fail. All prior fixes preserved (v0.2.46 push-reliability stack, v0.2.47 prefix-ignore + Created+Dir base path).

### Still deferred to v0.2.49

- Mirror mode for Push-all (Bug 1) — new drift bucket for `local-missing + remote-has + baseline-exists` → propose remote-delete + UI toggle.
- Stale-lock sweep UI button.
- Mass-delete guard fine-tune.
