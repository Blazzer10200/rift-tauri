# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 115 — 2026-05-20 — Assistant page UI polish + #2 + #5

### Completed
- Merged AssistantHeader action chips into ChatTabsBar — single top bar, no duplicate + New button
- EmptyState: `justify-content: center`, 12vh→24px padding, dropped redundant workspace subtitle when card already shows it, suggestion cards now single-line ellipsised teaser
- Moved `.error`/`.notice` banners out of message scroll → sticky `.alerts` strip above Composer
- **#5** New `StatusHub.svelte` — spinner + live label + elapsed + Stop button, above Composer while streaming
- **#2** MessageBubble: removed in-bubble stream-status (redundant w/ hub); role-row turn-badge gated to streaming-only; completed turns get bottom `.turn-footer` (divider + model · cost)
- `AssistantHeader.svelte` left on disk but unused (dead code — delete explicitly if desired)

### In Progress
- v0.4.14-alpha still uncommitted (S113 + S114 + S115 changes on `main`)

### Key Decisions
- Hub pattern over in-bubble status: status strip lives where user's eyes are (near input), not scrolled-up inside the bubble
- Single tabsbar: chips parked on right of strip, + at far right — no separate header band

### Files Modified
- `src/lib/components/shell/ChatTabsBar.svelte` — absorbed action chips
- `src/lib/components/assistant/AssistantPage.svelte` — StatusHub + alerts strip wired
- `src/lib/components/assistant/EmptyState.svelte` — vertical centering + subtitle drop + compact cards
- `src/lib/components/assistant/MessageBubble.svelte` — stream-status removed, turn-footer added
- `src/lib/components/assistant/StatusHub.svelte` — NEW

---

## Session 114 — 2026-05-20 — Tier-2 FE + Sync MEDs

**FE (4/4):** #143 per-tab fields → TabState w/ store getters; #144 dropTab+pruneTabUi for removed tabs; #145 per-tab saveTimer + snapshot scheduleSave + flushNow iterates all tabs; #150 Settings $effect untrack. ensureTab precedes per-tab writes in send/openTab/newTab/closeTab.

**Sync (8 full + 2 partial):** #43 is_pushing safer-Err; #44 stop_watch unwatch-first; #45 FS-drop AtomicU64 + 100-drop Error (*partial*); #46 pending_dir_reconcile kick-before-clear; #48 force_pull_now poison emits diag; #57 download_paths open-before-CT; #61 TOFU write_probe None; #63 mutex into_inner (*partial*); #64 CLAUDE_EXE Mutex w/ is_file revalidate; #65 save_config tmp+rename.

**Sync deferred:** #47 (CT plumbing), #58 (DiagStage decision), #59+#60 (post-expansion guard), #62 (read/write token split — needs FE coord).

**Verify:** cargo check clean (1 pre-existing warn); npm run check 0/0 (4051 files). 13 files uncommitted on `main` w/ S113.

## Session 113 — 2026-05-20 — HIGH sweep

14/16 HIGH fixed: #34 #35 #36 #39 #40 #41 #74 #139 #140 #141 #142 #163 #219 #220. Deferred: #37 #38 (Phase 6). Agent shakedown showed operator dispatch wrong-priced for tightly-scoped single-file work.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source HEAD = **v0.4.13-alpha**; working tree carries v0.4.14-alpha batch uncommitted (S113 + S114 + S115). Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:** `/git-ship` the v0.4.14-alpha batch. Bumps three lockstep files (`package.json` + `Cargo.toml` + `tauri.conf.json`), writes CHANGELOG entry referencing #34-#220 + #143-#145+#150 sweeps + S115 assistant UI polish, commits + pushes. Origin still 3 commits behind from S112 — they ride along.

### After v0.4.14 ships

Priority queue:
1. #37 + #38 — Phase 6 OS-keychain (pair w/ #9.3).
2. Remaining sync MEDs: #47 #58 #59 #60 #62.
3. Wave-2 FE MEDs: #146 (mutateStreaming O(n)), #147 (thinking ref-eq), #148 (microtask race), #149 (openTab/delete), #151-#177 (a11y + lifecycle).
4. Strategic: Compaction Phase B, bg-tab session-lost retry, lib.rs → commands/*.rs split.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only (NASM/aws-lc-rs blocked). russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` = `std::sync::Mutex`. `force_pull_now`/`force_push_now` invariants preserved.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` (NOT dontAsk) + full `BUILTINS` in `--allowed-tools` across all three branches.
- TabState: per-tab field → add to TabState class + getter on AssistantStore. Never put per-tab state back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` when attachments present. 20MiB cap + `image/*` gate.
- Settings is now a workspace (kbd 9), `Ctrl+,` flips workspace; do NOT reintroduce the slideover scrim/aside. Dialog callbacks ride `src/lib/state/dialogs.svelte.ts`, populated by AppShell at mount.
- `list_watched_folders` Tauri cmd returns name + remote_root + cached file_count from `FolderCountCache`; lock count + last-event derived client-side from `connection.locks` + `connection.activityFeed`.
- Assistant scrollbar: `.scroll` + `.strip` BOTH set `scrollbar-width: none` + `::-webkit-scrollbar { display: none }` — don't reintroduce `scrollbar-gutter: stable`, leaks the WebView2 arrow-buttons on top-right. (#163 fixed in S113.)
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0` after init resolves. Don't reintroduce the empty-tabs CTA.
- `UpdateService` is managed Tauri state — register w/ `.manage(Arc::new(UpdateService::new(...)))` in `lib.rs::run()`. `apply_updates` is split: `download_update` then `apply_pending_update`.
