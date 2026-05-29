# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-29 (c) — updater FIXED + upgraded (committed, NOT released)

**Done + verified (cargo check + npm run check both green, seen live):**
- `5a3618c` fix(update): **`mailto:**` → `mailto:*`** in `capabilities/default.json`. THE bug behind the "Couldn't open the installer link / error deserializing scope" crash. v0.4.36 added scope `https://** http://** mailto:**` but `mailto:**` is an invalid glob (recursive wildcard must be its own path component); opener deserializes the whole allow-list on first `openUrl`, so that one entry poisoned EVERY openUrl incl. Download. v0.4.36's "fix" never worked. `https://**`/`http://**` are valid (verified vs glob 0.3) — left as-is. Same commit: `check_for_updates` now returns `Err` on real failures (was `Ok(None)` for everything → looked like "up to date"); `Ok(None)` reserved for current/404/no-asset.
- `5a013dc` feat(update): in-app download. New `download_update` cmd streams installer to `%TEMP%/rift-update`, emits `update://download-progress`, frontend launches via `openPath`; **falls back to browser `openUrl` on any failure (never regresses)**. UpdateDialog gains `downloading` state + progress bar.

**In progress — git-ship (user invoked, INTERRUPTED):**
- Attempted 0.4.38 bump (package.json/Cargo.toml/tauri.conf.json) + CHANGELOG v0.4.38 entry, but the batch was CANCELLED (guard hook blocked a compound `grep` cmd). **VERIFY with `git status` — bump likely did NOT apply / is partial. Redo cleanly.**
- Read `scripts/release.ps1`: preflight checks version lockstep, runs `npm run tauri build`, `gh release create` to `Blazzer10200/rift-releases`, SHA256 round-trip. Unattended. NO `--prerelease` (latest API excludes them).

**RESUME HERE (do in fresh session):**
1. `git status` — confirm tree state; redo 0.4.38 bump if needed (3 files + `Cargo.lock` via cargo check + CHANGELOG top entry must say v0.4.38).
2. Commit the bump, then `pwsh ./scripts/release.ps1`.
3. After release: do ONE real end-to-end update test (Download → NSIS → relaunch) — the in-app path is compile/type-verified only, not runtime-tested.
4. ⚠️ Existing clients ≤0.4.37 stay broken (capability baked into binary) — they need ONE manual install of 0.4.38; after that in-app updates work.

**Don't retry / gotchas:**
- Severe bash tool-result delivery lag ALL session (results arrived in huge delayed bursts — ignore if it recurs, just wait). User had another session fixing the bash issue.
- `guard-wrong-tool-bash.sh` BLOCKS compound bash containing `grep`/`cat`/`sed` and foreground `npm run tauri build`. Use Read/Grep tools; run builds via `run_in_background` or release.ps1.
- Don't run `release.ps1`/`tauri build` in foreground or at high context — it's a 10-15min irreversible public release.

---

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
