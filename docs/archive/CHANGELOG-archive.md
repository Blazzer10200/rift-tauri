# rift-tauri — Changelog Archive

> Retired entries from `docs/CHANGELOG.md`. Newest first. Pre-archive history also available via `git log -- docs/CHANGELOG.md`.

## v0.4.6-alpha — 2026-05-17 — Hot-fix: switch embedded Claude to `bypassPermissions`

A second Assistant session reported `mcp__rift__remote_bash` denied with: *"Permission to use mcp__rift__remote_bash has been denied because Claude Code is running in don't ask mode."* — surfaced after the v0.4.5 ship despite `mcp__rift__remote_bash` being in the `--allowed-tools` allowlist (verified). Root cause: [src-tauri/src/assistant/mod.rs:926](src-tauri/src/assistant/mod.rs#L926) passed `--permission-mode dontAsk`, which auto-DENIES anything that would otherwise prompt the user — and MCP tools (incl. `mcp__rift__remote_bash`) require per-call approval that `--allowed-tools` does NOT short-circuit in `dontAsk`. Rift has no interactive permission UI by design, so the right mode is `bypassPermissions` — auto-allows every call, and `--allowed-tools` continues to act as the actual gate over which tool names are reachable.

One-line change (`dontAsk` → `bypassPermissions`) plus a comment block explaining why. `--allowed-tools` from S91 unchanged: the full `BUILTINS` set + scoped `mcp__rift__*` (+ `mcp__rift__remote_bash` when the remote-shell toggle is on) still defines what's reachable.

This is the real root cause behind the chronic "permission denied" reports — S91 widened tool *availability* but every MCP call still hit the per-call approval gate. v0.4.6 closes that gate via mode switch.

Verify: `cargo check` clean (auto-verifier). Functional verification via CDP after binary install — re-run the previously-blocked `mcp__rift__remote_bash` from a Trey-mode session.

## v0.4.5-alpha — 2026-05-17 — Embedded Claude tool allowlist + STT alternates

**S91 priority 1 — permission denials in the Assistant tab.** Blazzer + Trey hit recurring "tool not in allowlist" / "permission denied" messages whenever the embedded Claude tried to spawn a subagent, run a background bash command, edit a notebook, etc. S88's fix added `Skill` to all three `--allowed-tools` branches in [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) but stopped short of the rest of the built-in surface. S91 widens to the full CLI built-in set: `Agent` (subagent spawning — used by `/plan`, `/quick-review`, `/check`), `AskUserQuestion`, `BashOutput` + `KillBash` + `KillShell` (the CLI auto-invokes these whenever Bash runs with `run_in_background: true`), `ExitPlanMode`, `MultiEdit`, `NotebookEdit`, `SlashCommand`. Refactored the three branch bodies to share a single `BUILTINS` const so future additions land in one place. MCP scope is unchanged — full-config keeps `mcp__*`, scoped branches keep the explicit `mcp__rift__*` entries (+ `mcp__rift__remote_bash` when the remote-shell toggle is on). Verified via CDP: a fresh Assistant tab successfully called Bash + spawned an Agent subagent with zero permission denials.

**S91 priority 2 — STT slurred-speech tolerance.** [src/lib/state/stt.svelte.ts](src/lib/state/stt.svelte.ts) requested only one recognition alternate (`r.maxAlternatives = 1`), so the engine's first guess won regardless of confidence. Bumped to 3 and added a `pickBestAlternate` helper in `onResult` that walks `results[i][0..length]` and returns the highest-confidence transcript. When WebView2's Azure backend bothers to score the alternates (sometimes it returns 0 for every one — spec-allowed), a cleaner lower-ranked variant can now win over the slurred primary. When all confidences are 0, behavior collapses back to `alt[0]` (no regression). Vocabulary hints + Azure-direct fallback deferred (stretch goals).

Verify: CDP-driven smoke test — fresh chat tab, "run pwd && ls then spawn a recon Agent to summarize" → assistant completed both calls cleanly, no denial strings in body text. svelte-check + cargo check both clean (auto-verifier).

## v0.4.4-alpha — 2026-05-17 — Revert TTS + workspace clean-out

User reversed course on text-to-speech same day v0.4.3 shipped: only STT was wanted. This release rips TTS out and folds in a wider hygiene pass.

**TTS removed end-to-end.** `src-tauri/src/tts/` deleted, `msedge-tts` crate dropped from `Cargo.toml` (Cargo.lock shed 30+ transitive crates incl. tungstenite/tokio-tungstenite, -447 lines). Frontend `src/lib/state/tts.svelte.ts` deleted; speaker toggle + per-message replay button + all 5 TTS cards in Settings + orphan voice-dropdown / slider / sub-head CSS purged. Settings section renamed `"Voice"` → `"Speech-to-text"` (id `"speech"`, icon `Mic`). `stt.svelte.ts:121` stale error string updated. `assistant.svelte.ts` lost `tts.init/feed/flush/cancel` integration points. STT is now the sole voice surface, unchanged.

**Workspace clean-out.** Dropped 6 dead npm deps (`bits-ui`, `clsx`, `tailwind-merge`, `tailwind-variants`, `tw-animate-css`, `@types/dompurify`) — leftover from the shadcn yank — pruning 18 packages from `package-lock.json` and cutting npm-audit findings 13 → 10. Deleted stale `scripts/cdp/smoke-v04.sh` (exercises retired dock primitive; `smoke-v04-1.sh` covers current shell). Cleared 24 CDP debug screenshots from `scripts/cdp/.tmp/` and the empty `.claude/worktrees/` directory. Removed pre-existing orphan `.reasoning-meta.subtle` CSS class from `MessageBubble.svelte` and trimmed 2 unused lucide-svelte icon imports (`ExternalLink` in `ConflictResolver.svelte`, `Upload` in `Bootstrap.svelte`). Pruned local branch `backup-s25` (4 unique commits from S22-S25 era) and remote orphan `claude/determined-driscoll-32834e`.

**Doc hygiene.** Project `CLAUDE.md` hot-files table refreshed to current line counts (10 backend + 5 frontend files drifted; `assistant/mod.rs` went 775 → 1167L, `Settings.svelte` 1060 → 1505L). Memory `project_rift_tauri.md` updated: current-state line rewritten as STT-only, two resolved caveats dropped (`state_referenced_locally`, `mode-watcher`). `docs/design/` claim corrected (no longer empty — carries `assistant-roadmap.md`).

Verify: `npm run check` clean (0 errors, 0 warnings, 0 files-with-problems for the first time in the v0.4.1 era). Net diff across 16 files: **-1858 / +149** = -1709 lines.

**S90 stress-test fix-ups.** Autonomous CDP-driven pass across every UI surface (ActivityBar / chat tabs / right-pane / Assistant / Settings × 7 / Sync / Files / Terminal / Velopack / status bar). Two latent UX bugs caught + fixed: (1) `right-pane.svelte.ts::init()` clamped the in-state width but didn't re-persist, so an out-of-range stored value survived across launches — now writes back the clamped width on first load. (2) `Composer.svelte` rendered the mic button unconditionally, so clicking it with STT disabled silently set `stt.lastError` and looked broken — now gated on `stt.config.enabled && stt.supported`, paired with an `onMount(() => stt.init())` so the gate reflects real backend config (without that, users with STT enabled would lose the mic until they touched Settings → Speech once). Tooling: extended `scripts/cdp/serve.cjs` `KEY_DEFS` with `Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight` so future CDP runs can drive `Ctrl+,`, `Ctrl+\``, etc. directly.

## v0.4.3-alpha — 2026-05-17 — Voice arc: text-to-speech + speech-to-text

Two-direction voice integration. Both surface through a new **Settings → Voice** section and toggle from the Assistant header / Composer respectively.

### Text-to-speech (Claude → audio)

`msedge-tts` Rust crate calls Microsoft Edge's read-aloud endpoint (Azure Neural voices, free, no API key). `src-tauri/src/tts/mod.rs` owns a single tokio task that drains a sentence queue serially and emits MP3 b64 over `tts://audio`. Frontend [src/lib/state/tts.svelte.ts](src/lib/state/tts.svelte.ts) buffers streaming text per message id, splits on `/[.!?]+["')\]]?\s+/`, dispatches each completed sentence, and plays back-to-back via HTMLAudioElement. Cancel = generation counter bumps (drops in-flight + queued) plus local queue clear.

AssistantHeader speaker toggle = single-click `enabled + auto_speak` on; click again mutes auto-speak (master stays on so per-message replay still works). MessageBubble gets a speaker icon next to copy for one-shot replay. Settings carries the voice picker (~500 Edge voices, English first), rate/pitch/volume sliders (-50..+50), and a Test button.

### Speech-to-text (audio → composer)

WebView2's built-in `SpeechRecognition` (Edge/Chromium → Azure when online) writes directly into `assistant.composerDraft`. Live interim text streams as the user speaks; final committed text replaces interim segments on each phrase commit. No Rust-side audio capture, no model download, no build deps. [src-tauri/src/stt/mod.rs](src-tauri/src/stt/mod.rs) only owns settings persistence at `~/.rift/stt-config.json`. Composer gets a mic button on the left — click to record (pulsing red), click again to stop (focus returns to textarea, cursor at end).

Settings exposes language (12 BCP-47 locales), live-partials toggle, continuous mode toggle, append-vs-replace insertion mode. Microphone permission prompts once via WebView; subsequent uses are silent.

### Why not whisper.cpp local

Pivoted away from `whisper-rs` mid-session. Build-time libclang requirement on Windows broke `cargo run` w/o LLVM installed; bindgen route would have forced every dev (Trey included) to install LLVM. Web Speech API delivers comparable quality (same Azure backbone as the TTS path) with zero install footprint, true real-time streaming, and no first-launch model download. Trade-off: requires internet (so does Anthropic, so does the TTS).

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

## v0.4.0-alpha — 2026-05-17 — Chat tabs + split dock (experimental v0.4)

Two layered features on top of the v0.3 single-canvas shell: browser-style chat tabs at the top of Rift, and a right-side dock that can grow up to half-screen and split horizontally into left + right slots, each carrying its own panel stack. Both ride the existing `uiPrefs.useV03Shell` toggle (Settings → Appearance → Experimental). v0.2 path remains pixel-identical — the toggle is the rollback.

### Chat tabs (Phase 1)

A dedicated 34px row sits below the Titlebar, only when v0.3 is on. Each tab is one Claude conversation; opening many lets you context-switch without losing state. Close keeps the convo in History. `MessageSquare` icon swaps to a pulsing dot while that tab is mid-stream. Tab titles auto-fill from the first user message (40-char cap) — unsaved new tabs show "New chat" until the first send saves them.

`AssistantStore` grows `openTabs: string[]` plus `openTab`/`closeTab`/`newTab`/`reorderTabs`/`cycleTab`/`closeAllTabs`/`closeOthers`/`closeTabsToRight`. Persistence is `localStorage["rift.ui.tabs.v1"] = { openTabs, activeTabId }`. On init, stored tab ids filter against `assistant_list_conversations` — orphan ids drop silently. `send()` now keys "first turn vs resume" off `convoCreatedAt`, not `currentConvoId`, so newTab can mint the id up-front without breaking the CLI's `--session-id` path. Click-to-switch handles unsaved-new-tab targets in-memory instead of disk-loading a record that doesn't exist yet.

Keybinds: `Ctrl+T` new tab · `Ctrl+W` close active · `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle · `Alt+1..9` jump. Drag a tab to reorder (HTML5 DnD + tail-zone for append). Empty state (no tabs) replaces chat + composer with a centered "+ New chat" card hinting at History.

