# Assistant — v0.2.57+ Roadmap

> **Author:** S70 research session, 2026-05-16. Read-only research only — no code shipped in this session beyond the v0.2.57 polish + tool-parity work already on `main` at `04b3ffd`. This doc is the executable plan for the next session(s).

## Context (60s read)

The Assistant is a CLI-direct wrapper around the user's installed `claude` binary, spawned by Rust w/ NDJSON over stdin/stdout (`tokio::process::Command`). NOT the Agent SDK — we go one layer deeper because Tauri webview can't host Node. Two real users (Blazzer + Trey) collaborate on the same FiveM workspace via the auto-sync engine.

What's shipped on `main` at handoff: full Claude Code tool parity (Read/Write/Edit/Bash/Glob/Grep/WebFetch/WebSearch + Rift MCP read/list/grep + TodoWrite), spawn `cwd` set to workspace root, frontend parser deny-list (was bug: strict allow-list dropped all built-ins), TasksDock summarize/icon coverage for all 9 tools, MessageBubble `white-space` fix (was rendering `\n` between `<li>`s as full empty lines), Markdown.svelte `tagFlatShortLists` pass that auto-columns 5+ short flat lists.

Uncommitted at handoff: nothing — `04b3ffd` is the checkpoint. Scratch dir `scratch/scratch_test.txt` is throwaway test data; delete or ignore.

Two CDP verification primitives are live in dev: `npm run cdp:serve` + `bash scripts/cdp/c.sh {state|eval|type|click|wait|shot|key|shutdown}`. Use these before asking the user to screenshot.

## Don't-Touch (perpetual)

Workspace boundary trust model — Claude can write/edit/Bash without per-call confirmation; user reverts via git. Per-user conversations at `~/.rift/assistant/conversations/<uuid>.json` — DO NOT sync. Per-user config at `~/.rift/assistant/config.json` — DO NOT sync. Trey-Mirror discipline (HANDOFF note: keep Trey OFF Mirror until on latest + fresh-Pulled baseline). Everything in `docs/HANDOFF.md` CRITICAL DON'T-TOUCH section.

## Phase 1 — Harness Pull-Through (~30 min)

**Why:** the Assistant feels generic. Pulling user's `~/.claude/CLAUDE.md`, slash commands, skills, MCPs makes it feel like *their* Claude Code.

**Status check first:** confirm hooks/skills/CLAUDE.md ARE loading today. The CLI's docs imply they do (we use `--append-system-prompt` not `--system-prompt`, and `--bare` only fires when an API key is configured — see [mod.rs:643](src-tauri/src/assistant/mod.rs#L643)). Test by adding a one-line user rule to `~/.claude/CLAUDE.md` (a string the model would never invent), firing a prompt that asks "what's your favorite color" or similar, and watching for the test string in the response. CDP-driven: `bash scripts/cdp/c.sh type '.composer textarea' '<probe>' Enter` + `bash scripts/cdp/c.sh wait` + state read.

