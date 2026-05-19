# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 103 — 2026-05-19 — Chat polish + Compaction Phase A (not released)

**Chat UI** — 5 improvements, all committed-WIP:
- **StepGroup slide-in** — 240ms→360ms, 4px→10px, ease-out-expo (`cubic-bezier(0.16,1,0.3,1)`), brief accent rail flash at entry, `rail-pulse` delayed 360ms on pending so they don't fight.
- **Stream-status sub-row** — activity label moved OUT of role-row into a dedicated `.stream-status` row below it. Single-line ellipsis-truncated, spinner inline, mono for `$ ...` commands vs italic for prose stages. Killed `.whim`, `.stage-row`, orphan `hasContent` derived.
- **Turn-badge right-anchored** — `margin-left:auto` + `white-space:nowrap` + `flex-shrink:0`; model/cost pill never wraps or gets pushed off-screen.
- **EditDiff blank-line collapse** — new `gap` kind; single blank ctx → 6px dashed spacer, 2+ consecutive blanks → `··· N blank lines ···` strip. Non-ctx blanks (intentional model edits) untouched.
- **Split-header tear fix** — `reconcileSplitHeaders` pre-pass detects `## S` → tool → `tep 1 — ...` model interleaving, reconstructs the first full header + keeps tool under the right step. `parseTextBlock` replaces `parseStepHeader` — returns all segment headers in one text block instead of just the first, so `## Step 1 ... ## Step 2 ...` in one chunk splits cleanly.

**CDP workflow** — [CLAUDE.md](../CLAUDE.md) now has a DOM-vs-`shot` decision table. Triggers that MUST fire a shot: user pastes UI screenshot, CSS edit lands, "look at the UI" prompt, before claiming UI done.

**Live binary updated** — release build swapped into `C:\Users\BLAZZER\AppData\Local\Rift\current\rift-tauri.exe`. Backup at `.bak-S102-S103`. No Velopack/release push.

**Compaction Phase A — in progress:**
- **A1 (3/5 probes)** — `--max-budget-usd` confirmed per-invocation flag ✓. One-shot `claude -p --resume <uuid> --model haiku --output-format stream-json` works, `result.usage` + cost present ✓. Multi-JSONL coexist in cwd dir ✓. Cold-cache finding: each one-shot pay ~$0.05 surcharge from SessionStart hook; fix = `CLAUDE_DISABLE_HOOKS=1` in child env. Probes 4+5 (kill-resume + system-reminder sentinel) queued for next session — need live dev UI driving.
- **A2 (complete)** — decoupled `currentCliSessionId` from `currentConvoId`. Frontend: new `$state` field, all 7 reset/mint paths updated, `send()` passes `currentCliSessionId` to backend, `loadConversation` restores with legacy fallback. Backend: `cli_session_id: Option<String>` on `Conversation` struct (`#[serde(default, skip_serializing_if)]`). Zero behavior change today — both IDs identical until Phase C remint.
- **A3/A4/A5 + Phases B–E** — not started.

**Compaction doc nits patched** — [docs/design/assistant-compaction.md](design/assistant-compaction.md): cost cap 0.50→1.50, C7/E2 archive contradiction resolved (full array stays in main convo JSON, `visibleMessages` is a derived view), B2 TodoWrite task interpolation added, 2 extra A1 probe items, cold-cache finding + `CLAUDE_DISABLE_HOOKS=1` mitigation documented.

---

## Session 102 — 2026-05-19 — Assistant chat rhythm overhaul (shipped, not released)

Multi-batch UI/UX pass on the Assistant chat. Committed only; `/git-ship` deferred (versions still v0.4.11-alpha).

**New:** [EditDiff.svelte](../src/lib/components/assistant/EditDiff.svelte) (jsdiff side-by-side, Edit/MultiEdit only); [ToolChip.svelte](../src/lib/components/assistant/ToolChip.svelte) (every non-Edit tool: per-tool kv input rows + terminal/code/list/plain result blocks); [StepGroup.svelte](../src/lib/components/assistant/StepGroup.svelte) (`Step N — title` → numbered marker + accent rail + status rings + auto-collapse).

**Modified:** [MessageBubble.svelte](../src/lib/components/assistant/MessageBubble.svelte) parses step headers, rolls up tool status, cycles whim words (12-word pool, 2.4s) in role-row when no per-tool label, auto-collapses done steps when ≥4 total (grid-template-rows 1fr↔0fr 280ms; last/pending/error stay expanded). [AssistantPage.svelte:101](../src/lib/components/assistant/AssistantPage.svelte#L101) scopes streaming to last assistant msg (caret leak fix); caret CSS removed. [AssistantHeader.svelte:55](../src/lib/components/assistant/AssistantHeader.svelte#L55) recognizes Sonnet 4.5/4.6 + Opus 4.6/4.7 as 1M context. [TasksDock.svelte](../src/lib/components/assistant/TasksDock.svelte) ~480→~135L (Tasks-only). [stt.svelte.ts](../src/lib/state/stt.svelte.ts) `consume()` kills post-send re-paste. [assistant/mod.rs:981](../src-tauri/src/assistant/mod.rs#L981) per-session toggles moved to user-turn `<system-reminder>` (cache-stable).

**Verified:** `npm run check` 0/0 throughout; CDP-driven probes per batch.

**Next candidates:** soft slide-in for new mid-stream steps; auto-expand the in-progress step as it advances. Skipped (info-loss): per-tool whim variants, full chain rail.

---

## Session 101 — 2026-05-19 — Compaction design research (no code shipped)

Plan at [docs/design/assistant-compaction.md](design/assistant-compaction.md). A1 probes + A2 code picked up in S103.

---

> S100 (Velopack stub fix) + S96–99 (workspace shell + assistant fixes) archived to [docs/archive/HANDOFF-archive.md](archive/HANDOFF-archive.md). Velopack watch: any NSIS-then-Velopack machine may have stale root binary (shortcut launches old version even after successful update).

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.10-alpha** (committed — 3 commits; v0.4.8 binary live in `rift-releases`, not yet shipped via Velopack — delta chain investigation required first). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

**Workspace shell** = single shell, no fallback. Activity bar on right swaps main pane. 8 reachable workspaces + 2 disabled stubs + settings gear. Order persists to `rift.ui.workspace-order.v1`; active to `rift.ui.workspace.v1`.

**CDP autonomous-verify live** — `run-dev.bat` sets WebView2 port; `npm run cdp:serve` on 9223; `scripts/cdp/c.sh state|eval|type|click|wait|shot|key`.

**Voice:** Settings → Speech (STT only); v0.4.5 picks highest-conf of 3 alternates. **A11y:** Settings → Accessibility (dyslexia-friendly mode, font, spacing, warm tint — v0.4.7).

**Active work:** (a) **Compaction (S101/S103)** — full plan at [docs/design/assistant-compaction.md](design/assistant-compaction.md). A1 3/5 done, A2 complete. **Next: A3** (whitelist `result.subtype` at [assistant.svelte.ts:~1380](../src/lib/state/assistant.svelte.ts#L1380)), then A4 (add `auto_compact_threshold`/`compact_model` to `AssistantConfig` in [mod.rs:177](../src-tauri/src/assistant/mod.rs#L177)), then A5 (no UI), then Phase B (summarize primitive). (b) **Chat UI (S102/S103)** — all items shipped to WIP commit, nothing remaining from the proposed-next list. Clean base for new asks.

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