### Split dock (Phase 2)

`PanelState` grows a `slot: "left" | "right"` field. localStorage migration on load — any panel without `slot` defaults to "left", so existing v0.3 users see no visible change. The outer dock width recomputes its max per resize: `Math.min(900, innerWidth - 480)`, reserving a 480px chat minimum. Double-click the outer resize handle to snap to ~50% viewport.

Inside the dock, a CSS grid `[left slot] [4px split-handle] [right slot]` collapses to a single column when the right slot is empty. Drag a panel header across the slot boundary to reassign — the right slot appears as a "Drop here → New right slot" target during left-source drag; an occupied slot shows a soft outline + dragover tint. New `dockSplitPct = $state(50)` drives `--dock-split-pct` on `:root`; the internal 10px split-handle hit area (2px visual via `::after`, RAF-throttled, persist-on-release) follows the same drag-vs-release pattern as the outer width handle. Min/max 20–80%, double-click snaps to 50.

Accordion sweep (`applyOpenState(closeOthers=true)`) restricts to the dragged panel's slot — opening a left-slot panel no longer collapses a right-slot panel. `Ctrl+1..8` still toggles the named panel regardless of which slot it lives in.

### Polish (Phase 3)

Settings → Appearance picks up a Layout sub-card under the v0.3 accordion toggle: "Reset dock split" button (→ 50%), "Close all chat tabs" button, and a kbd cheat sheet. New `scripts/cdp/smoke-v04.sh` runs 23 checks end-to-end (Ctrl+W loop reset → 3-tab open → keyboard cycle → close-middle → cross-slot drag → per-slot accordion → split resize → maximize+Esc → empty-right collapse). 23/23 green.