**Edits if confirmed (current behavior already pulls CLAUDE.md/hooks/skills):** drop the two fences at [mod.rs:625](src-tauri/src/assistant/mod.rs#L625) and [mod.rs:628](src-tauri/src/assistant/mod.rs#L628) — `--strict-mcp-config` and `--disable-slash-commands`. To merge user MCPs alongside Rift's: read `~/.claude.json` for user's MCP servers, splice them into `write_mcp_config()` at [mod.rs:334](src-tauri/src/assistant/mod.rs#L334) under `mcpServers` alongside the `rift` entry. Settings toggle in Settings.svelte → Assistant section: "Use my full Claude Code config" (default ON for piggyback, default OFF for API-key mode), gates the two flag drops.

**Edits if hooks AREN'T loading today (less likely):** investigate why, possibly the spawn env is missing something like `HOME` resolution. Surface the gap before fixing.

**Verify:** fire a prompt that exercises a known user CLAUDE.md rule (e.g. compression rules → response uses w/ for "with"). Surface a "Custom config · N skills · M MCPs" pill in `AssistantHeader.svelte` next to the workspace chip.

## Phase 2 — Session-ID + Native Resume (~half day)

**Why:** replace our hand-rolled Human:/Assistant: history replay ([mod.rs:537-552](src-tauri/src/assistant/mod.rs#L537-L552)) with the CLI's native session continuation. Cheaper tokens (no full-history replay), better context, and unlocks `--max-budget-usd` + `--max-turns` + prompt-caching compounding.

**Edits:** in `assistant.svelte.ts`, mint a UUID when `currentConvoId` is assigned (already happens at [send():470](src/lib/state/assistant.svelte.ts#L470)) and pass it through `invoke("assistant_send", { ... sessionId })`. In `mod.rs::assistant_send`, drop the `build_prompt(history, prompt)` call and replace with: first turn → spawn with `--session-id <uuid>`, subsequent turns → spawn with `--resume <uuid>`. Stop emitting `history` from the frontend. The CLI handles its own persistence per-session in `~/.claude/projects/<cwd>/`.

**Caveat:** the CLI persists sessions under `~/.claude/projects/<cwd-hash>/`. Multi-user collision: when both Blazzer and Trey work in the same `cwd`, do their session IDs collide? They shouldn't (UUID-keyed), but verify. Also: `claude project purge` exists if cleanup is needed.

**Wire `--max-budget-usd`** as a Settings option — Claude exits with error if exceeded. Surface as a tasteful "limit hit" notice with retry button. (`--max-turns` was in an earlier draft of this doc; S72 found it doesn't actually exist as a flag in the current CLI — only `--max-budget-usd` shipped.)

**Verify via CDP:** fire turn 1, fire turn 2 referring to turn 1's context, check the response shows continuity. Backend: log the spawn args, confirm `--resume <uuid>` is on turn 2.

## Phase 3 — Rift-Native Sprint (~1 focused session)

**Why:** the differentiator. No Cursor, no Cline, no Continue has lock-presence + drift-aware + remote-Bash because none of them have a sync engine underneath.

**Build a `WorkspaceContext` struct in `assistant/mod.rs`** that gathers: workspace roots, current LockPresence foreign-locks (subscribe to `autosync://locks` from `lock_presence.rs:114`), last 5 DiagBus events (subscribe to the broadcast at `diagnostics/mod.rs:103`), pending-drift counts from the drift_scanner. Serialize to a single-line addendum that gets concat'd onto `RIFT_SYSTEM_ADDENDUM_TOOLS` at [mod.rs:528](src-tauri/src/assistant/mod.rs#L528) per-turn (not globally — needs to update with state). Format: "Workspace is multi-writer. Active edits by others: Trey on inventory.lua (started 3m ago). Recent sync events: 2 uploads ok, 1 drift scan found 0 conflicts. If reading a file older than 30s, re-read before editing."

**Remote-Bash tool** — add a new MCP tool `mcp__rift__remote_bash(command, timeout?)` in `mcp_server.rs` that opens a russh exec channel against the active SSH session (auth via the existing PinningHandler in `transport/ssh_handler.rs`, keepalives via the same Config block as `tunnel/mod.rs:68-74`). Streams stdout/stderr back as the tool result. Surface as a Settings toggle "Allow remote shell" defaulting OFF. First fire shows a one-shot banner.

**Session-lock for remote commands** — both users running `pm2 restart` simultaneously is a footgun. Use a workspace-scoped advisory lock (extend `LockPresence` with a `remote_shell` lock key) — queue commands behind it; show a "Trey is running a remote command" indicator if held.

**Verify:** open a fresh convo, ask Claude "what's currently changing in this workspace" with another file being edited remotely via Trey's spoof (touch a `.rift-lock` file manually). Confirm Claude mentions it. For remote-Bash: ask `run pm2 status on the remote` and confirm real output.

## Phase 4 — UX Polish Pass (1-2 hr each, ship piecemeal)

**Diff view in Edit op-cards** — `TasksDock.svelte` op-card body currently dumps raw `old_string`/`new_string` JSON. Replace with a `diff` library (recommend `diff` npm package — small, single-string diff is its core competency, outputs hunks we render as red/green spans). Markdown.svelte already has diff-fence rendering at lines 11-66 — extract that into a reusable `<Diff>` component, use in op-card.

**Per-message cost + model badge** — `result` envelope has `total_cost_usd` per turn already captured at [assistant.svelte.ts:799](src/lib/state/assistant.svelte.ts#L799). Attach per-message, not just session-total. Surface as a tiny pill in `MessageBubble.svelte`'s `.role-row` next to "Claude".

**@-file mention picker in composer** — Composer.svelte gains a `@` trigger (mirror of slash menu at lines 62-71). Opens a fuzzy file picker scoped to workspace root. Inserts the path as plain text into the draft. Use a small fuzzy library (`fuse.js` or hand-roll — only ~50 files in typical FiveM resources).

**Code-block copy buttons** — Markdown.svelte already has the parsed HTML at [line 142](src/lib/components/assistant/Markdown.svelte#L142). Add a `walkCodeBlocks(html)` pass that injects a copy button per `<pre>`. ~30min.

**Conversation search in HistoryDrawer** — text input at top of drawer, filters `assistant.conversations` by title substring. ~30min.

**Context-aware empty-state suggestions** — `EmptyState.svelte` suggestions array is hardcoded ([lines 19-35](src/lib/components/assistant/EmptyState.svelte#L19-L35)). Pass workspace metadata (we know if it's a FiveM resource folder via `fxmanifest.lua` presence or similar marker) and swap suggestions accordingly. Stack-aware prompts: "list every event handler", "find resources with missing dependencies", etc.

**Misc small fixes** — stale `/tools` slash notice text at [assistant.svelte.ts:927](src/lib/state/assistant.svelte.ts#L927) still says "read-only" + lists only 3 tools (pre-parity ship). Update to reflect full toolset.

## Phase 5 — Background Agents (~1 session, new UI surface)

**Why:** `claude --bg <task>` returns immediately with a session ID. `claude agents` lists them. `claude logs <id>` tails output. `claude attach <id>` resumes. `claude respawn` restarts stopped. This is a separate concurrency model from chat — kick off long audits while continuing the conversation.

**Wire** a new `assistant_spawn_background(task)` Tauri command that invokes `claude --bg ...`, captures session_id from stdout. Persist the agent list at `~/.rift/assistant/background-agents.json`. Poll `claude logs <id>` periodically and surface progress in a new "Background Agents" tab or dock section.

**UI surface** — could be a 3rd tab next to History + Tasks docks, or a sidebar mode. Open question — choose based on space.

## Verification Approach (all phases)

CDP-driven: `npm run cdp:serve` once per dev session, then per-phase verification via `bash scripts/cdp/c.sh`. Each phase needs at least one CDP-runnable smoke test that touches the new surface end-to-end. Document the test command in commit message.

For Rust changes: tauri dev auto-rebuilds on save. Don't run `cargo check` while dev is alive (CLAUDE.md). Wait for the rebuild then re-verify via CDP.

For Svelte changes: HMR is instant, but state resets on file save — re-navigate to Assistant tab + re-fire test prompt.

## Open Questions to Resolve Early

1. **Do user hooks actually fire today?** Probe before Phase 1 edits.
2. **Does `--bare` toggle correctly for API-key mode?** Verify both auth paths.
3. **Multi-user session-id collisions in `~/.claude/projects/<cwd-hash>/`?** Verify before Phase 2.
4. **Does the CLI keep the russh tunnel alive for the lifetime of a multi-turn session?** Phase 3 remote-Bash question.

## Recommended Sequence

Phase 1 first (cheap, high signal, validates research assumptions about harness loading). Phase 2 second (unlocks Phases 3-5 features). Phase 3 third (the moat). Phase 4 in parallel with Phase 3 since polish is mostly frontend-only and orthogonal. Phase 5 last (new UI surface, can wait).

Total est: 5 focused sessions to land the full map. Each phase ships independently as its own version bump (v0.2.57 → v0.2.58 → v0.2.59 → v0.2.60).

## File Anchors (so the next session doesn't have to grep)

Backend: `src-tauri/src/assistant/mod.rs` (775L, all 15 Tauri commands), `src-tauri/src/assistant/mcp_server.rs` (447L, stdio JSON-RPC), `src-tauri/src/sync/lock_presence.rs` (343L, `.rift-lock` infra + `autosync://locks` emit), `src-tauri/src/diagnostics/mod.rs` (358L, DiagBus broadcast), `src-tauri/src/tunnel/mod.rs` (206L, russh session pattern for remote-Bash).

Frontend: `src/lib/state/assistant.svelte.ts` (1014L, store + NDJSON parser), `src/lib/components/assistant/AssistantPage.svelte` (172L, layout), `AssistantHeader.svelte` (282L, top chrome), `Composer.svelte` (634L, slash menu + model picker + queue), `MessageBubble.svelte` (326L, text + thinking + tool render), `TasksDock.svelte` (390L, op-card rendering), `HistoryDrawer.svelte` (318L), `EmptyState.svelte` (482L), `Markdown.svelte` (631L).

Settings: `src/lib/components/settings/Settings.svelte` (1060L, Assistant section starts around line 510).

CDP infra: `scripts/cdp/serve.cjs`, `scripts/cdp/c.sh`, `scripts/cdp/README.md`.
