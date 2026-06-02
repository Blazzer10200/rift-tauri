# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-02 (cont'd) — Sync tabbed-hub + Activity demotion (deferred #1 DONE; frontend-only, on main, NOT pushed/shipped)
Knocked out deferred item #1. `npm run check` + `cargo check` 0/0; all 3 tabs CDP-verified live (Drift dashboard / ConflictsPage / ActivityFeed).
- **Sync = tabbed hub** — `SyncPage.svelte`: persistent hero + a **Drift/Conflicts/Activity** tab strip (`.sync-tabs`). Drift = existing totals/resources/dashboard/sticky-apply flow, **logic untouched** (wrapped in `{#if syncPage.tab === "drift"}`). Conflicts = full `<ConflictsPage/>` (replaced the old capped inline embed; drift now shows a lightweight "N conflicts need resolution" banner → switches to the tab). Activity = full `<ActivityFeed/>`. Sync toolbar (Sync now/kebab) gated to Drift. **Tab state on the `syncPage` store (`syncPage.tab`)**, not local — so deeplinks target it.
- **Activity demoted from top-level** — removed `activity` from `workspaces/index.ts` + `WorkspaceId` union/`WORKSPACE_IDS` (`workspace.svelte.ts`). Kbds renumbered: home·1 chat·2 sync·3 files·4 settings·5 (settings 6→5). 3 `setActive("activity")` deeplinks (`WatchedFoldersTable`, `RecentActivityCard`, `CommandPalette`) → `syncPage.tab = "activity"`.
- **Redesign is now essentially complete** on main. Only deferred #2 (Files detail pane — skipped) + #3 (Onboarding restyle — unverifiable w/o new-user path) remain. Ship the whole arc via /git-ship when ready.

## Session 2026-06-02 (overnight, autonomous) — redesign cont'd: Settings rebuild + Sync hero + calm rail (frontend-only, on main, NOT pushed/shipped)
User said "do the entire redesign, full autonomy, go til you can't." 4 commits, all `npm run check` 0/0, all CDP-verified live.
- **`d95f4f9` Settings single-scroll** — `SettingsPage.svelte` fully rebuilt into the Graphite-Ink layout: sticky left **scroll-spy index** (getBoundingClientRect + smooth jump), ghost section-head icon tiles, `.st-card/.st-row/.st-switch/.st-seg` primitives, body-portal `Select` (no native `<select>`), 8-swatch accent grid. **Preserves EVERY real feature** (mockup only showed a subset): full Speech/Whisper, SSH servers + fingerprint/bridge-token rotation, accessibility, assistant CLI/budget/compaction, real connection hero, SSH key path, diagnostics. Section id `network` kept internally (label "Server") so command-palette deep-links still work. **Legacy `Settings.svelte` now orphaned (no importers) — left in place per deletion-safety, flag for later removal.**
- **`1a65f88` calm rail** — `ActivityBar.svelte`: dropped the infinite breathing-stripe + looping halo on the active workspace icon (redesign forbids infinite loops on resting content). Active = static thin accent left-bar + ghost fill.
- **`9fed7bb` Sync hero + select swap** — `SyncPage.svelte`: **additive** state-driven mission-control hero band below the toolbar ("<server> is fully in sync" / "N changes need a sync" / "Sync paused", WATCHING eyebrow + live last-scan). Reads existing derived state; does NOT touch drift/apply/mirror engine or `toolbarActions`. + swapped the last live native `<select>` (Whisper mic) for themed `Select`.

### Triage finding (important): the app was ALREADY ~90% on-brand before tonight
Token foundation + prior sessions already made Sync/Files/Chat/Activity cohesive emerald-graphite. **Chat is already a flat-timeline** (numbered tool steps, collapsible thinking, session-review dock — the v0.4.44-46 wave). **Activity** already matches the redesign's engine-parity table (filter chips+counts, Pause/Clear). So most "remaining screens" needed no work.

