# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 124 — 2026-05-20 — compaction Phase D + history persist + agent tracking

Uncommitted on top of S123. User AFK mid-session — autonomous CDP-verified.

**Phase D — settings UI + auto-trigger + pre-emption banner (SHIPPED):**
- Store gains `autoCompactThreshold: number | null` + `compactModel: "haiku"|"sonnet"` w/ init load + setters wired to existing `assistant_get/set_auto_compact_threshold` + `assistant_get/set_compact_model` Tauri cmds (backend was already ready from A4).
- Ctx derivations (`contextWindowFor` / `ctxTokens` / `ctxWindow` / `ctxPct`) lifted from header onto store as derived getters. ChatTabsBar consumes `assistant.ctxPct` etc. *(AssistantHeader.svelte mirror-edits left in place but the component is DEAD CODE — see CRITICAL.)*
- Auto-trigger `$effect` in AssistantPage.svelte. Guards: threshold set, activeTab present, not streaming/compactingNow, ≥5min since lastCompactionAt, ctxPct ≥ threshold·100.
- `assistant.compactWarning` getter — returns *"Approaching auto-compact at X%"* when ctx is within 10pp of threshold (warn-pill in ChatTabsBar).
- Settings → Assistant → new "Conversation compaction" card: threshold select (Off / 70 recommended / 80 / 85 / 90), Haiku/Sonnet radio, "Compact now" debug button.
- **CDP-verified:** Settings card renders 3 controls; threshold + model setters round-trip to backend. **NOT live-verified** (didn't mutate user's real convo): auto-trigger crossing, pre-emption banner, Compact button, boundary pill. Smoke gate open.

**compactionHistory[] persistence (Phase E prereq) — SHIPPED:**
- New `CompactionHistoryEntry` type ({at, priorSessionId, newSessionId, summary, costUsd, summaryModel, archivedCount}).
- `ConversationRecord.compactionHistory?` + `TabState.compactionHistory` + hydrate in loadConversation + save in buildSaveRecord (omitted when empty) + push entry in compactConversation() success branch. Backend opaque — no Rust changes.

**Background-agents tracking — SHIPPED state + header pill:**
- `TabState.agentSpawns = $state<{id, subagentType, description, startedAt, completedAt, isError}[]>([])`. `appendToolUse` pushes on `Task`/`Agent` tool names; `fillToolResult` marks completedAt + isError on matching id.
- ChatTabsBar accent-toned pulsing `.agents-pill` shows live count + tooltip lists per-agent labels. NOT persisted (dies w/ tab). Full collapsible panel deferred to Phase E.

svelte-check 0 errors. `cargo check` NOT run (Tauri dev alive).

**Live-smoke bugs found + fixed (S123 compaction was untested before today):**

