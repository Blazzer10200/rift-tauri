# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-05-29 (e) — SftpOps trait + DriftScanner offline tests (#265 Wave B Phase 1)

**Done + verified (full `cargo test` 115 pass / 0 fail / 2 ignored; 0 rustc + 0 clippy warnings in touched files):**
- `64b79ef` — new `SftpOps` trait (`src-tauri/src/sftp/sftp_ops.rs`): 8 sync-facing `SftpClient` methods, object-safe via `#[async_trait]` (now a direct dep). `impl SftpOps for SftpClient` delegates via inherent-method precedence (`self.method()` → inherent, no recursion). `DriftScanner` field + `new()` → `&dyn SftpOps`; 4 Arc call sites `&sftp`→`&*sftp` (`commands/sync.rs:384`, `auto_sync.rs:880/1214/1784`). Engine field stays `Arc<SftpClient>` (Phase 2). `MockSftp` + 2 first-scan tests.
- `c6bf279` — 4 baseline-seeded tests (`SyncSnapshot::for_path`, all `sha1:None` → pure stat path): ToDelete / ToPush-edited / Conflict / Synced. **6 drift tests total, fully offline.**
- `SyncSnapshot::for_path` already existed (Wave A) — free.

**⚠️ Untested-but-language-guaranteed:** `impl SftpOps for SftpClient` delegation never runs at runtime (SftpClient needs live SSH); correctness rests on Rust inherent-precedence (solid). Real drift via prod DOES route SftpClient through `&dyn SftpOps` now.

**RESUME HERE (next, cheap → fill against same `MockSftp`):**
1. Easy adds: `ToDeleteRemote` (`.with_mirror(true)` + baseline); guard paths `RemoteMissing` (empty remote + `remote_exists`→false) + `SuspiciousEmptyAborted` (≥10 baseline, listing <half) — make `MockSftp.remote_exists` configurable.
2. Fiddly: hash-path trio (false-conflict collapse, first-scan equality, remote-jitter) — need `MockSftp.get_remote_sha1` returning a digest matching `compute_sha1` of real local bytes.
3. **Phase 2** (bigger, gated on `AppHandle` engine work): extend/split `SftpOps`+`SftpExec` so `remote_bridge` keeps non-trait methods, flip engine `sftp` field + `.sftp()` getter (7 consumers) to `dyn` → unblocks `flush_batch` tests (#21.1).

**Gotcha:** no live `tauri dev` this session (the 2 running `rift-tauri.exe` are the INSTALLED app at `AppData\Local\Rift`, not `target/`) — so manual `cargo test` was safe. Build target = `C:\cargo-targets` (global env).

---

## Session 2026-05-29 (d) — v0.4.38 SHIPPED (updater fix released)
Bump `f71a3bb` → `release.ps1` (via **powershell.exe** — pwsh 7 not on PATH; script is ASCII-safe for 5.1) → NSIS + `gh release create` + SHA256 MATCH. Live: rift-releases tag v0.4.38, `latest` API serves it. Ships `5a3618c` (`mailto:**`→`mailto:*` openUrl-poison fix) + `5a013dc` (in-app `download_update` + browser fallback).

**⚠️ Download path STILL runtime-untested — un-testable from THIS release (chicken-and-egg):** 0.4.38 is newest → a 0.4.38 client sees no update; only ≤0.4.37 clients see it but have the bug baked in → Download fails (known caveat, not a test). **First testable on 0.4.38→0.4.39 — verify progress→NSIS→relaunch THEN.** setup.exe: `C:\cargo-targets\release\bundle\nsis\Rift_0.4.38_x64-setup.exe`. ⚠️ ≤0.4.37 clients need ONE manual install of 0.4.38 first.

---

## Session 2026-05-29 (c) — 830a851 + 3487dcb (NOT pushed)
Assistant UX polish, all frontend, user-driven (check 0/0/0):
- Composer streaming indicator: top-edge bar (squared line over input) → model-tinted **border-only ring** breathing around whole frame (`.composer.streaming::before` inset:0). No overlap.
- Removed `.send-sweep` (2px bar streaking −72vh up chat on send — the "weird box"; never clipped). `fireKey` now drives only the send-btn ripple.
- Activity pills toggle: `openActivity()` closes dock on 2nd click when already on Activity tab.
- ActivityPanel: running rows + slowest-tool = buttons → `jumpTo()` scrolls transcript to the call + flashes it via new `actnode-<id>` anchor on `.tl-node` (MessageBubble:525). Shells AND agents resolve (`agentSpawns` id == Task tool-block id). Cancelled parallel calls split out of failed count. Tool-mix: full-name tooltip + widened label + **expandable "+N more"** (`toolsExpanded`).

## Session 2026-05-29 (b) — 07710e3 (pushed)
CR1–CR5 from v0.4.37 review landed (check 0/0/0). ISSUES **#14/#15 CLOSED** (code-signing declined). Full detail: `git log 07710e3`.

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
