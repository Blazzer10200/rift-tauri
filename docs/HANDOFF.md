# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-01 (c) — frontend file-structure clean + pushed to GitHub source repo
Pre-design cleanup. 1 commit `18206c0` (frontend-only, NOT a release — no version bump/CHANGELOG). `npm run check` 0/0/0 (4106 files). **Pushed to `Blazzer10200/rift-tauri` main** (`6a098f7..18206c0`, fast-forward — this is the SOURCE repo, separate from `rift-releases` update feed). Pre-push secret scan clean (only key-detection code + `sk-ant-…` placeholders; `.gitignore` covers `.secrets/`/`target/`/`node_modules/`/`Releases/`).
- **Deleted 4 dead components** (verified zero live refs, only stale comment-mentions): `assistant/StatusHub.svelte` (replaced by MessageBubble in-flight strip), `assistant/TasksDock.svelte`, `onboarding/EmptyStateBanner.svelte`, `workspaces/DisabledWorkspace.svelte` (not in WORKSPACES registry).
- **Merged `src/lib/util/` → `src/lib/utils/`** — `diag.ts`+`redact.ts` moved (git mv, history kept); rewrote 4 import sites (ActivityPanel, Settings → `$lib/utils/redact`; SyncActivityBanner, connection.svelte.ts → `utils/diag`). `util/` dir gone — one helpers dir now.
- **Renamed `assistant/EmptyState.svelte` → `AssistantWelcome.svelte`** (756L rich chat welcome) to end name-collision w/ generic 121L `shell/EmptyState.svelte` primitive. AssistantPane import+tag updated.
- **NEXT:** user moving to overall frontend *design* work via claude.ai. Structure now clean baseline: `components/` domain-grouped, `state/` (+ `assistant/` sub-split), `utils/`, `actions/`. Every remaining ~90 file is live/imported.

## Session 2026-06-01 (b) — shell layout + View dropdown + backdrop calm (frontend-only, committed, NOT shipped)
4 commits: `8c92037` rail+actions · `9916fd3` pane fix · `bdbb4ef` View dropdown · `a3cc3d9` backdrop. `npm run check` 0/0/0 throughout; CDP-verified live. Files: `AppShell.svelte`, `ChatTabsBar.svelte`, `AssistantPane.svelte`.
- **Activity rail raised full-height** (`8c92037`) — was stacked `tabs-rail` (full-width) **over** `[ActivityBar | pane]`, rail started *below* the tab. Restructured `.middle`: `ActivityBar` is now the full-height left column of `.body` (top-aligned w/ tab strip); tabs-rail + pane nest in a new `.content` flex-column right of the rail. Tab strip + bottom-border stop at the rail edge. Collapse anim preserved.
- **Top-right `.actions` zone-grouping** (`8c92037`, closes (a)'s PAUSED item) — History + ws chip in `.grp` (hug @5px); inter-zone gap 6→9px; `.vdiv` hairline before the view control so cluster parses `[context · status │ view]`.
- **Pane-height regression FIXED** (`9916fd3`) — the rail restructure moved `.pane` from a grid cell (auto-stretch) into the `.content` flex column w/o grow → WorkspaceShell `height:100%` panes collapsed to zero. Fix: `.pane { flex: 1 1 0 }`. (Watch for this on any future flex/grid swaps of `.pane`.)
- **View dropdown** (`bdbb4ef`) — replaced the 3-icon `.seg` w/ one `.view-btn` (panel icon + chevron + accent `.view-dot` when any panel open) opening a portaled `.view-menu`, à la Claude Code desktop options: rows **Web browser** (Ctrl+Shift+B), **Activity panel** (Ctrl+Shift+E), │ **Split pane** (Ctrl+\) — icon·label·kbd-chip·trailing check on active toggles. The two B/E shortcuts are NEW, wired in `AppShell.onGlobalKey` (Ctrl+Shift letter space, prev unbound).
- **Pane backdrop calmed** (`a3cc3d9`) — `.atmos-glow` breathed 9s (0.70↔0.95) → too busy for a terminal. Now static: accent 12%→6%/4%→2%, band 55%→42%, opacity 0.7; dropped breathe keyframes + reduced-motion rule. Grain unchanged.

## Session 2026-06-01 (a) — assistant UI polish wave (frontend-only, committed, NOT shipped) — detail in git log
Cohesive transcript + dock pass (`MessageBubble` major, `ActivityPanel`, `AssistantPane`, `ToolChip`). Transcript LEFT-aligned both roles (reverses 05-31(f) right-anchor); content-first headers (model+cost recede, brighten on hover); both chain rails → faded gradient threads w/ step-circle as sole marker; motion language extended to transcript + Now-strip crossfade + error-aware Tool-mix histogram. `npm run check` 0/0/0.
- **NEXT (flagged, not done):** transcript scrollbar styling (currently hidden). (Header simplification + atmosphere-grain/backdrop both DONE in (b).)

## Shipped 2026-05-30 → 05-31 (v0.4.40–v0.4.46, all SHIPPED — detail in git log)
- **v0.4.46** (`16db171`) permanent activity dock + quick-actions capsule + completion-ack motion (frontend-only).
- **v0.4.45** (`5fcf965`) mod.rs model-pin per conversation (`.model` sidecar — fixes `400 thinking blocks` brick on mid-chat model switch); resizable dock (`ui.dockWidth` 260–520).
- **v0.4.44** (`07710e3`-era) assistant UI wave: side panel → ONE scrolling surface (`ActivityPanel`/`SidePanel` thin wrapper), numbered step rail + captions, AskUserQuestion permission fix, `tooltip.ts` hardening. **NEXT (open):** richen heterogeneous group captions; move giant help-paragraph tooltips out of hover popovers.
- **v0.4.43** (`b4b5cf1`) 7 `#[ignore]` live-SFTP integration tests (`sftp/integration_tests.rs`), env-gated. **NEXT:** `flush_batch` coverage (engine `sftp`→`&dyn SftpOps`).
- **v0.4.42** (`7c8e17d`-era) 39-agent swarm audit: auth subscription detection, 3 conflict data-loss fixes, compaction 401. **OPEN info-tier:** dead IPC (`scan_drift`+5 cmds), `close_edit_in_place` unused, `ToDelete`→`ToDeleteLocal` rename — detail in 05-30(f) swarm output.
- **v0.4.40/41** (`7c8e17d`/`a2cb0cd`) silent-401 fix + bg-process turn-end #242 + auth-lockdown. **Open:** orphan-reaping (Win Job Object KILL_ON_JOB_CLOSE).

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
