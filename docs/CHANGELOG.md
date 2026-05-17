# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.2-alpha — 2026-05-17 — Hot-fix: embedded Claude's `Skill` tool

Trey reported `/handoff`, `/check`, `/plan` etc. were rejected inside Rift's Assistant tab w/ "skills blocked." Root cause: `assistant_send` builds `--allowed-tools` as an explicit comma-list and `Skill` was missing from all three branches (full-config, scoped, scoped+remote-shell). The CLI's allowlist gate then denied the `Skill` tool even though `--disable-slash-commands` wasn't set. Added `Skill` to every branch in `assistant/mod.rs`; corrected the misleading `use_full_config` doc comment that claimed skills "always load via the CLI's own resolution" (true for command discovery, false for the tool gate).

Affects: every user of the embedded Claude in Rift since v0.2.56. Hot-fix only — no schema changes, no migration. Velopack delta is one Rust object.

## v0.4.1-alpha — 2026-05-17 — Right-pane refactor + audit fix-pass arc

Daily-driver use of v0.4.0-alpha surfaced two model mismatches, both corrected. `useV03Shell` toggle name + storage key reused for upgrade compat; v0.2 path stays pixel-identical.

### Tasks → AssistantPage (Phase 1)

Tasks is an Assistant property, not a Files/Sync peer. Dropped from `PanelId`/`PANEL_IDS`/`PRESETS`; `TasksPanel.svelte` deleted; `TasksDock` renders inside `AssistantPage` in both shells. Header Tasks toggle collapses to `assistant.ui.dockOpen`. Remaining kbd labels rebump (Sync → 1 … Activity → 7).

### ActivityBar + RightPane shell (Phase 2)

Right side = ONE full page at a time picked by a 40px vertical activity bar on the far edge (VS Code pattern). Files · Sync · Activity · Terminal · Agents · Attachments · History — drag-reorder via HTML5 DnD, persisted to `rift.ui.activitybar-order.v1`. `Ctrl+1..7` follows that order, `Ctrl+0` closes. `RightPane.svelte` lazy-mounts each page on first activate + everOpened-latches so scroll/selection/terminal session survive toggles. Left-edge resize 320..1200, dblclick snaps 50% (RAF-throttled).

`state/right-pane.svelte.ts` runs a one-shot storage migration: reads `rift.ui.panels.v1`, seeds `activeId` from the exactly-one-open panel (drops `tasks`), deletes the key. `dock-w.v1` → `right-pane-w.v1`. `dock-split.v1` / `maximized.v1` / `preset-picked.v1` / `dock-accordion.v1` all delete same boot.

Body grid `[chat | --right-pane-w (0 when closed) | 40px bar]`. `FilesPanel` always full `<TwoPane />`; `SyncPage` always full drift table + Mirror toolbar. `TerminalPanel` + `terminal.toggle()` route through `rightPane`.

### Dock primitive retired (Phase 3)

`Dock.svelte` + `PanelShell.svelte` + `PresetPicker.svelte` deleted. `ui-prefs.svelte.ts` 325L → 58L (only `density` / `railPinned` / `useV03Shell` survive). `PanelState` / `DockSlot` / `LayoutPreset` / `PRESETS` deleted. Settings → Appearance → Layout swaps "Reset dock split" → "Reset right pane" (`rightPane.reset()`); kbd cheat updated.

### Audit fix-pass arc (S80–S86 — 17 items closed)

S80 verification archived 16 items silently fixed by v0.3/v0.4 refactors. S81 (6): `editor_for` race-loss warn, `DiagBus` AtomicI64 lock-free, `DiagEvent.file` basename-only, `safe_profile_key` empty-sentinel, `lock_presence` stale-delete fail counter, `ssh_keygen` chmod 0o600. S82 (6): `RiftConfig::load` 1 MiB cap pre-parse, velopack-pin doc, `watch::try_watch` local_root forward-slash normalize, `sftp::close` lock-snapshot, `RIFT_UPDATE_FEED` gated `#[cfg(debug_assertions)]` (release builds cannot accept attacker-supplied local update feed), `in_place::Drop` detached thread. S83 (3): `LogForwarder::scrub_log_message` redacts `$USERPROFILE`/`$HOME` + OpenSSH/RSA/EC PRIVATE KEY markers; LocalPane/RemotePane `$effect` cleanup bumps `loadToken` for cancellation. S84: `compute_sha1` streams via BufReader (8 KiB buf, EINTR retry) — peak heap drops from O(N×file_size) to O(N×64 KiB). S86: `flush_batch` no longer awaits `safe_count_files` inline — `FolderCountCache { AtomicU64, AtomicI64 }` keyed by remote_root, 5-min TTL, per-batch `(created, deleted)` delta; `stop_watch` evicts.

### Session-lost auto-recovery + History dedupe

Long-idle resume sometimes hits "No conversation found with session ID:" even though the JSONL is intact on disk (claude's resume index drifts). Backend emits `assistant://session-lost {session_id, prompt}` on stderr-match AND non-first turn. Frontend pops the failed pair, nulls `convoCreatedAt` so next send uses `--session-id`, surfaces "Session was lost — retrying as a fresh start", re-sends. Tab-aware: ignored if user switched tabs while error was in flight. First-turn failures still go through normal error path.

Header History button now hidden under v0.4.1 (ActivityBar exposes it); v0.2 path keeps it (no ActivityBar there).

### S86 polish

`isHandshaking = $derived(connecting && status === null)` in `connection.svelte.ts` — Titlebar/StatusBar pills no longer flash "Connecting" during the post-handshake-pre-first-status window. Raw `connecting` still gates action buttons (anti-double-fire). New `lib/utils/file-display.ts` (`fmtSize`/`pickIcon`/`clampMenuPos`/`isEditableTarget`) shared by LocalPane + RemotePane (~68 LOC dedup).

### Verify

`npm run check` — 0 errors, 1 pre-existing warning (`.reasoning-meta.subtle`). `cargo check` — 0.50s clean. CDP smoke `scripts/cdp/smoke-v04-1.sh` green. v0.3 toggle OFF renders pixel-identical to v0.4.0-alpha's v0.2 path.
