# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-30 (a) — assistant harness: background-process queue hang (#242) + latency profile
**UNCOMMITTED in `src-tauri/src/assistant/mod.rs` — NOT compile-verified (dev was running, couldn't `cargo check`). Next session: quit dev → `cargo check --manifest-path src-tauri/Cargo.toml` → review `git diff` → test → ship.**
- **#242 (the real fix):** turn-end was gated on `claude` process **exit** (`child.wait()`), but a `run_in_background` child (dev server / localhost / `sleep`) keeps claude alive for its whole lifetime → `child.wait()` blocks for minutes → DONE_EVENT never fires → UI stuck "streaming"/queue stranded. **Reproduced live** (UI hung 1m43s on a bg `sleep 300`, claude.exe still alive). Fix: the stdout reader sets an `Arc<AtomicBool> result_seen` + emits DONE the instant the `result` frame lands (turn is semantically over). Main loop then waits with a 5s grace; if claude lingers past it (bg child pinning), `start_kill()`s **claude's PID only (NOT taskkill /T)** so the detached bg process survives. Terminal-event block now early-returns when `result_seen`; `status` became `Option<ExitStatus>` (None = killed-before-result → ERROR).
- **#240 (complementary):** bounded both `stdout_task.await`/`stderr_task.await` with 500ms `timeout`+`abort` — a surviving bg child holds the inherited stdout/stderr pipe write-ends so the drains never hit EOF. Kept; #242 is the primary.
- **#241 (instrumentation, keep):** `log::info!` TTFT (spawn→first-stream-line) + turn total (spawn→exit) in `assistant_send`. **Measured:** fresh first-turn TTFT ~1.1s (fixed floor = spawn + MCP-child spawn + init handshake + hooks + prefill); resume +0.5–0.7s scaling w/ convo length. Floor is the harness, delta is context-prefill.
- **Open (not done):** orphan-reaping — bg children survive app-exit (only `assistant_stop` does taskkill /T). Proper fix = Windows Job Object w/ KILL_ON_JOB_CLOSE. Separate change. Also: SessionStart-on-resume re-fire still unconfirmed (the ~1.1s floor didn't show an obvious 46K-token hook tax, but not isolated).
- **Note:** repeated session crashes this session (context bloat from large dev-log reads + CDP). Left wedged `claude.exe`/`sleep.exe` from testing may need a kill; dev server may still be running.

## Session 2026-05-29 (f) — v0.4.39 SHIPPED (assistant /clear + queue fix)
Two frontend-only assistant fixes, committed `95ab30c` + bump `b135262`, pushed (also flushed the 12 previously-unpushed commits incl. the v0.4.38 ship). `release.ps1` via **powershell.exe** → NSIS `Rift_0.4.39_x64-setup.exe` + `gh release create` + SHA256 MATCH. Live: rift-releases tag v0.4.39, non-prerelease (latest API serves it).
- **Queue hang fixed:** `onDone` drained the outbound queue but `onError` didn't → partial-stream-then-error (looks "completed") stranded the queued msg in "Queued (N)" forever. `TabState.onError` now fires `onTurnComplete`; drain centralized into idempotent `drainQueue(tab)`; microtask re-checks `streaming` + re-queues vs strands; tab activation (`openTab`/`cycleTab`/`setFocusedPane`/`addPane`) re-drains backgrounded queues.
- **`/clear` is real now:** was a hidden alias of `/new`. New `clearConversation()` (`state/assistant/tabs.ts`) re-keys the SAME tab/pane to a fresh session, flushing old convo to History first. Distinct from `/new` in picker + `/help`.
- **⚠️ First live updater test available NOW:** a ≤0.4.38 client → 0.4.39 is the first time the in-app Download→NSIS→relaunch path is end-to-end testable (v0.4.38 was un-testable, chicken-and-egg). Verify it.

## Session 2026-05-29 (e) — SftpOps trait + DriftScanner offline tests (#265 Wave B Phase 1)
`64b79ef`+`c6bf279` — new `SftpOps` trait (`sftp/sftp_ops.rs`, object-safe `#[async_trait]`), `DriftScanner` now takes `&dyn SftpOps`; 6 fully-offline drift tests vs `MockSftp` (115 pass/0 fail). RESUME (cheap): add `ToDeleteRemote`/`RemoteMissing`/`SuspiciousEmptyAborted` (configurable `MockSftp.remote_exists`); hash-path trio (`MockSftp.get_remote_sha1` matching `compute_sha1`); **Phase 2** flip engine `sftp` field → `dyn` (7 consumers) → unblocks `flush_batch` tests (#21.1). Detail in git log.

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
