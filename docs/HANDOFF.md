# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `git log -- docs/HANDOFF.md`.

## Session 43 — 2026-05-12 — Backend hardening + frontend cleanup pass

Parallel split. **Backend (Codex):** 15 items applied per `docs/audit/codex-fixes-2026-05-12.md`. Symlink-safe `delete_recursive_via` (lstat per child, reject `/`/`.`/empty); rename collision split (`rename_via` preflights existence, only atomic upload calls `rename_overwriting_via`); per-path `OpStatus { ok, error }` on remote/local delete commands; profile-path containment on rename/delete/list/bootstrap/enqueue/conflict-resolve; autosync deadlock avoidance (clone engine before `.await` to drop mutex); retry map drop-after-final-failure; 64-bit transport temp-id entropy; CRLF-safe `edit_trail` trim. Connect-time write probe under `remote_root` before declaring SFTP healthy. `.rift-lock` sweep on watch attach (local-user owned, age-gated). Removed 3 dead pub fns. **File splits:** `sftp/mod.rs` → mod+list+transfer+ops+remote_exec (1348L → 286+373+202+73+320 = 1254L, public API unchanged); `auto_sync/path.rs` extracted (152L); `auto_sync/{watch,flush}.rs` left as 1-line `CODEX-FLAG` stubs — private state cross-cuts notify/queue/lock/cache/bridge/trail, mechanical move judged unsafe. Skipped (with reasons): `local_list_dir` containment (no server_key param — frontend contract change), log redaction / CSP / capability tightening / safe file-count cache / tunnel per-connection cancellation (product or cross-cutting decisions).
**Frontend (Claude):** 5 new audit fixes from `scan-frontend-2026-05-11.md` (AddServer destroy-guard, ConflictResolver redundant-effect removed, `updates.svelte.ts` swallowed catch surfaced, Diagnostics wire comment, `dirtyEdits` Set-replace invariant). 9 svelte-check warnings → 0 (StatusBar `.val.ok` dead CSS, `splitEl`/`pickerEl`/`panelEl` → `$state`, split compound `<!-- svelte-ignore -->` per rule — Svelte 5 quirk drops 2nd rule from single-line). ConflictList S39 dev seed stripped (HANDOFF flag cleared). TwoPane many-tabs horizontal scroll + 20px edge fade mask. `app.css` dropped 5 unused rules: `.btn.lg` `.pill.warn` `.pill.xs` `.vdivider` `.count-pip.warn` (verified via dynamic-class grep of `connCfg.cls` and `t.countCls`).
**Repo cleanup:** `scripts/bg-backlog.sh` deleted (stale Session 30 backlog, `--bg` never rolled out); `src-tauri/icons/{android,ios}/` deleted (desktop bundle only); `Releases/` pruned 44 → 10 nupkgs (~194 MB freed, last 5 versions kept).
**Verify:** `cargo check` 0 errors · `npm run check` 0/0/3994 · `npm test` 6/6.

**Files:** `docs/audit/codex-fixes-2026-05-12.md`, `src-tauri/src/sftp/{mod,list,ops,transfer,remote_exec}.rs`, `src-tauri/src/sync/auto_sync/{path,watch,flush}.rs`, `src-tauri/src/{lib,path_guard}.rs`, `src-tauri/src/sync/{auto_sync,lock_presence,edit_trail}.rs`, `src-tauri/src/transport/env.rs`, frontend: `app.css`, `AddServer.svelte`, `ConflictList.svelte`, `ConflictResolver.svelte`, `TwoPane.svelte`, `TerminalPanel.svelte`, `LocalPane.svelte`, `RemotePane.svelte`, `StatusBar.svelte`, `Diagnostics.svelte`, `state/{updates,connection,diagnostics}.svelte.ts`.

## Earlier sessions — compressed