CDP wrapper (`scripts/cdp/serve.cjs`) — Key dispatch grows on-demand resolution for digits 0-9 + letters a-z; `KEY_DEFS` stays the source of truth for special keys.

### Don't-touch carryovers from v0.3

Registry-based PanelShell mount, `applyOpenState` clearing `maximized` when its panel closes, Terminal lazy-mount fallback, slide-over Settings, S76 dock-resize snappiness (RAF + `setDockWidthLive`/`persistDockWidth` split + no grid transition under v0.3). All v0.2 codepaths preserved verbatim.

### Verify

`svelte-check` clean. `cargo check` clean (no Rust changes this arc). CDP smoke `bash scripts/cdp/smoke-v04.sh` 23/23 PASS. v0.3 toggle OFF renders pixel-identical to v0.2.56-alpha.

## v0.2.57-alpha — 2026-05-17 — Assistant maturity + experimental v0.3 shell

Seven sessions (S69–S75) of layered work. The Assistant gains harness pull-through, native session resume, workspace context, remote-bash, multi-user awareness, streaming polish, and a per-message cost+model badge. An experimental v0.3 single-canvas shell ships flag-gated behind Settings → Appearance → "Experimental v0.3 shell layout" (default OFF — current v0.2 shell unchanged). Both `cargo check` + `npm run check` clean.

