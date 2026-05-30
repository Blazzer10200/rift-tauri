# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-30 (c) — auth-clarity hardening (UNSHIPPED, on `main`, tree dirty)
Closes the silent-401 class the Trey arc exposed. **Not shipped** — code-complete + verified, awaiting `/git-ship` (version bump + release deliberately deferred per project rules). `cargo check` 0 err, `svelte-check` 0/0 (4109).
- **Silent system `ANTHROPIC_API_KEY` trap (root cause of "green pill but 401"):** `current_api_key()` only reads keychain/config, so a system env key was invisible to the probe yet inherited by the spawned `claude` → 401 under a different identity. Fix (`assistant/mod.rs`): (1) `AuthStatus.env_api_key_present` (detect via `std::env::var`); (2) **`claude_command()` builder strips `ANTHROPIC_API_KEY` from EVERY spawn** (probe ×2, send, title-gen, enhancer, compaction) — single source of truth; the configured-key send branch (`use_api_key`) re-adds the sanctioned Rift key after. Env keys never silently win on ANY claude call; (3) probe summary warns when an env key is being ignored (green-but-noted if logged in; actionable red if no login/no Rift key).
- **Bad-key 401 now actionable:** send error decoder gained a 401/`authentication_error`/`Invalid authentication`/`invalid x-api-key` branch → "configured key rejected, clear it in Settings" (if `current_api_key()`) vs "login expired, run `claude`" (else). Was falling through to the bare `claude exited with N — …`.
- **Frontend:** `AuthStatus.envApiKeyPresent` in `types.ts`; Settings API-key section shows a ⚠ note when an env key is set but unconfigured in Rift.
- **SSH connect-error decoder (`sftp/mod.rs::decode_connect_err`):** the raw `WSAEACCES`/errno from `client::connect` (the NordVPN-vs-Tailscale clash that cost Trey hours) now decodes to actionable hints — EACCES/10013→"VPN/firewall blocking, close NordVPN or split-tunnel"; refused/10061→"sshd down or wrong port"; timeout/10060/unreachable→"host offline or wrong Tailscale IP". Raw appended for logs. (`tunnel/mod.rs` connect left raw — secondary path.)
- Touched: `src-tauri/src/assistant/mod.rs`, `src-tauri/src/sftp/mod.rs`, `src/lib/state/assistant/types.ts`, `src/lib/components/settings/Settings.svelte`. `cargo check` 0/0, `svelte-check` 0/0. RESUME: `/git-ship` when ready (bump 3 files + Cargo.lock).

## Session 2026-05-30 (b) — v0.4.40 SHIPPED (bg-process turn-end #242 + auth-aware send guards)
Commit `7c8e17d`, release `rift-releases` tag **v0.4.40**, SHA256 MATCH, non-prerelease. Bundled:
- **#242/#240/#241 (prior session (a) — now compile-verified + shipped):** turn-end was gated on claude *exit* (`child.wait()`), but a `run_in_background` child keeps claude alive → UI stuck "streaming" for minutes / queue stranded (reproduced: 1m43s on bg `sleep 300`). stdout reader emits DONE on the `result` frame (`Arc<AtomicBool> result_seen`); 5s grace then `start_kill` **claude's PID only** (NOT taskkill /T) so detached bg child survives. stdout/stderr drains bounded 500ms+abort (#240). TTFT instrumentation (#241, ~1.1s first-turn floor). **Open:** orphan-reaping (bg children survive app-exit; proper fix = Win Job Object KILL_ON_JOB_CLOSE).
- **Auth lockdown (NEW):** Enter keybind bypassed the composer auth gate — `fire()` called `onsubmit` w/o checking `canFire` → fresh/logged-out users fired doomed turns ("claude exited with 1 —", empty stderr). Fixed: `fire()` (`Composer.svelte`) guards before clearing draft (text preserved); `send()` (`assistant.svelte.ts:1777`) gates EVERY send path at the chokepoint (slash cmds stay local); backend (`mod.rs:2986`) runs `assistant_auth_probe` on empty-stderr exit → reports not-installed vs not-logged-in. `cargo check` 0 + `svelte-check` 0/0 (4109).
- **Docs:** `DEVELOPING.md` gained "Remote connection (Tailscale)" subsection + reconnect runbook.

### RESUME HERE — Trey (collaborator) onboarding, in flight
Connection **WORKS** now. Root cause of the multi-hour SSH failure was **NordVPN** strangling the Tailscale tunnel → `WSAEACCES` "Permission denied" on connect (NOT keys/server — both server-side verified fine). Fix: close Nord OR split-tunnel `tailscaled`+Rift to bypass the VPN. Trey SSH user = `treyday` (uid 1001, key in `/home/treyday/.ssh/authorized_keys` on CT120); connects to fxserver tailnet IP `100.122.178.19` (NOT LAN `.170`).
**Current blocker:** Assistant shows **"401 Invalid authentication credentials"** (v0.4.40 detection working — surfaced a real error vs the old cryptic exit). Diagnosis: a bad API key configured in Rift **shadows** his claude login (`mod.rs:1011` — api key > OAuth precedence). FIX given to Trey: clear the API-key field in Rift Settings → `claude` + `/login` (Pro/Max) → pill flips green. Fallback: stale system `ANTHROPIC_API_KEY` env var (`echo $env:ANTHROPIC_API_KEY` → remove). **Awaiting Trey's result.**

## Session 2026-05-29 (f) — v0.4.39 SHIPPED (assistant /clear + queue fix) — superseded by v0.4.40
`95ab30c`+`b135262`. `onError` now fires `onTurnComplete` (queue drained from every terminal path, not just success); real in-place `clearConversation()` (`state/assistant/tabs.ts`) re-keys the same tab/pane vs the old hidden `/new` alias. Frontend-only. Detail: git log.

## Session 2026-05-29 (e) — SftpOps trait + DriftScanner offline tests (#265 Wave B Phase 1)
`64b79ef`+`c6bf279` — `SftpOps` trait (`sftp/sftp_ops.rs`); `DriftScanner` takes `&dyn SftpOps`; 6 offline drift tests vs `MockSftp` (115 pass). RESUME: **Phase 2** flip engine `sftp` field → `dyn` (7 consumers) → unblocks `flush_batch` tests (#21.1). Detail in git log.

---

## Earlier 2026-05-29 sessions (all shipped/pushed — detail in git log)
- **(d)** v0.4.38 SHIPPED — updater fix (`5a3618c` mailto-glob openUrl-poison + `5a013dc` in-app `download_update`). Now superseded by 0.4.39.
- **(c)** `830a851`+`3487dcb` — assistant UX polish (streaming border-ring, removed `.send-sweep`, Activity `jumpTo()` anchors). Pushed in session (f).
- **(b)** `07710e3` — v0.4.37 review CR1–CR5; ISSUES #14/#15 CLOSED (signing declined).

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