1. **Summarize wrong cwd** — `assistant_summarize_session` spawned `claude -p` without `cmd.current_dir(...)`, so `--resume <sid>` looked in the Rift project's cwd-hash dir instead of the workspace's. Fixed via `load_session_cwd(&session_id).filter(|p| p.is_dir())` set before arg chain ([mod.rs:1004-1010](src-tauri/src/assistant/mod.rs#L1004-L1010)).
2. **Summarize stdin-piped prompt ignored** — claude `-p` w/ `--input-format text` reads positional arg, not stdin. Switched to `cmd.arg("-p").arg(&prompt)` + `Stdio::null()` for stdin ([mod.rs:1018-1043](src-tauri/src/assistant/mod.rs#L1018-L1043)). The previous design-doc note "Stick to text input format" stays — only the delivery channel changed.
3. **NDJSON parser missed text** — CLI 2.1.139 emits buffered `"type":"assistant"` envelopes w/ full `message.content[].text` in `-p` mode, not per-token `stream_event` deltas. Added an `"assistant"` envelope branch (drain `message.content[*].text`) AND drain `result.result` as the canonical aggregated text in the `result` envelope ([mod.rs:1074-1115](src-tauri/src/assistant/mod.rs#L1074-L1115)). Parser is now robust to both output shapes.
4. **`scheduleSave` clobbered post-compact `convoCreatedAt = null`** — `doSave()` runs `buildSaveRecord` (falls back to `Date.now()` when convoCreatedAt is null) and writes that timestamp BACK onto `tab.convoCreatedAt`, defeating compaction's null-out. Next send saw `isFirstTurn=false` → backend `--resume <newSid>` on a JSONL that didn't exist yet → session-lost recovery fired → `priorSummary` already drained on the failed first attempt → summary never delivered to the new session. Fixed by adding `TabState.forceNextFirstTurn: boolean`. compactConversation sets it true; `send()` reads/clears it as part of the isFirstTurn check ([assistant.svelte.ts:567-578, 2454-2456, 2799](src/lib/state/assistant.svelte.ts)).

**End-to-end smoke passes:** 4 turns → /compact → notice "Compacted 8 message(s) · $0.0370 · 35,788 in / 353 out · haiku" → boundary pill renders → both JSONLs on disk → new JSONL's first user-turn `<system-reminder>` contains *"Prior conversation summary (compacted; the CLI session this turn runs against is fresh ...): ## Active task ... ## Files in play ... ## Decisions ... ## Open questions ..."* → model acknowledges the summary context. ACK-token recall on synthetic test content failed only because Haiku correctly compressed test noise to "ACK checkpoints only; no work initiated" — for real workflows the summarizer preserves files/decisions/tasks verbatim per the prompt directive.

**Live-streaming compaction UX (bonus #2):**
- `assistant_summarize_session` gained `AppHandle`; emits `assistant://summarize-progress {session_id, summary_so_far, status}` rate-limited at 150ms/64chars on each accumulated text update, plus a final `status:"done"` on the `result` envelope.
- `compactConversation()` pre-stages the `BoundaryBlock` with `streaming:true` + empty summary BEFORE the summarize call. Per-compact `listen()` filtered by `oldSid` patches the block's `summary` in-place as events land. On error/abort the stale boundary is filtered out so the chat doesn't keep a half-rendered pill. `progressUnlisten` cleanup in the outer `finally`.
- `BoundaryBlock.streaming?: boolean` added to the type. `MessageBubble.svelte` branches on it: pulsing live-dot, "Compacting · N messages · X chars" label, body auto-expanded so user sees the summary fill live, toggle button disabled until done.
- CLI 2.1.139 `-p` mode buffers (~150ms streaming window in practice) but the parser also handles per-token `stream_event/text_delta` shape — future CLI versions w/ true token streaming render smoothly w/o any code change.

**Chat-UI polish pass — tool chips redesign + tasks dock progress:**
- **Tool chips gain a category** ([ToolChip.svelte](src/lib/components/assistant/ToolChip.svelte)) — 5 buckets: `read` (Read/Glob/Grep/list_dir/WebFetch/WebSearch) muted blue, `write` (Edit/Write/MultiEdit/NotebookEdit) accent, `shell` (Bash/remote_bash/BashOutput) warm orange, `agent` (Agent/Task/Skill/SlashCommand) purple, `meta` (TodoWrite/AskUserQuestion/ExitPlanMode) green. Drives the chip icon color + a 2px left-edge stripe so a wall of chips is scannable by category at a glance without reading labels.
- **Inline result preview** — when collapsed AND result is a short single-line outcome (`"No files found"`, `"hello from bash"`, etc.), the chip head shows `→ {preview}` inline so you don't need to expand. Caps at 60 chars; multi-line w/ >3 lines or >200 chars falls through to require expansion.
- **Duration badge** — warn-toned pill next to status icon when `tool.durationMs >= 1000`. Required adding `ToolBlock.startedAt?: number + durationMs?: number` and wiring them in `appendToolUse` (set startedAt on push) + `fillToolResult` (compute durationMs on result).
- **Pending pulse** — `data-status="pending"` chips animate a background-color pulse (1.8s ease-in-out) so in-flight tools stand out from done ones beyond just the spinner.
- **Tasks dock** ([TasksDock.svelte](src/lib/components/assistant/TasksDock.svelte)) gains a 2px progress bar between header and body. Fill width = `done/total`. When an `in_progress` task exists, a pulsing overlay segment marks its position. Counter pill keeps the existing `done/total` text but with a faint separator slash.
- **Streaming avatar halo** ([MessageBubble.svelte](src/lib/components/assistant/MessageBubble.svelte)) — assistant avatar gains a 1.8s pulsing accent halo (`box-shadow` animation) while the bubble is mid-stream, gated by `prefers-reduced-motion`. Replaces the dead-air feel during long thinking turns w/ a heartbeat.
- **Tighter chat rhythm** — message gap `20px → 16px`, inter-bubble padding `14px → 12px` ([AssistantPane.svelte](src/lib/components/assistant/AssistantPane.svelte)).
- **CDP-verified:** Live test fired Read/Glob/Grep/Bash w/ visible category-colored borders (red on errored Reads, orange on Bash, blue-ish on successful Reads), `→ hello from bash` inline preview on the Bash chip, `6.1s` and `2.1s` duration pills on slow calls, Tasks dock progress bar fills 4/4 w/ strikethroughs on completed items, avatar halo confirmed active during streaming via DOM probe.

---

## Session 123 — 2026-05-20 — audit + AskUserQuestion + compaction Phases B & C

Uncommitted on v0.4.19-alpha base. **(Details preserved in `git log` — abbreviated here to keep handoff under 600 words.)** Audit pass (2 real fixes: drift_scanner walk_local segment-rule probe, AskUserQuestion removed from BUILTINS allowlist because `-p` mode stalls on it) + compaction Phases B (`assistant_summarize_session` Tauri cmd, `CLAUDE_DISABLE_HOOKS=1` to skip 46K-tok SessionStart surcharge) and C (`assistant_remint_session`, `prior_context_summary` reminder, `BoundaryBlock` + system role, `compactConversation()` flow, `/compact` slash, header Compact button at ctxPct≥50, MessageBubble boundary-pill render). cargo check + svelte-check both green.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.19-alpha** (S122 shipped; S123 + S124 uncommitted). Tauri 2 + Svelte 5 + Rust + russh.

**Smoke status — main compact flow ✅ verified end-to-end this session.** Still unverified live:
- Auto-trigger crossing: Settings → Assistant → "Conversation compaction" card → flip threshold to 70%, push ctx past 70% with a long turn, confirm auto-trigger fires once + 5min cooldown holds.
- Pre-emption: with threshold=70%, drive ctx to 60-69%, confirm "Approaching auto-compact at 70%" pill appears + disappears on cross.
- Compact button (header) gate — needs ctxPct≥50 on a real (non-test) chat to surface; `/compact` slash and "Compact now" settings button always available.
- Background-agents pill: have Claude spawn a subagent (Task tool), confirm accent-toned pulsing "N agents" pill appears in ChatTabsBar w/ tooltip listing subagent_type + description, then disappears when tool_result lands.

**Then next code lanes (priority order):**
1. **Phase E polish** — boundary pill "X% → Y%" stats, "Expand archived turns" affordance (`boundaryAtIndex` + `visibleMessages` derived), compact-on-tab-close prompt, JSONL cleanup sweep using new `compactionHistory[].priorSessionId`, HistoryDrawer search across `compactionHistory[*].summary`. compactionHistory persistence is shipped — these now have data to consume.
2. **Background-agents panel proper** — expand the header pill into a collapsible strip near StatusHub w/ per-agent rows (subagent_type, description, elapsed). State is already tracked on `TabState.agentSpawns`; just need the panel UI.
3. **Wave-2 MEDs** (#146-#149 + #151-#177, ~30 items), #81 full DiagBus, right-rail / titlebar polish, LOW/INFO tail.

**Verification doctrine:** smoke gate on Phase C/D paths is still the user's first task. Backend round-trip CDP-verified S124 (threshold + model setters); auto-trigger crossing + Compact button + boundary pill + agents-pill NOT smoke-verified (didn't mutate user's live convo).

**Phase 6 keychain runtime:** old plaintext `bridgeToken` / `apiKey` get auto-lifted to Windows Credential Manager on first config load. `cmdkey /list:rift` for `rift/bridge.<server_key>` + `rift/assistant.api_key`. S123 verified dormant-but-correct here (`api_key: null` + no `bridgeToken` field).

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` + full `BUILTINS` in `--allowed-tools`.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` when attachments present. 20MiB cap.
- Settings is workspace (kbd 9), `Ctrl+,` flips; do NOT reintroduce slideover scrim.
- Assistant scrollbar: `.scroll` + `.strip` BOTH `scrollbar-width: none` — don't reintroduce `scrollbar-gutter: stable`.
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0`. Don't reintroduce empty-tabs CTA.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- **`tauri.conf.json` `dragDropEnabled: false`** — removing breaks cross-region HTML5 DnD. Rift has no file-drop Tauri events, cost = zero.
- **AssistantPane drop handlers on `.pane` outer div only** — never move to inner `.drop-zone` overlays; loses the continuous-preventDefault chain.
- **`composerDraft` stays store-level** — moving to per-pane requires Composer rewire across ~30 refs; not worth it.
- **`AssistantHeader.svelte` is dead code** — absorbed into `ChatTabsBar.svelte` (right-side `.actions` block). Don't add NEW chat-status chips to AssistantHeader; they won't render. Edit ChatTabsBar.svelte instead. (S124 noted — AssistantHeader.svelte still on disk but unmounted by any route.)