### Assistant maturity (S69–S74)

S69 fixed the blank-response bug — Windows `claude.cmd` shim was mangling `--output-format stream-json` arg quoting. Spawn invokes the underlying JS bin directly via `node` to bypass. Same session surfaced extended thinking via `MAX_THINKING_TOKENS=10000` env (only the env var works; `--settings '{"thinking":...}'` + `--permission-mode plan` do not).

S70 shipped CDP autonomous-verify infra. `scripts/cdp/serve.cjs` (port 9223) wraps WebView2's CDP endpoint with a persistent ws. `bash scripts/cdp/c.sh {health|state|eval|type|click|wait|shot|key|shutdown}` drives + observes the running UI without screenshots. ~40-60ms per call. Used for Phase A/B/C verification this arc.

S71 (Phase 1) harness pull-through. `AssistantConfig.use_full_config` default ON drops `--strict-mcp-config` + `--disable-slash-commands` so user MCPs and slash commands layer alongside Rift's. API-key mode forces off via `--bare`. Multi-user `<cwd-hash>` collision resolved (per-user `~/.claude/` isolates).

S72 (Phase 2) native session-id + resume. `--session-id` on turn 1, `--resume` on follow-ups, deleted the hand-rolled history replay. `AssistantConfig.max_budget_usd` + Settings "Per-turn cost cap" — only `--max-budget-usd` shipped, `--max-turns` is not a real CLI flag.

