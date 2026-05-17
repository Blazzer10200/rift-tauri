# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

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
