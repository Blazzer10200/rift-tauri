# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.3-alpha — 2026-05-09 — Audit fix-pass round 2: 14 more findings landed

Continuation of the 2026-05-09 audit sweep. Round 1 (v0.2.2-alpha) closed 28 findings; round 2 closes another 14 across the medium and low tiers, plus a defense-in-depth path-traversal extension. 18 audit findings remain (mostly LOW/style).

### High-impact
- **H2** `refreshStatus()` no longer swallows IPC errors silently — `console.error` surfaces channel-closed / serialization failures.
- **H9** `askConfirm()` Promise leak fix — pending confirm resolvers tracked in a Set; `onDestroy` resolves them all to `false` so awaiting code paths unblock if the shell unmounts mid-dialog.

### Medium
- **M2** `reject_path_traversal` guard extended to `local_list_dir`, `enqueue_for_flush_batch`, and `resolve_conflict`. Defense-in-depth — any path with `..` components from JS is rejected at the boundary.
- **M5** `state/discovery.rs` and `state/remote_state.rs` mutex-poison recovery now logs an `error!` before falling through. The original poisoning panic is no longer hidden.
- **M9** `loadServers()` fetches `list_servers` and `get_last_selected` in a single `Promise.all` round so `selected` doesn't briefly resolve to `null` between awaits.
- **M12** `ConflictResolver` "Edit in editor" switched from raw `invoke("plugin:opener|open_path")` to typed `import { openPath } from "@tauri-apps/plugin-opener"`. Failures now surface in the dialog instead of being swallowed.
- **M18** `TopBar.connCfg` wrapped in `$derived.by(...)` so detail strings re-evaluate when fingerprint / status.detail change. Was a script-eval-time snapshot before.
- **M21** `DriftReview` `selected` is pruned via `$effect` whenever the `selectableIds` derived list changes (sideFilter / grouping / filtered shifts). `applyPushPull` no longer operates on entries the user can't see.

### Low
- **L3** `Cargo.toml` velopack pin tightened from `"0.0"` to `"=0.0.1298"`. 0.0.x crate families have shipped breaking changes between patches.
- **L9** Verified via `cargo tree -d`: ring is the canonical crypto stack, but rustls-webpki transitively pulls aws-lc-rs through `rustls-platform-verifier`. **Documented for next round** — needs a feature-flag investigation to fully de-dupe; not a blocker.
- **L12** `browser-tabs.svelte.ts` `savePersisted` empty catch upgraded to `console.warn`.
- **L13** `Reupload.svelte` Enter-key always picks `"reupload"` (one-time) regardless of the "Always" checkbox state. Persistent default now requires an explicit button click — prevents accidental pin while typing.

### Verify
- `cargo check --lib` — clean, only the known russh future-incompat note.
- `cargo clippy --lib --tests` — clean.
- `npm run check` — 3933 / 0 errors / 1 advisory (intentional Settings:18).

v0.2.2-alpha archived.
