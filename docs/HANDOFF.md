# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 96 — 2026-05-18 — v0.4.10-alpha: workspace shell

Replaces the v0.2 / v0.4.1 dual-shell with a single workspace-swap shell — the 40px activity bar is now the navigator and each icon swaps the entire main pane. Eight reachable workspaces (Chat default · Sync · Files · Conflicts · Diagnostics · Terminal · Activity · History) plus Agents + Attachments disabled "Coming soon" tiles. Settings gear at the bottom of the rail (was palette/Ctrl+, only). ChatTabsBar mounts only inside Chat workspace.

Ship from [docs/design/workspace-shell.md](design/workspace-shell.md) — 3 commits (Phase 1 scaffold → Phase 2 cutover → Phase 3 demolition). Verified by [`scripts/cdp/smoke-v04-10.sh`](../scripts/cdp/smoke-v04-10.sh) — 74 PASS / 0 FAIL across shell/ab/default/swap/stub/settings/kbd/storage sections plus grep-cleanliness + file-deletion checks at Phase 3. ~956 LOC deleted (TabRail, RightPane, right-pane/*, panel-types, useV03Shell), ~150 LOC added (workspaces/, WorkspaceShell, DisabledWorkspace).

Plan §10 open questions resolved by implementer (commits document rationale): Diagnostics + Sync badges skipped (no reactive frontend count); HistoryDrawer overlay/scrim/close-X dropped (workspace fills full pane); min-width unchanged (tauri.conf.json has no explicit min); TerminalPanel divider/resize/collapsed-strip removed; `Ctrl+Shift+D` now routes to Diagnostics (was Activity stand-in).

**Pending:** (a) **Velopack delta chain investigation — STILL TOP PRIORITY** carried over from S95; v0.4.10 ship should reset the delta chain. (b) Trey-config sync. (c) Prune dead CSS warnings in TerminalPanel.svelte (~15 collapsed-strip / divider rules) + AssistantHeader (hdr-btn.active, convo-chip) + TasksDock (dock-head/title/closebtn) — cosmetic, doesn't affect runtime. (d) Drop unused width-resize handlers from TerminalPanel (panelEl is still bound but never resized).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.10-alpha** (S96 pending commit; v0.4.8 binary live in `rift-releases`). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**Workspace shell** = single shell, no fallback. Activity bar on right swaps main pane. 8 reachable workspaces + 2 disabled stubs + settings gear. Order persists to `rift.ui.workspace-order.v1`; active to `rift.ui.workspace.v1`.

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice:** Settings → Speech (STT only); v0.4.5 picks highest-conf of 3 alternates. **A11y:** Settings → Accessibility (dyslexia-friendly mode, font, spacing, warm tint — v0.4.7).

**v0.2 queue** (needs `/grill` or `/plan`): auto-Mirror on rename; dry-run Mirror preview; EACCES auto-fix-perms; `lib.rs`→`commands/*.rs` split (1790L); LocalPane/RemotePane base extract; integration tests phase 1. A11y stretch: SymSpell+Metaphone "did you mean" pill; STT vocab hints / Azure-direct.

**Audit queue:** 6 LOW lib/config, upstream-blocked. See [docs/AUDIT.md](docs/AUDIT.md).

**Multi-user:** Trey OFF Mirror until on-latest. [docs/TREY-SETUP.md](docs/TREY-SETUP.md). v0.4.7 auto-updates him.

**Don't reintroduce:** dock primitive, maximize-to-center, `PanelState.slot`, `dockSplitPct`, Tasks-as-peer, AddPanelMenu, TabRail / v0.2 tab-rail shell, OpRail/TopBar, RightPane sidecar + width-resize, `useV03Shell` toggle + storage key, whisper-rs (libclang dep), `msedge-tts` / TTS / speaker UI.

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