### RESUME HERE — deliberately DEFERRED (judged too risky/low-value for unsupervised overnight)
1. ~~Sync full tabbed-hub + Activity demotion~~ — **DONE 2026-06-02 (cont'd), see top block.**
2. **Files detail pane** — SKIPPED: mockup's tree+detail would replace the app's local|remote two-pane, which is MORE capable. Don't regress it; at most add a code-preview detail without losing local/remote.
3. **Onboarding restyle** — unverifiable overnight (4-cond gate needs servers=0 + no ssh key; can't fake w/o disrupting real server). Do when a new-user path can be exercised.
- **Env note:** during dev, an HMR full-reload transiently empties `connection.servers` on views that don't re-fetch (Sync/Activity); navigating to Settings (`loadServers()`) repopulates from disk. NOT data loss. I reselected Endure RP → reconnected fine.

## Session 2026-06-02 — "Graphite Ink" redesign, foundation + Home + accent picker (frontend-only, committed on main, NOT pushed, NOT shipped)
Implementing the full visual redesign from `C:\Users\BLAZZER\Downloads\Rift App.zip` (design handoff = README + COMPONENT_MAP + DESIGN_TOKENS + JSX/CSS mockups; extracted to `Downloads/.rift-redesign-tmp/design_handoff_rift_redesign/`). User asked for autonomous overnight progress, incremental on main. **5 commits, all `npm run check` 0/0, all CDP-verified live.**
- **`a28ec01` token layer** — `app.css` `:root` recolored to true-neutral graphite (was blue hue 270), softened radii (control 6→8), added motion vocab (`--ease-page/-soft`,`--dur-*`,`--stagger`), `--bg-inset`, body radial glow. **Accent is now THEMEABLE: one `--accent-h` hue var drives the whole ramp** (accent/hover/active/fg/soft-ghost/ghost-border/ring); default emerald 163 (was violet 275). Status LEDs kept accent-independent. `ui-prefs.svelte.ts` gained `accentHue`/`presence`/`code` prefs (persisted + applied via `--accent-h`/`data-presence`/`--code-*`), `ACCENTS[8]`. + fix: null-guard accent parse (`Number(null)===0` clobbered default → pink; caught via CDP shot).
- **Select** (`lib/components/Select.svelte`) — reusable body-portal popover, flip-above, scroll/resize close, keyboard nav, selected=ghost+check. Use app-wide; **no native `<select>`** per spec. NOT yet adopted by existing selects.
- **Home workspace** (`lib/components/home/HomePage.svelte`) — net-new "Focal" landing. Real-data wired (NO fabricated counts): state-driven hero (offline/attention/clear off real `connection`), Ask-bar→composer, Sync-health (conflicts/dirtyEdits/pending/activityFeed/pill), Jump-back-in (real `assistant.conversations`). Registered `home` first in `WORKSPACES`/`WORKSPACE_IDS` (`workspace.svelte.ts`), **kbds renumbered 1-6**.
- **Appearance controls** (surgical insert into legacy `settings/Settings.svelte` Theme section) — live 8-swatch accent picker (verified emerald↔violet↔emerald repaints whole app + persists), density + presence segmented; `[data-presence=bold]` strengthens ghost fills in `app.css`.

(Foundation RESUME-HERE superseded by the overnight session block above — Settings + rail done, rest deferred there.)

## Earlier 2026-06-01 (a/b/c) — pre-design frontend cleanup + polish (all frontend-only, committed, detail in git log)
Pushed source repo to `18206c0`. (c) deleted 4 dead components, merged `util/`→`utils/`, renamed `EmptyState`→`AssistantWelcome.svelte` (756L). (b) full-height Activity rail + View dropdown (Ctrl+Shift+B/E new) + calmed backdrop. (a) transcript left-align + content-first headers + gradient chain rails. **Open NEXT:** transcript scrollbar styling (hidden).

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
- Settings is workspace (kbd **6** since 06-02 Home add; was 5), `Ctrl+,` flips; no slideover scrim.
- **Redesign accent is themeable via `--accent-h`** (app.css `:root`): never hard-code an accent oklch hue — write `oklch(L C var(--accent-h))`. `ui-prefs.setAccentHue()` persists + applies. Parse from localStorage MUST null-guard before `Number()` (null→0→pink). Status LEDs (`--ok/warn/danger/info`) stay fixed, NOT hue-derived.
- **IA:** `home` top-level kbd 1; **Activity is now a Sync tab (`syncPage.tab`), NOT a workspace** — removed from `WORKSPACES`/`WorkspaceId`/`WORKSPACE_IDS`. Kbds: home·1 chat·2 sync·3 files·4 settings·5. Deeplinks to Activity set `syncPage.tab = "activity"` (+ `setActive("sync")` if off-page). Home wiring real-data only (no fabricated counts).
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }`.
- Updater signing-key-free as of v0.4.34. Do NOT reintroduce `tauri-plugin-updater` / `createUpdaterArtifacts` / `.sig` / `latest.json` — see CHANGELOG v0.4.34.
