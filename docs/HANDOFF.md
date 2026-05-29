# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-28b (ultracode) — SHIPPED as v0.4.37 (4f9ca6e)
ultracode mode (4th THINKING DEPTH tier, Composer popover — wiring in DON'T-TOUCH) + git-rcon git tools + onboarding + side-panel split + 4 code-review fixes. Verified: `npm run check` 0/0, Rust clean, headless smoke ✓ (opus 4.8, exit 0). 6 OPEN follow-ups (CR1-5 + CR-UX; only CR1 a warning) + permission round-trip UNVERIFIED — all detailed in ISSUES.md.

## RESUME HERE
**v0.4.37 SHIPPED** (2026-05-28, commits 837408a + 4f9ca6e) — git-rcon git tools + onboarding flow + ultracode tier + side-panel split + 4 CR fixes. Working tree clean; all three version files + CHANGELOG in lockstep at 0.4.37. **Workspace docs/memory cleaned up 2026-05-28** — stale version/updater/Velopack refs corrected across CLAUDE.md, ISSUES.md, README, DEVELOPING, design briefs, and `~/.claude/memory/` rift files; `git-rcon-tools.md` brief archived (git half shipped). Optional follow-ups: CR1 (dock anim no-op) · CR2-5 + CR-UX (minor, see ISSUES.md) · live permission round-trip (throwaway repo, in-app mode selector).

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
