# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-29 (c) — 830a851 + 3487dcb (NOT pushed)
Assistant UX polish, all frontend, user-driven (check 0/0/0):
- Composer streaming indicator: top-edge bar (squared line over input) → model-tinted **border-only ring** breathing around whole frame (`.composer.streaming::before` inset:0). No overlap.
- Removed `.send-sweep` (2px bar streaking −72vh up chat on send — the "weird box"; never clipped). `fireKey` now drives only the send-btn ripple.
- Activity pills toggle: `openActivity()` closes dock on 2nd click when already on Activity tab.
- ActivityPanel: running rows + slowest-tool = buttons → `jumpTo()` scrolls transcript to the call + flashes it via new `actnode-<id>` anchor on `.tl-node` (MessageBubble:525). Shells AND agents resolve (`agentSpawns` id == Task tool-block id). Cancelled parallel calls split out of failed count. Tool-mix: full-name tooltip + widened label + **expandable "+N more"** (`toolsExpanded`).

⚠️ **Uncommitted, NOT mine** — `commands/update.rs` (check_for_updates → `Err` on real failures, was silent `Ok(None)`) + `capabilities/default.json` (`mailto:**`→`mailto:*`) already dirty this session; left untouched. Review before next push.

## Session 2026-05-29 (b) — 07710e3 (pushed)
CR1–CR5 from v0.4.37 review landed (check 0/0/0). CR1: dock animates `.dock-wrap` reactive width → native syncBounds fires. CR4: ActivityPanel constant `mountTs` for `liveActivity` fallback. ISSUES **#14/#15 CLOSED** (code-signing declined). Full detail: `git log 07710e3`.

## RESUME HERE
main @ 3487dcb (2 commits ahead of origin, NOT pushed) + uncommitted backend (update.rs + default.json, not from this session — decide first). Still v0.4.37 ×3 — NOT released. **Next session = #20 hot-file splits** (user's chosen start). Order: `assistant.svelte.ts` 2314L — M8 (streaming pump) + M9 (send) open, brief `docs/design/assistant-svelte-split.md`; then `assistant/mod.rs` 2795L (worst backend); then `auto_sync.rs` 2232L. M8/M9 highest blast-radius — want a conversation-playback test harness first. Open decisions: **CR-UX** trust enum (rec: collapse to 2-level, drop dead `full` — nothing gates `trust_at_least("full")`); **#17** two-repo (only if going public). Also on board: #265 test `SftpOps`-trait unblock, code lanes (Files diff-dot, RCON `rcon_resource`/`dev_cycle`), #4 UX sweep.

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
- SidePanel = Session/Activity via `assistant.ui.panelTab`. AssistantPane `dockOpen` = `ui.dockOpen && !!tab` (Activity always renders).
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
