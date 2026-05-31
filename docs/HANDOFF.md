# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-31 (f) — v0.4.44 SHIPPED (assistant UI wave: merged panel + step rail + tooltip hardening)
Shipped the whole 05-31 UI wave as **v0.4.44** (frontend-only, no backend behavior change). Bundles: **(d, was committed)** numbered step rail + captions, AskUserQuestion permission fix, right-anchor user bubbles, tool-run collapse; **(e, was uncommitted)** side panel merged into ONE scrolling surface (`ActivityPanel` rewrite, `SidePanel` thin wrapper, `liveActivity` surfaces all pending tools w/ friendly captions); **(f, today)** `tooltip.ts` hardening — `:focus-visible` gate (no reappear-after-click), hide-on-scroll + reposition-on-resize (no drift in streaming panel), single active tooltip, auto-kbd chip lifting trailing "(Ctrl+W)" shortcuts into the styled chip. `npm run check` 0/0/4110; NSIS + SHA256 round-trip via `release.ps1`. Tag `v0.4.44` on rift-releases. **NEXT:** (1) richen heterogeneous group captions ("Read 2 files · 1 command" vs generic "Running 3 actions"). (2) Optional lane: move giant help-paragraph tooltips (SyncPage "Mirror mode…", ChatTabsBar compaction ~250 chars) out of hover popovers into an info affordance — flagged, not a mechanism bug.

## Session 2026-05-31 (e) — side panel merged into one live surface (shipped in v0.4.44)
Reworked the right-side panel from a 2-tab (Session/Activity) split that left ~70% void into ONE scrolling surface. `npm run check` 0/0/4110 clean throughout; live-verified via CDP across many real turns.
- **Merged panel (`ActivityPanel.svelte` rewritten, `SidePanel.svelte` now a thin wrapper):** stacks top→bottom — **Now strip** (live turn headline + elapsed, streaming-only) → **Running** → **Tasks** (TodoWrite + progress, folded in from TasksDock) → **Outputs** (files touched) → **Sources** (web refs) → **Tool mix** histo → **Insights**. Dropped the redundant "This session" stat card (tok/s·tools·cost already in the status bar). Tabs gone (`oldTabs:0`).
- **Live region fix (`helpers.ts::liveActivity`):** was Bash+agents ONLY → now surfaces ALL pending tools (Read/Edit/Grep/Glob/Write/WebFetch…) + a thinking state. Tool rows labelled via `captionForTool` (same friendly captions as the transcript rail — "Reading package.json", not raw paths). Agent-launch tools (`Task`/`Agent`) skipped in the tool branch (ride agentSpawns). Composer live pills gained a matching `toolCount` pill so counts don't undercount.
- **Polish:** thinking is a turn STATE (owned by Now strip), not a Running row — dedup'd. Running identity icons (Terminal/Wrench/Bot) use `mon-pulse` (opacity), NOT `mon-spin` — spinning a wrench looked broken; only the Now-strip Loader2 spins. Insights font unified (both `<b>`, was mono vs sans mismatch).
- **Workflow enforcement (`~/.claude/`):** session regressed to echo flush-spam (misreading batched tool results as "stuck"). Added hook block in `guard-wrong-tool-bash.sh` (bare no-op `echo` → exit 2, tested 13/13) + critical memory `feedback_workflow_efficiency.md`. These are in `~/.claude/`, NOT this repo.
- **NEXT:** (1) ship the d+e wave as a release (bump 3 files + Cargo.lock + CHANGELOG). (2) richen heterogeneous group captions. (3) NEW LANE: deeper Activity/side-panel iteration if wanted. Files modified (uncommitted): `src/lib/components/assistant/{ActivityPanel,SidePanel,Composer}.svelte`, `src/lib/state/assistant/helpers.ts`.