**S42** Settings rework: 7 sections → 4. Killed Design tokens/Sync/Editor; stripped dead Direction/Mode/Font from Appearance — Density no-op so shipped Sparkles "Coming soon" card. Server-card inset-stripe canon, SSH keys inline header, About updates btn `.btn ghost sm`. Sub-page swap mirrors TabRail (`fly y:6 d:180 delay:90 quintOut` in, `fade 90ms` out, keyed on `section`); `.body` `position:relative overflow:hidden`, `.sub-shell` absolute inset:0. `SettingsSection` narrowed in AppShell. Net: 187 → 54 lines on `Settings.svelte`. **S41** Terminal A.2: panel promoted `TwoPane` → `AppShell` (persists across tabs), Ctrl+\` global. New `state/terminal.svelte.ts` store + `TerminalPanel.svelte` w/ split-button picker dropdown (**Presets** Claude Code/Codex + **Shells** git-bash/pwsh/ps/cmd), per-tab close + close-all trash, collapsed strip dynamic-labels w/ active shell + N-tabs pip. Backend `term_spawn` → `SessionStartInfo { id, shell_id, shell_label }`. Per-user localStorage: open/height/defaultShell/fontSize/autoLaunch/tabs/activeIdx. Per-tab auto-launch (250ms post-spawn `cmd\r` write). Restore dedupes by `(shellId, autoLaunch)` + caps at 4 — testing burst of 14 git-bash tabs collapses to 1 on relaunch. WebView2 cache HRESULT 0x8007139F recovery (rename `EBWebView` dir). `TwoPane` stripped of ~260 LOC terminal code. Store APIs (`setDefaultShell`/`setFontSize`/`setAutoLaunchCommand`/`closeAllTabs`) ready when Settings → Terminal sub-view ships. **S40** Terminal MVP build fixes (PathBuf drop, `use tauri::Manager` for `try_state`), collapsed-state floating-pill→integrated-24px-strip redesign, xterm scrollbar canon, dead CSS sweep. **S39** Conflicts polish (tone-coded bulk bar, inset-stripe selected rows, resolver meta cards, slot-in `animate:flip`). **S38** Browser polish + embedded terminal MVP (portable-pty + xterm.js). **S36-37** ActivityFeed canon. **S31-35** post-WPF cleanup, OpRail kill, Titlebar+TopBar merge, palette, tone-coded TabRail, `.btn` skeleton. Full diffs: `git log -- docs/HANDOFF.md`.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri (Rift). Path: `C:/AI Workflow/projects/rift-tauri/`. Version **v0.2.40-alpha-test** — hotfix for list-on-remote-root regression introduced in 0.2.39. v0.2.39 prerelease stays up (delta-receivers already on 0.2.32 → 0.2.40 still works via fresh delta). Shipped 2026-05-12 via Velopack to `Blazzer10200/rift-releases`.

**Rhythm:** apply canon (`docs/UI-POLISH-MAP.md`) to remaining unpolished pages. Tone via `data-tone` + `--tone` var, surface 8-14% rest / 22% hover, hover icon scale 1.1-1.18 w/ overshoot + reduced-motion guard, active inset-stripe `inset 2px 0 var(--tone)`, focus-within blur, hide-when-zero, title+hint empty states, truncation tooltips, `hour12: true` everywhere.

**Polish done:** AppShell, Titlebar, TabRail, StatusHero, StatusBar, CommandPalette, TwoPane, LocalPane, RemotePane, PathBreadcrumbs, LockBadge, Confirm, Reupload, FlashToast, ActivityToast, ActivityFeed, ConflictList+Resolver, Terminal (S40) + TerminalPanel multi-tab (S41), Settings 4 sections (S42), `.btn` skeleton.

**Pending:** Tier 3 login flow (Bootstrap, AddServer, Keygen), Tier 4 modals (SyncModal, UpdateDialog), Diagnostics. Carryover: Terminal Settings sub-view (shell picker + font size UI — APIs ready on `terminal.svelte.ts` store, folds into Appearance coming-soon pass). Appearance density/font/accent-tint controls (future). `auto_sync/{watch,flush}.rs` extraction (S43 left stubs; cross-cut private state too risky for mechanical move).

**Don't reintroduce:** OpRail, TopBar (merged into Titlebar), rail kbd hints `⌘1`, StatusBar `⌘K` pip, titlebar Settings gear, StatusHero big H1, dupe "watching" words, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design tokens / Sync / Editor sections (S42-killed), `.btn.lg` / `.pill.warn` / `.pill.xs` / `.vdivider` / `.count-pip.warn` (S43-dead CSS), `bg-backlog.sh` (S43-removed).

**Flagged v0.2.40+:** `local_list_dir` profile containment (needs frontend contract change to add server_key), log redaction / CSP / capability tightening (product policy), safe file-count cache (watch-level invalidation design), tunnel per-connection cancellation (cross-cuts task ownership), `commands` `$derived` churn in AppShell (debatable value).

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs).
- `~/.rift/*.json` compat — don't change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` is strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- Source `.secrets/env.sh` first on ship/auth tasks — non-interactive bash won't auto-load.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — called from sync notify handler; tokio `blocking_lock` panics.
- `force_pull_now` does inline drift scan + dispatches w/ guard in front (post-v0.2.38). Don't "optimize" to cache-only — stale tombstones fire mass deletes.
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros for size/mtime/atime/uid/gid → file truncation + epoch mtime. Use `FileAttributes::empty()` + set only fields you want. (v0.2.27 post-mortem.)
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`. Don't shortcut to `remove_file` for "files only" — push pipeline can't distinguish ahead of time. (v0.2.29.)
- `mkdir_p_via` chmods each segment to 2775 — setgid + group-writable required for shared-group teammate pushes. Don't drop SETSTAT. (v0.2.31.)
- Upload pre-flight SHA-collapse before raising CONFLICT — sizes match + baseline SHA exists → hash local cheap, remote via SSH exec; if both = baseline, refresh baseline mtime + drop push. (v0.2.31 / v0.2.32 — 53 phantom conflicts.)
- `DriftBucket::ToDelete` is the tombstone path — `local + no remote + has_baseline` MUST classify as `ToDelete`, NOT `ToPush`. Routes to `delete_local_one` which guards on foreign-lock + dirty-local. Empty-parent walk-up post-delete. (v0.2.33.)
- All time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US. (v0.2.34.)
- Mass local-delete circuit breaker lives in `force_pull_now`: `(file_count * 0.30).clamp(5, 25)`, `BLOCKED — N local-deletes`, `kind=block`. Pull is the ONLY tombstone-propagation path post-auto-sync rip. (v0.2.36 / v0.2.38.)
- DO NOT restore `drift_watcher::spawn` / `run_tick` / `flush_cycle` / `auto_flush_enabled` / `remote_scan_interval_secs` / `drift_watcher_task` / `flush_task` / `LOOP_TICK_MS` / `track_pull_handle` / `register_scan_cancel` / `clear_scan_cancel` — deleted v0.2.38. Auto path ping-ponged `[world]` resources on 10s tick. Push/Pull buttons only. Watcher still runs to populate dirty queue. (v0.2.38 post-mortem.)
