# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 104 — 2026-05-19 — Multi-tab send + thinking effort + live counter (live binary swapped)

**Effort ladder + live thinking timer** — Composer pill cycles Fast/Quick/Deep (`--effort low|medium|high`), persisted per-conversation + localStorage ([Composer.svelte](../src/lib/components/assistant/Composer.svelte)). `MAX_THINKING_TOKENS` env replaced with the documented `--effort` CLI flag in [mod.rs:1132](../src-tauri/src/assistant/mod.rs#L1132). `ThinkingBlock.startedAt` + live `elapsedFor` in [MessageBubble.svelte:210](../src/lib/components/assistant/MessageBubble.svelte#L210) — role-row ticks `5s → 10s → 17s...` during active reasoning instead of frozen "Thinking …" for 17-40s.

**Multi-session backend** — `CURRENT_CHILD_PID` + `USER_STOPPED` singletons replaced with per-session `SESSION_PIDS: Mutex<HashMap<String,u32>>` + `SESSION_STOPPED: Mutex<HashSet<String>>` ([mod.rs:32-90](../src-tauri/src/assistant/mod.rs#L32)). All `assistant://stream|done|error` events now carry `session_id`. `assistant_stop(session_id)` kills only that session — Tab A's stop no longer kills Tab B's stream.

**Multi-tab send** — Frontend `streaming` singleton replaced with `streamingSessions: Set<string>` + getter/setter delegating to active `currentCliSessionId` ([assistant.svelte.ts:197-230](../src/lib/state/assistant.svelte.ts#L197)). `stop()` pre-clears state synchronously before invoking backend so late done events are idempotent. **Tab B can now send fresh while Tab A is mid-stream** (no more queue-onto-wrong-tab bug from S104 screenshot). Concurrent live streaming on 2+ tabs at once is still v0.4.1 (current code kills old stream on tab switch — existing behavior).

**Per-tab UI cache** — `tabDrafts`/`tabAttachments`/`tabScroll` maps preserve composer state across switches. Stash on outgoing, restore on incoming, prune on close ([assistant.svelte.ts:344-383](../src/lib/state/assistant.svelte.ts#L344)). `AssistantPage` $effect watches `currentConvoId` and restores scrollTop per tab.

**Crash-flush** — `flushNow()` fire-and-forget IPC on `beforeunload` closes the 700ms `scheduleSave` debounce gap for clean window-close.

**Live binary swapped** — release build → `C:\Users\BLAZZER\AppData\Local\Rift\current\rift-tauri.exe`. Backup at `.bak-S104`.

**Verified live via CDP:** Tab B `6×7 → 42` while Tab A streams a 5-paragraph story — placeholder shows "Ask Claude" not the queue message; no cross-talk; per-tab composer draft survives switch (typed → switch → switch back → restored verbatim).

---

> S101–103 (compaction design + A2 cli_session_id decoupling + chat rhythm overhaul w/ EditDiff/ToolChip/StepGroup) → see `git log -- docs/HANDOFF.md` for full notes. S100 + S96–99 archived to [docs/archive/HANDOFF-archive.md](archive/HANDOFF-archive.md). Velopack watch: any NSIS-then-Velopack machine may have stale root binary.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.11-alpha**. Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser. Live binary swapped S104 (multi-tab send + effort pill + live counter); no Velopack push.

**Workspace shell** = single shell, no fallback. Activity bar right, 8 reachable workspaces + 2 stubs + settings. Order → `rift.ui.workspace-order.v1`; active → `rift.ui.workspace.v1`.

**CDP autonomous-verify** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Active work:** (a) **Compaction** — plan at [docs/design/assistant-compaction.md](design/assistant-compaction.md). A1 3/5 + A2 done. Next: A3 (whitelist `result.subtype`), A4 (`auto_compact_threshold`/`compact_model` in `AssistantConfig`), A5, then Phase B. (b) **Concurrent live streaming** — multi-tab send works (S104); true concurrent streaming on 2+ tabs needs per-tab `messages`/`activity`/`tasks` buffers (~300 LOC, deferred). (c) **HANDOFF cap** — file just trimmed S101–103 into git-log; future sessions, follow suit.

**v0.2 queue:** auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split (1790L); LocalPane/RemotePane base extract; integration tests phase 1.

**Audit queue:** 6 LOW lib/config, upstream-blocked. See [docs/AUDIT.md](docs/AUDIT.md). **Multi-user:** Trey OFF Mirror until on-latest. [docs/TREY-SETUP.md](docs/TREY-SETUP.md).

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail / v0.2 tab-rail shell, OpRail/TopBar, RightPane sidecar + width-resize, `useV03Shell` toggle, whisper-rs, `msedge-tts` / TTS / speaker UI, `MAX_THINKING_TOKENS` env (was a no-op, replaced w/ `--effort`).

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only (NASM blocks aws-lc-rs). russh `Config{keepalive 20s/3, window 2 MiB, packet 32 KiB}` in `sftp::open_session`+`tunnel::start`.
- `~/.rift/*.json` compat — keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher conflict-rename guard — never overwrite dirty local. `.rift-trail.jsonl` ignore rule mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` = `std::sync::Mutex` (NOT tokio). `force_pull_now`/`force_push_now` invariants preserved.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. Upload pre-flight SHA-collapse before CONFLICT. `DriftBucket::ToDelete` deletes LOCAL; `ToDeleteRemote` deletes REMOTE (mirror+baseline gated).
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- **v0.2.56:** Assistant tab self-execs MCP via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop.
- **v0.4 chat tabs:** `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt` (NOT `currentConvoId`). Chat-tab keybinds (Ctrl+T/W/Tab, Alt+1..9) gated on `workspace.activeId === "chat"`.
- **v0.4.10 workspaces:** registry in [workspaces/index.ts](../src/lib/components/workspaces/index.ts). ActiveId persists to `rift.ui.workspace.v1`. ChatTabsBar mount gated on `workspace.activeId === "chat"`. DisabledWorkspace renders Agents/Attachments — do NOT remove these stub entries until real components ship (registry breaks if WorkspaceId enum members vanish).
- **Activity bar layout:** top group (10 workspaces, reorderable, drag-persist to `rift.ui.workspace-order.v1`) + bottom group (settings gear, fixed). Adding a workspace = add to WorkspaceId enum + WORKSPACES registry + DEFAULT_ORDER + smoke test indices.
- **S87 context pill:** `recordTurnUsage(u, accumulate)` — only `result` envelope updates `sessionUsage`; both refresh `lastTurnUsage`. Effective ctx = `input + cache_read + cache_create`. `[1m]` suffix = 1M window.
- **S87 image paste:** `assistant_send` flips `--input-format text → stream-json` when attachments present. 20 MiB cap + `image/*` gate.
- **S91 allowlist + S92 mode:** `assistant_send` MUST keep `--permission-mode bypassPermissions` (NOT `dontAsk` — that auto-denies MCP calls) AND the full `BUILTINS` const in `--allowed-tools` (Agent/BashOutput/KillBash/SlashCommand etc) across all three branches. Both gates required; either change → per-tool denials in the Assistant.
- **S88 STT:** WebView's `SpeechRecognition` writes directly to `assistant.composerDraft`. `baseDraft` snapshot on start preserves pre-existing text in append mode. `errno("not-allowed")` → friendly mic-perm message. Settings section id=`"speech"` (was `"voice"`). TTS is fully removed — do not reintroduce.