S73 (Phase 3) Rift-native sprint. Per-turn `WorkspaceContext` addendum: live foreign-locks + AutoSync queue + recent DiagBus events spliced onto the system prompt at spawn. `mcp__rift__remote_bash(command, timeout_secs?)` tool exec's over the auto-sync engine's live russh session via a loopback NDJSON bridge (`assistant/remote_bridge.rs`); ~5ms loopback RTT vs ~500ms cold SSH dial. Env-gated by `RIFT_REMOTE_SHELL_ENABLED=1`. Workspace-scoped `<remote_root>/.rift-shell.rift-lock` advisory lock; foreign holders surface as a "trey (4m)" pill in `AssistantHeader`.

S74 (Phase 4) UX polish, seven items: (1) `/tools` slash notice rewritten for full Claude Code parity + conditional remote_bash line; (2) diff view in Edit op-cards — TasksDock swaps raw JSON dump for unified red/green list; (3) per-message cost+model badge — "Sonnet 4.6 · $0.0772" pill in MessageBubble's role-row; (4) @-file mention picker — new `assistant_list_workspace_files` Tauri cmd (SKIP_DIRS mirror, 4000-cap), three-tier fuzzy ranking; (5) code-block copy buttons via `annotateCodeBlocks()`; (6) conversation search in HistoryDrawer; (7) context-aware empty-state — detects FiveM/RedM via `fxmanifest.lua`. Plus streaming pacer (~120 ch/s drip from rAF queue, auto-drain in 400ms) + blinking-caret + soft fade-mask chrome on streaming bubbles, and a dedicated `WEBVIEW2_USER_DATA_FOLDER` for dev so it doesn't collide with the installed Rift's lock.

### Experimental v0.3 shell — flag-gated (S75)

Twenty-three commits, all behind `uiPrefs.useV03Shell`. Flag-off path renders pixel-identical to v0.2 — zero regression risk.

Flag-on: chat is the permanent center, every other tool lives in a right-side dock. Eight panels (Tasks, Sync, Files, History, Agents stub, Terminal, Attachments stub, Activity Feed) wrap the existing v0.2 surfaces via a registry-based `PanelShell` primitive. First-launch preset picker offers Minimal (Tasks + History), Standard (5 panels), or Power (all 8). Settings becomes a slide-over modal (Esc + X dismiss). Panel headers carry optional reactive count pips (Sync conflicts in danger-tone, Activity events + Tasks + History in info-tone). Drag a panel header to reorder. Drag the dock's left edge to resize 280–560px. Accordion mode on by default (one panel open at a time; shift-click or Ctrl+Shift+N bypasses). Maximize-to-center: click ⛶ on any panel header and that panel takes over `<main class="pane">` while chat hides — Files + Sync use this for their drift-table / file-browser views via compact summary cards in the dock + a "View … in center" button. Terminal auto-maximizes on first open (xterm at 320px dock width is unusable). Esc restores chat. `applyOpenState` clears the maximized cursor if its panel is closed via rail.

Architecture note: PanelShell instantiates `def.component` from the PANELS registry directly — not slot-based. Wrappers ARE the bodies. PanelDef carries optional `getCount` / `getTone`. All v0.2 codepaths preserved verbatim under the `useV03Shell` branch.

Toggle: Settings → Appearance → "Experimental v0.3 shell layout." Restart required (some mount-time reads). Both `npm run check` + `cargo check` green at every commit.

### Verify

