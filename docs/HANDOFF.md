# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 96 — 2026-05-18 — v0.4.10-alpha: workspace shell (shipped)

Single workspace-swap shell replacing v0.2 / v0.4.1 dual-shell. Activity bar (40px) navigates; 8 reachable workspaces (Chat · Sync · Files · Conflicts · Diagnostics · Terminal · Activity · History) + 2 disabled stubs (Agents, Attachments) + settings gear bottom of rail. ChatTabsBar gated on `workspace.activeId === "chat"`. 3 commits (2c48bc7 → 87e9345 → 7b96146); smoke 100/0 fresh, 97/3 non-fresh (sleep-2 race); `npm run check` 0/0; ~956 LOC deleted / ~150 added. Design: [docs/design/workspace-shell.md](design/workspace-shell.md). Details: `git log -p 2c48bc7..7b96146`.

**Pending from S96:** (a) **Velopack delta chain investigation — TOP PRIORITY** (carried from S95, v0.4.10 ship resets chain). (b) Trey-config sync. (c) Dead CSS warnings: TerminalPanel (~15 collapsed-strip/divider rules), AssistantHeader (hdr-btn.active, convo-chip), TasksDock (dock-head/title/closebtn). (d) Drop unused width-resize handlers from TerminalPanel. (e) `smoke-v04-10.sh` sections C+F: `sleep 2 → 3` one-liner.

## Session 97 — 2026-05-18 — cmdk palette REMOVED (user decision after 2nd failed attempt)

Bug from prior S97 attempt persisted. Tried external `class PaletteStore { open = $state(false) }` module store (anti-pattern from prior bare-local `$state`), then a propless variant where CommandPalette imported the store and read `palette.open` directly inside `{#if}` (mirrors working SyncModal pattern). Both attempts: store value flipped correctly under `palette.open = true`, but the CommandPalette-side `{#if palette.open}` block and a debug `$effect(() => console.log(palette.open))` never re-ran. AddServer / SyncModal use the same patterns and work — only this site was broken. Reactivity-source plumbing isn't the bug; root cause never identified.

**User halted: removed the feature entirely.** No command palette in titlebar going forward. Surface decluttered.

**Files deleted:**
- `src/lib/state/palette.svelte.ts` (created this session)
- `src/lib/components/dialogs/CommandPalette.svelte`

**Files modified:**
- [Titlebar.svelte](../src/lib/components/shell/Titlebar.svelte) — `.cmdk` button, `onOpenPalette` prop, `Search` lucide import, and `.cmdk` CSS rules all removed
- [AppShell.svelte](../src/lib/components/AppShell.svelte) — CommandPalette mount, `palette` import, Ctrl+K keybind, and the entire `sharedCommands`/`workspaceCommands`/`commands` derived registry removed

`npx svelte-check`: **0 errors, 0 warnings, 4026 files**. Not committed — user holding for the next session.

**Lessons (workflow):** I CDP-probed compiled JS for 30+ min on the symptom side instead of cutting bait at the first failed direct fix. Same trap as S97's original attempt. Rule for next time: **2 failed direct attempts on a reactivity-shape bug → ship without the feature OR ask user before further archaeology**, don't grind on Svelte internals.

**Workspace switching, settings, server picker, bridge indicator all unaffected.** Only the search-button → palette path is gone.

## Session 98 — 2026-05-18 — assistant context inconsistency fix

Assistant was reading stale/missing context. Two root causes in [assistant/mod.rs](../src-tauri/src/assistant/mod.rs):

1. **cwd pinned per session.** `--resume <uuid>` only searches the current cwd's `~/.claude/projects/<cwd-hash>/` ([anthropics/claude-code#35226](https://github.com/anthropics/claude-code/issues/35226) — no fallback). Workspace switches mid-conversation → resume aimed at wrong dir → session-lost → frontend popped messages, silently restarted. Now a sidecar `~/.rift/assistant/sessions/<uuid>.cwd` captures cwd on first turn and overrides root resolution on every subsequent turn. Legacy convos auto-migrate on their next resume. Deleted on `assistant_delete_conversation`.

2. **Per-turn workspace context moved from `--append-system-prompt` → user-turn `<system-reminder>` block.** Live AutoSync state (foreign locks, sync queue, recent diag events) was being spliced into the system prompt every turn → busted the cache-prefix every turn (cache layout: system → tools → CLAUDE.md → conversation tail). Static addendum (tool list, ACT FIRST, dyslexia, remote_shell description) stays in `--append-system-prompt`; the per-turn snapshot now rides the user message. Newline-separated for readability since the stdin path has no argv constraint.

Also added `--exclude-dynamic-system-prompt-sections` so the CLI's own cwd/env/git auto-injection also leaves the cached prefix.

Backend-only; wire-compatible w/ existing `assistant_send` invocation. `cargo check` clean. Frontend untouched.

## Session 99 — 2026-05-18 — Assistant cwd lands at common ancestor (FiveM workspace fix)

Symptom: Assistant told user "Your FiveM server is running in a `[voice]` resource directory" — model's cwd had landed inside a single resource folder instead of `resources/` where every resource is visible. Same on Trey's machine.

Root cause: AutoSync's `folders_clone()` yields one `FolderWatch` per resource. `roots[0]` was used as `cmd.current_dir`. Resources beginning with `[bracket]` (`[` = 0x5B in ASCII, before letters) sort first, so `[voice]/` won.

Fix in [assistant/mod.rs](../src-tauri/src/assistant/mod.rs): `common_ancestor()` helper computes the lexical common parent of all roots. When the AutoSync path produces >1 root (and there's no pinned sidecar + no explicit `current_root`), prepend the ancestor to `roots`. `roots[0]` is now e.g. `<server>/resources/` instead of `<server>/resources/[voice]/`. The individual folder roots stay in the list so MCP path-resolution behaviour is unchanged. Ancestor must (a) share a real path beyond fs root, (b) have a parent, (c) exist as a directory — otherwise we fall through to the old behaviour.

S98 sidecar interaction: existing pinned conversations keep their captured cwd (preserves --resume continuity even if narrower); new conversations + legacy non-pinned conversations get the ancestor.

`cargo check` clean. Backend-only.

**Pending verify (combined S98+S99):** real-world send round-trip; multi-writer scenario (Trey machine); optional follow-up = surface CLI compaction events in UI so the user knows when earlier context got summarized.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Source at **v0.4.10-alpha** (committed — 3 commits; v0.4.8 binary live in `rift-releases`, not yet shipped via Velopack — delta chain investigation required first). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater, NSIS perUser installer.

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