## Session 2026-05-31 (d) — assistant turn UI: numbered step rail + AskUserQuestion fix (committed, NOT shipped)
Committed the full 05-31 UI wave (b+c+d) to main, **no version bump** — ship deferred to tomorrow. `npm run check` 0/0/4110, `cargo check` exit 0. Live-verified via CDP.
- **Numbered step rail (`MessageBubble.svelte`, `ToolChip.svelte`, NEW `toolCaption.ts`):** every action unit (chip / coalesced group / edit) gets a sequential number rendered AS the rail bullet (status-colored: done-green / pending-pulse / error-red) + a plain-language caption inline in the head row. Caption = model's "Step N —" divider title if narrated, else synthesized (`captionForTool`/`captionForGroup`). Numbering is ours — sequential even on silent turns. `numberActions()` post-pass after `coalesceToolGroups()` folds preceding dividers, keeps orphans. Tool-group fold now `slide` (200ms). Replaces the old detached `Step N —` divider + bare-chip look.
- **AskUserQuestion fix (`mod.rs::handle_permission_request`, `MessageBubble.svelte`):** builtin AskUserQuestion was hitting the raw Allow/Deny permission bar (off the allowlist → `can_use_tool` prompt) and stalling (no headless surface). Now auto-denied at the permission layer with a steer to `mcp__rift__ask_user` (Rift's working card + answer injection); the dead AskUserQuestion chip is filtered from the timeline. Model re-asks via the rich card.
- **Discipline (`CLAUDE.md` Don't-do):** added "code-only, not prose" (no multi-line WHY-comment blocks) + "read once" (no re-reads) — TTFT/efficiency.
- **Earlier today (b+c, now committed):** vertical rhythm, quiet <3s thinking rows, trailing-activity row, tool-chip lowercase labels + neutral duration pill, edit-diff auto-expand, rail spine tint, user-bubble 82% right-anchor, GROUP_MIN=3 tool-group collapse, `scripts/cdp/send.sh`. Detail in git log.
- **NEXT (tomorrow):** (1) finalize the assistant-turn UI + ship the wave as a release; richen heterogeneous group captions ("Read 2 files · 1 command" vs generic "Running 3 actions"); deferred wave items — expanded-Bash nested-box lightening, Agent/Todo/AskUser card chrome unification, typography heading-scale, composer-pills↔activity dedup. (2) NEW LANE after that: Activity / side-panel work (`ActivityPanel`, `SidePanel`, `assistant.ui.panelTab` Session/Activity).

## Session 2026-05-31 (a) — v0.4.43 SHIPPED (#265 live-SFTP transfer coverage — detail in git log)
Additive test-infra release: 7 `#[ignore]` integration tests in `src-tauri/src/sftp/integration_tests.rs` (transfer/ops/remote_exec/list + worker-pool batch). No prod code touched. Env-gated `RIFT_TEST_SFTP_{HOST,PORT,USER,KEY}`; all 7 GREEN vs Proxmox LXC 121 (`192.168.1.16`). Commits `b4b5cf1`/`04e22bb`/`2c08adf`. **NEXT:** `flush_batch` = last HIGH-risk reconcile path uncovered → Wave B Phase 2 (engine `sftp` → `&dyn SftpOps`, 7 consumers).

## Session 2026-05-30 (f) — v0.4.42 SHIPPED (auth detection + conflict data-safety — detail in git log)
39-agent swarm audit → fixed crit/warn tier: auth subscription detection (claude.ai Pro/Max vs Console/API in `mod.rs::assistant_auth_probe`); 3 conflict-resolution data-loss fixes (`auto_sync.rs`); compaction 401 (re-inject key for API-key users). cargo+vitest+svelte all green. rift-releases tag **v0.4.42**.
- **OPEN — info tier (deferred, no code yet):** dead IPC surface (`scan_drift` + 5 registered-never-called cmds), `close_edit_in_place` never invoked, dead `serverKey` arg on local delete/rename, `ToDelete`→`ToDeleteLocal` rename, onboarding `dismissed` never resets on server delete, WebBrowser Go-button fires on `example.com` placeholder, `ctxPctBefore` reads active-not-target tab. Full detail in (f) swarm output.

## Session 2026-05-30 (e) — live UI↔backend CDP verification + stale-pill fine-tune (shipped in f, git log)
Drove app via CDP, confirmed every workspace wired to backend (auth probe, drift scan, SFTP list, chat spawn+stream+turn-end #242, smart-title, telemetry). StatusBar `isStale` gated on `queue>0||failed>0||conflicts>0` so idle+clean watching server stays green.
- **DEV ARTIFACT (no data loss):** saving the .svelte → HMR + my `location.reload()` reset frontend to offline/"No servers". Confirmed intact: `rift.json` holds endure-rp profile (`lastSelected`), `list_servers`→`["endure-rp"]`. `loadServers()` is wired to app-launch/Tauri-ready, NOT webview reload → plain reload won't reconnect.

## Session 2026-05-30 (d) — dev-tooling + test-infra (committed, no ship — git log)
CDP console capture (`14c84d2`: serve.cjs now subscribes `Runtime.enable`+`Log.enable`, ring buffer, `c.sh console`). Proxmox SFTP test target (`c9ef1fb`: LXC 121 `rift-sftp-test`@`192.168.1.16`, key `.secrets/rift-sftp-test`, helper `scripts/sftp-test-target.sh`; MCP stays read-only — see `docs/design/proxmox-sftp-test-target.md`).

## Sessions 2026-05-30 (b)+(c) — v0.4.40 + v0.4.41 SHIPPED (detail in git log)
v0.4.41 (`a2cb0cd`): silent-401 fix (strip `ANTHROPIC_API_KEY` from spawns) + sftp connect-err hints. v0.4.40 (`7c8e17d`): bg-process turn-end #242 + auth-lockdown. **Open:** orphan-reaping (bg children survive app-exit → Win Job Object KILL_ON_JOB_CLOSE).

### RESUME HERE — Trey (collaborator) onboarding, in flight
Connection **WORKS** now. Root cause of the multi-hour SSH failure was **NordVPN** strangling the Tailscale tunnel → `WSAEACCES` "Permission denied" on connect (NOT keys/server — both server-side verified fine). Fix: close Nord OR split-tunnel `tailscaled`+Rift to bypass the VPN. Trey SSH user = `treyday` (uid 1001, key in `/home/treyday/.ssh/authorized_keys` on CT120); connects to fxserver tailnet IP `100.122.178.19` (NOT LAN `.170`).
**Current blocker:** Assistant shows **"401 Invalid authentication credentials"** (v0.4.40 detection working — surfaced a real error vs the old cryptic exit). Diagnosis: a bad API key configured in Rift **shadows** his claude login (`mod.rs:1011` — api key > OAuth precedence). FIX given to Trey: clear the API-key field in Rift Settings → `claude` + `/login` (Pro/Max) → pill flips green. Fallback: stale system `ANTHROPIC_API_KEY` env var (`echo $env:ANTHROPIC_API_KEY` → remove). **Awaiting Trey's result.**

---

## Earlier 2026-05-29 (all shipped — detail in git log)
v0.4.38 updater fix (`5a3618c`/`5a013dc`); assistant UX polish (`830a851`/`3487dcb`); v0.4.37 review CR1–CR5 + ISSUES #14/#15 closed (`07710e3`).

## Open work (not started)
**#20 hot-file splits** — `assistant.svelte.ts` 2314L (M8 streaming + M9 send open, brief `docs/design/assistant-svelte-split.md`); `assistant/mod.rs` 2795L (worst backend); `auto_sync.rs` 2232L. M8/M9 want a conversation-playback test harness first. Decisions: **CR-UX** trust enum (collapse to 2-level, drop dead `full`); **#17** two-repo (if going public). Also: code lanes (Files diff-dot, RCON `rcon_resource`/`dev_cycle`), #4 UX sweep. Full live queue: `docs/ISSUES.md`.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (#7):** `showOnboarding` MUST keep all 4 conditions (`!dismissed && serversLoaded && servers.length===0 && defaultSshKeyExists===false`). Loosening flashes onboarding for existing users. `defaultSshKeyExists` null=unknown→don't show.
- **Trust (git-rcon):** `effective_trust_level` derives from `allow_remote_shell` when unset (on→full/off→readonly) — don't change to a default that silently grants git-write. git tools gate server-side in mcp_server dispatch AND via --allowed-tools (defense-in-depth). force push rejected; all params §11-validated.
- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo. Hardcoded in `release.ps1` + `commands/update.rs::RELEASES_REPO`.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Smart-title `titleGenerated` (per-tab): true on disk-load + rename → auto-gen runs ONCE per new convo. Don't default-false on load (re-spams Haiku). `assistant_generate_title` = headless Haiku, no session/tools.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations`. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: always `--input-format stream-json` + `--permission-prompt-tool stdio` + initialize handshake + stdin kept open. `--allowed-tools` mode-aware: bypass/auto = full BUILTINS+mcp; prompting modes = SAFE_BUILTINS + SAFE_MCP + GIT_READ only (git write + mutating MCP ride can_use_tool prompt).
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- SidePanel = ONE merged scrolling surface (`ActivityPanel.svelte`), NOT tabs anymore (05-31 e). `ui.panelTab` field still exists in state + `openActivity()` sets it to "activity" — harmless, dock has one view now. AssistantPane `dockOpen` = `ui.dockOpen && !!tab`. Live work = `liveActivity()` (helpers.ts) → ActivityPanel Running + Composer pills; labels via `captionForTool`.
- Live in-flight work has ONE source: `liveActivity()` in `state/assistant/helpers.ts`. ActivityPanel `running` + Composer live pills both call it — don't re-derive inline.
- `assistant.model` IS the literal `--model` CLI arg (`ModelSel`). `opus`=newest(4.8); `claude-opus-4-7`=pinned 4.7. No brackets (Rust `is_valid_model_name` rejects `[1m]`). Aurora hue via `modelFamily()`. Browser dock = REAL-width slide so native webview overlay tracks.
- Effort tiers (`ThinkingEffort`): none→`--effort low`, quick→medium, deep→high, **ultra→xhigh + `--settings '{"ultracode":true}'`** (ultracode = autonomous dynamic-workflow mode; CLI settings-key boolean, not a flag). All gated to non-haiku. TS `effortToFlag` MUST mirror mod.rs mapping. Don't expose ultra for haiku (thinking-depth section hidden there).
- Image paste: `assistant_send` flips `--input-format text→stream-json` w/ attachments. 20MiB cap.
- Settings is workspace (kbd **5**), `Ctrl+,` flips; no slideover scrim.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }`.
- Updater signing-key-free as of v0.4.34. Do NOT reintroduce `tauri-plugin-updater` / `createUpdaterArtifacts` / `.sig` / `latest.json` — see CHANGELOG v0.4.34.