`svelte-check` 0 errors. `cargo check` clean. CDP smoke-pass per phase. v0.3 flag-off renders pixel-identical to v0.2.56-alpha.

## v0.2.56-alpha — 2026-05-15 — AI Assistant + full UI consistency rework

The big one. Nine sessions (S60-68) of work covering Rift's biggest identity change since v0.2.0: an in-app **AI Assistant** that lets you talk to Claude against an open project folder, plus a top-to-bottom UI consistency pass that re-shaped every page around a canonical skeleton.

Assistant tab (Ctrl+3, BETA chip) auth-piggybacks on the user's `claude` CLI session; API-key fallback for pay-per-token. Rift ships a stdio MCP server inside its own binary; CLI spawns w/ `--mcp-config` pointing back at itself + `--allowed-tools mcp__rift__*`. Three read-only tools (`read_file` ≤500KB, `list_dir` ≤500 entries, `grep` walkdir+regex ≤200 matches), all paths canonicalized + checked against `RIFT_MCP_ROOTS`. Plus `TodoWrite`. Workspace decoupled from FiveM Sync — VSCode-style "Open Folder," works on any stack. Chat surface: AssistantHeader, Composer (autosize, send→stop morph, slash menu w/ 9 cmds, ↑/↓ recall), MessageBubble (avatar gutter, copy btn), state-aware EmptyState, TasksDock (auto-opens on first TodoWrite/MCP tool call), HistoryDrawer (rename + two-step delete). Markdown via marked+marked-alert+DOMPurify, full GFM. Real stop button via taskkill/kill -TERM tracking child PID. Auto-scroll respects user intent (stickToBottom flag).

UI consistency: four new shell primitives (PageHeader, PageToolbar, PageFooter, EmptyState). Five pages converted to canonical skeleton (Conflicts, Activity, Files, Sync, Assistant). Titlebar declutter (connection pill folded into server-picker dot). StatusBar simplified. TabRail rework (groups + hairline dividers + active-tone glow + pin button + container query collapsing). Files tab drag-reorder via pointer events + animate:flip. Sync shrink-banner collapsible. About page Paths + Diagnostics sections w/ privacy scrub on copy.

Verify: svelte-check 0 errors across 4020 files, cargo check clean, privacy audit confirmed standalone.

## v0.2.55-alpha — 2026-05-14 — Sync page overhaul: one-button Sync, auto-rescan, keep-alive tabs

A focused UX pass on the Sync page — the most-used screen after Browser. Two longstanding annoyances (Pull-then-Push needing two clicks; pushes hidden after Pull all completes) are gone, drift now auto-rescans on first connect + on a user-settable interval, and tab switches lost their flash.

### One-button Sync (pull then push)

Replaced separate `Pull all` + `Push all` buttons in the hero with a single primary `Sync` button. Click sequences `sync_pull_pending` → 2.5 s drain → `sync_push_pending` → 1.2 s → rescan, in that order. Pull-before-push is canonical: it rebases local against remote so push never dispatches against a stale baseline. Button label live-updates `Sync (N↓ M↑)` → `Pulling… (N)` → `Pushing… (M)` so the phase is always visible. Pull-only / Push-only granular controls demoted into the new `⋯` kebab menu under an "Advanced" section. Conflicts stay in the conflict bucket (not auto-resolved); Mirror remote-deletes stay gated behind the typed-confirm modal.

### Rescan-after-dispatch fix

Calling `Pull all` with mixed pull + push drift previously dispatched the pull, then `refresh()` returned an empty drift snapshot (backend clears cached pending entries on dispatch), so the page rendered "Everything in sync" — hiding pushes the user could clearly see two seconds earlier. Now `pullAll` / `pushAll` / `applySelected` / `confirmMirrorApply` all chain to `rescan()` instead of `refresh()`, so the next snapshot is a fresh `sync_reconcile` result with every remaining drift entry intact.

### Auto-scan on first connect

