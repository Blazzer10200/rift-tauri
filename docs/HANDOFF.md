# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 29 — 2026-05-12 — v0.2.33-v0.2.34: deletion propagation + 12-hour time format

Two ships.

### Ship trail (newest first)
- **v0.2.34** — 12-hour time format everywhere. Audit found 3 sites passing `hour12: false` (ActivityFeed, Diagnostics, StatusHero) + 4 sites using locale-default `toLocaleString()` (RemotePane, LocalPane, DriftReview, ConflictResolver). All forced to `hour12: true` explicitly so non-US locales also get 12-hour. Internal ISO storage untouched.
- **v0.2.33** — Deletion propagation. Blazzer deleted `gt_zombies_qb` locally → remote delete propagated ✅ → Trey refreshed but file lingered on his side. Root cause: missing tombstone path in drift scanner — `local + no remote + has baseline` was misclassified as `ToPush` ("remote vanished — re-pushing local"), leaving ghost files on teammates' machines + risking accidental resurrection on next touch. New `DriftBucket::ToDelete` variant; scanner classifies on baseline (the tombstone). Watcher's `run_tick` + auto_sync's `force_pull_now` dispatch via new `delete_local_one`: foreign-lock defer, dirty-local skip+warn, else `fs::remove_file` + `snapshot.forget` + `cache.forget` + empty-parent-dir walk-up cleanup. SyncModal gained "To Delete" count cell.

### Verify (post-v0.2.33)
- `cargo check`: clean (8.33s). `svelte-check`: 0 errors, 2 warnings (pre-existing svelte-ignore quirk).

### Flagged for v0.2.35+ (carried from S28)
- **Pre-flight write probe** on autosync start — catch EACCES at connect time, not first push.
- **Token-slot race** in `register_scan_cancel` — overlapping `run_tick` mid-`force_pull_now` can shadow user op. Move to `Vec<CancellationToken>` w/ `cancel_all` if it bites.
- **Activity-feed row grouping** — bulk reconciles spam 30+ identical "pulled" rows. Collapse runs of same-resource same-action.
- **Modal copy update** — still says "Listing remote files…" but post-v0.2.28 the listing is ~1s.
- **DriftReview bucket-string mismatch** — manual review filters check `"ToPush"`/`"Conflict"` but serde rename_all = snake_case emits `"to_push"`/`"conflict"`. Likely shows all rows incl. Synced. Pre-existing, not in scope for v0.2.33.
- **`svelte-ignore` non-suppression** on `<section>` a11y warnings — investigate svelte-check version bump.

---

## Session 28 — 2026-05-12 — v0.2.27-v0.2.32: data-loss recovery → WAN speedup → perms parity → phantom-conflict killer

Post-S27 compaction continuation. **Six ships** this session, all driven by Trey running Rift on residential Tailscale uplink — every bug surfaced because WAN latency + cross-user shared-group semantics exposed assumptions LAN-only testing hid.

### Ship trail (newest first)
- **v0.2.32** — Phantom-conflict killer. Trey's diag export showed 53 phantom CONFLICTs on `[ox]/web/build/` UI bundles where `local_size === remote_size === last_known_size` (bytes identical, only mtimes drifting). Drift scanner already SHA-collapses this shape on scan, but the **upload pre-flight** at [auto_sync.rs](src-tauri/src/sync/auto_sync.rs) had no such guard. Added SHA-equality collapse: when sizes all match + baseline SHA exists, compute local SHA → if it matches baseline, fetch remote SHA via SSH exec → if it also matches, drop the push as `synced (mtime jitter)`. Real edits skip the SHA path entirely. Conflicts are in-memory only (not persisted) → Trey's 53 disappear on relaunch.
- **v0.2.31** — Directory perms parity (the other half of v0.2.26). v0.2.26 chmod'd files (0664) but never dirs — new dirs landed at umask-0022 default (0755), so teammates couldn't push into dirs the other person created. `mkdir_p_via` now chmods each segment to **2775** (setgid + group-writable) via `FileAttributes::empty()`. New helper `SftpClient::heal_owned_dirs(root)`: `find <root> -type d -user "$(id -un)" -exec chmod 2775 {} +` runs fire-and-forget on every `add_folder_watch`. Backlog cleanup for dirs Rift created pre-v0.2.31.
- **v0.2.30** — Whole-codebase audit. Clippy 1→0 (`io_other_error` in paths.rs). Svelte-check 5→2 (untrack in Settings + Escape handler in ctxmenu wrappers). Remaining 2 `<section>` warnings persist due to known svelte-check directive quirk. Zero TODO/FIXME/HACK. One `#[allow(dead_code)]` (intentional SSH session-keeper). No orphan modules.
- **v0.2.29** — Folder-delete fix. `SftpClient::delete` only called `remove_file` (SFTP rejects dirs → `No such file`). Now probes `remote_stat`: dirs → `delete_recursive_via`; missing remote → success (avoids re-queue loop).
- **v0.2.28** — Server-side `find`-exec listing in [sftp/mod.rs](src-tauri/src/sftp/mod.rs) `list_via_exec`. One SSH-exec round-trip per root vs N round-trips per dir in SFTP. Trey's scan latency dropped **30-60s → ~1-3s**. Falls back to SFTP worker path on per-root failure. Exit 0+1 both tolerated.
- **v0.2.27** — **CRITICAL data-loss fix.** v0.2.26's `set_metadata(0o664)` used `FileAttributes::default()` which (in russh-sftp 2.1.2) returns `size: Some(0), mtime: Some(0), atime: Some(0), uid/gid: Some(0)`. SETSTAT honored those → every Trey upload truncated to 0 bytes + epoch-1970 mtime instantly. `fxmanifest.lua` + `server/server.lua` + `client/client.lua` in `[endure]/endure_shooting/` zeroed in the live FiveM tree. Blazzer restored via FiveM session. Fix: `FileAttributes::empty()`. Bonus: scan Cancel now races `list_recursive_batch` via `tokio::select!`.