When the watcher transitions to `watching` / `idle` / `syncing` for the first time per server-key per session, the frontend auto-fires a drift `sync_reconcile`. Drops the "open Sync page → click Rescan → wait" first-launch ceremony — drift is already populated by the time the user navigates there. Latch clears on disconnect so reconnect re-fires.

### Auto-rescan (opt-in periodic)

Local watcher only sees local edits; remote drift from teammates pushing is invisible until manual rescan. New auto-rescan toggle in the kebab cycles `off → 30 s → 1 m → 2 m → 5 m → 10 m → off`. Persists to localStorage. Timer lives in `AppShell` (survives tab switches), gates on `enabled + watcher-ready + connected`, skips ticks when busy / loading / in preview. Interval changes tear down + recreate the timer cleanly via `$effect` cleanup.

### Tab-switch flash fix

`AppShell` was wrapping every page in `{#key active}` with `in:fly` (90 ms delay + 180 ms duration) + `out:fade` (90 ms). On every tab switch the active page fully unmounted + remounted, child components re-ran `onMount` (data fetches, listeners), and their own inner `in:fly` / `in:fade` transitions re-fired → cascade pop-in glitch. Now each page mounts once on first visit and stays mounted; `hidden` attribute toggles visibility instantly. Cold-launch unchanged (only Browse mounts initially). Inner re-key for `settingsSection` + `selectedConflict` preserved. Removed unused `fly` / `fade` / `quintOut` imports.

### UI reskin (Phase A)

* Hero compaction: `[⋯] [↻] [Apply Mirror (cond)] [Sync]` — three visible buttons down from seven. Kebab houses Mirror toggle, Auto-rescan, Sweep stale locks, Pull-only, Push-only, Design preview.
* Two-line entry rows: path + size on line 1, reason + relative mtime on line 2. `formatSize` (B/KB/MB/GB) + `formatMtimeRel` (s/m/h/d ago) helpers.
* Selection footer: tone-tinted breakdown (`2 push · 2 pull · 1 delete`) replaces the generic hint when items are selected.
* Empty-state subtitle: `Last scan Xs ago · N folders watched` + ghost `Rescan now` button.
* Design-preview fixture (Eye icon in kebab): injects 9-entry fixture across 3 resources covering every bucket + aborted-shrunk banner, dispatch buttons gated. Lets us screenshot every UI state without needing real drift.

### Verify

`svelte-check` 0 errors / 0 warnings across 3999 files.

## v0.2.54-alpha — 2026-05-13 — Fresh-install bootstrap + titlebar dropdown hotfix

Two onboarding bugs surfaced while bringing a second dev (Trey) on board for the first time. Both block the empty-local → populated-remote first sync path.

### Fresh-install bootstrap (Bug 1)

`auto_sync::try_watch` was silently returning `Ok(false)` when a folder's local subdir didn't exist on disk, leaving the engine with `watches = 0` for a brand-new install. The drift scanner only iterates registered folders, so Rescan returned zero entries → Sync page rendered "Everything in sync" → no way for the user to pull the remote tree down without finding the hidden `Ctrl+K → Bootstrap from remote…` dialog. Now: when the profile's `local_root` exists but a per-folder subdir doesn't, `try_watch` `mkdir_all`s the subdir, logs `"auto-created local folder for first-time bootstrap"` to diagnostics, and attaches the watcher normally. If the profile `local_root` itself is missing (genuine typo / config error) we still bail with `Ok(false)` + a clearer log, never silently mkdir somewhere unexpected. After this, a fresh install with empty local just works: connect → 8 bracket dirs auto-create → drift scan finds remote-only files → Sync page shows ToPull entries → Pull all streams the tree down.

### Titlebar server dropdown clipped (Bug 2)