### Tailscale diagnostic (capture for future ref)
Trey's `tailscale status`: `direct 69.50.245.28:41641, tx 285M rx 533M` — direct P2P, not DERP-relayed. `netcheck`: `UDP: true`, `MappingVariesByDestIP: false`, `PortMapping: UPnP+NAT-PMP+PCP`. Router permissive. SFTP protocol overhead was the entire bottleneck — v0.2.28's exec-listing addressed it directly.

### Verify (post-v0.2.32)
- `cargo check`: clean (1.20s). `cargo clippy --no-deps`: clean. `svelte-check`: 0 errors, 2 warnings (svelte-ignore non-suppression quirk, documented).
- Releases v0.2.27-v0.2.32 on `rift-releases`. All source commits pushed (`2a964b4` → `04aa48f`).

### Flagged for v0.2.33+
- **Token-slot race** in `register_scan_cancel` — overlapping `run_tick` mid-`force_pull_now` can shadow user op. Move to `Vec<CancellationToken>` w/ `cancel_all` if it bites.
- **Pre-flight write probe** on autosync start — catch EACCES at connect time, not first push.
- **Activity-feed row grouping** — bulk reconciles spam 30+ identical "pulled" rows. Collapse runs of same-resource same-action.
- **Modal copy update** — still says "Listing remote files…" but post-v0.2.28 the listing is ~1s; should say "Comparing against snapshot…".
- **`svelte-ignore` non-suppression** on `<section>` a11y warnings — investigate svelte-check version bump.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`.

**Current state (post S29):** **v0.2.34-alpha-test SHIPPED** to `rift-releases`. Two ships: deletion propagation (v0.2.33) + 12-hour time format pin (v0.2.34). Both teammates need to relaunch to pick up v0.2.34.

**Next session likely entry points:**
1. Confirm Trey's relaunch + verify ghost files cleared from his tree (he should see "deleted (remote removed)" activity rows) + all timestamps display in 12-hour format.
2. Pick next item from v0.2.35+ flagged list (pre-flight write probe is cheapest), or move to brainstorm items (per-resource sync mode, buddy presence).

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` compat — don't change rename rules; don't drop `serde(flatten) extra`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard — never overwrite dirty local
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo
- `path_guard.rs` API frozen (`validate_remote_child`, `validate_local_child`) — `edit/in_place.rs` + lib cmds depend
- `rename_via` is strict (user-facing); `rename_overwriting_via` is ONLY for atomic upload tmp-swap
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load
- **`current_scan_cancel` + `last_scan_entries` are std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; tokio Mutex `blocking_lock` panics there. Don't "fix" it.
- **`force_pull_now` dispatches from cache, NOT a fresh scan** — re-scanning makes it identical to Reconcile (SFTP listing is the cost). drift_watcher's 10s tick keeps cache fresh.
- **NEVER use `FileAttributes::default()` for SETSTAT** — it sends `size: Some(0)`, `mtime: Some(0)`, `atime: Some(0)`, `uid/gid: Some(0)` which the server honors → file truncation + epoch mtime. Always use `FileAttributes::empty()` and explicitly set only the fields you want to change. See v0.2.27 post-mortem.
- **`SftpClient::delete` routes by remote stat** — dirs go through `delete_recursive_via`. Don't shortcut back to `remove_file` for "files only" — the push pipeline can't distinguish file from dir deletes ahead of time. See v0.2.29.
- **`mkdir_p_via` chmods each segment to 2775** — setgid + group-writable is required for shared-group teammates to push into each other's dirs. Don't drop the SETSTAT call — backlog gets healed too via `heal_owned_dirs` on watch attach. See v0.2.31.
- **Upload pre-flight SHA-collapse before raising CONFLICT** — when sizes all match + baseline SHA exists, hash local first (cheap), then remote via SSH exec. If both match baseline, refresh baseline mtime + drop the push. Mtime jitter (npm builds, SETSTAT, git checkout) flooded Trey w/ 53 phantom conflicts in v0.2.31; v0.2.32 fixed. See `auto_sync.rs:1522`.
- **`DriftBucket::ToDelete` is the tombstone path** — `local + no remote + has_baseline` MUST classify as `ToDelete`, NOT `ToPush`. Without it, deletes from teammates leave ghost files locally + risk accidental resurrection (autosync re-uploads on next touch). Dispatcher routes ToDelete → `drift_watcher::delete_local_one`, which guards on foreign-lock + dirty-local (skip unflushed edits — never blow away user's work). Empty-parent-dir cleanup walks up post-delete. See v0.2.33 post-mortem.
- **All time displays use `hour12: true`** — Blazzer requires 12-hour everywhere. Any new `toLocaleTimeString`/`toLocaleString` call MUST pass `[], { hour12: true }` explicitly (locale-default emits 24-hour on non-US machines). See v0.2.34 audit.