The titlebar's server-picker dropdown menu opens below the 44 px titlebar row. Its parent `.left` flex container had `overflow: hidden` to constrain text overflow into the drag region, which also clipped the menu vertically — z-index can't escape an overflow-clip ancestor. Moved the overflow constraint from the parent down to the child spans (`.svr-name` / `.svr-host` now use `white-space: nowrap; text-overflow: ellipsis; overflow: hidden`) so long server/host text still truncates cleanly, but the dropdown can render outside the titlebar height. Defensive `z-index: 100 → 1000` on `.svr-menu` too.

### Verify

`cargo check` clean · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

## v0.2.53-alpha — 2026-05-13 — Mirror mode + auto-reconnect

The two queued safety nets land in one release. Mirror mode gives Rescan a recovery path when watcher events get missed (rare, but happens — e.g. notify-rs Windows issue #403 silently dropping events on a watched-dir delete). Auto-reconnect closes the loop on v0.2.50's `ConnectionWedged` detection: instead of just emitting a diag event and waiting for the user to click Sweep + manually reconnect, the frontend now self-heals after 3+ wedges in a 60 s window.

### Mirror mode (Bug 1)

New `DriftBucket::ToDeleteRemote` variant. When the drift scanner runs with `mirror = true` and sees `l.is_none() && r.is_some() && snap.is_some()`, it now buckets as ToDeleteRemote ("local deleted — removing remote") instead of ToPull ("remote-only — pull"). Normal mode keeps treating this case as ToPull (the safer non-destructive direction). The flag is session-scoped on the engine (`mirror_mode: AtomicBool`) and exposed via two new Tauri commands, `sync_set_mirror_mode(enabled)` and `sync_get_mirror_mode()`. Dispatch lives in `auto_sync::apply_selected`, which routes ToDeleteRemote entries to `sftp.delete(remote_path)` — the SftpClient::delete router already handles dirs through `delete_recursive_via` and files through `remove_file`, so folder deletes propagate cleanly. The mass-delete circuit breaker is intentionally skipped for ToDeleteRemote because the user reached dispatch through the typed-confirm modal — that gate is the consent.

Frontend: a "Mirror" toggle next to Rescan/Sweep on the Sync page (red accent when enabled). Toggling triggers an immediate Rescan so the bucket counts redraw. When entries are in the ToDeleteRemote bucket, a red "Apply Mirror (N)" button appears. Clicking opens a hard-gate modal: count of files to delete, warning copy about irreversibility and multi-user baseline coordination, and a typed-confirm input requiring the literal text "MIRROR" before the Confirm button enables. Backdrop click and Escape both cancel. Backend session-scoped means the toggle resets to off on engine restart — paranoia against accidental destructive ops on a fresh launch.

### Auto-reconnect (v0.2.50 follow-through)

`connection.svelte.ts` now listens to `diag://event` for `stage === "connection_wedged"` emits (these come from `sftp/transfer.rs::with_t` when an SFTP op blows the timeout). A rolling 60 s window holds the timestamps; once 3+ wedges land inside the window, the frontend calls `stop_autosync`, sleeps 1 s for clean teardown, then calls `startAutosyncForSelected()` to re-open the session with the same server + folder spec. A `reconnecting` guard prevents overlapping reconnects. Single wedges still don't reconnect — those usually self-resolve on the next op and aren't worth the session churn. Lives entirely client-side so we don't have to refactor the engine's owned `SftpSession` (which isn't behind a RwLock).

### Verify

`cargo check` clean 5.00 s · `cargo test --lib` 46 passed · `svelte-check` 0 errors / 0 warnings across 3996 files.

### Deferred to v0.2.54

- Integration test suite phase 1 — 10 mock-SFTP scenarios (clean reconcile, local-add, local-delete Normal + Mirror, remote-add, conflict, SuspiciousEmptyAborted, dry-run Mirror, Mirror-disabled-when-shrunk). Requires either an SftpClient trait abstraction for mocking or a testcontainers-based real SFTP server in CI — its own evening.
- Dry-run Mirror preview UI (current modal goes straight to confirm; a "preview rows" pre-confirm step would let the user spot-check before typing MIRROR).
