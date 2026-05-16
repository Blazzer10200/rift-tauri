# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 74 — 2026-05-16 — Phase 4 UX Polish Pass

**Committed (db19a44), bundles into v0.2.57-alpha alongside S69–S73.** Roadmap §4. One commit, seven items: (1) `/tools` slash notice rewritten for full Claude Code parity + conditional `remote_bash` line; (2) **diff view** in Edit op-cards — `TasksDock` swaps raw JSON dump for unified red/green list (single-string-replace, no ctx resolution); (3) **per-message cost+model badge** — `ChatMessage.costUsd`/`model` captured from `result`/`system:init` envelopes, "Sonnet 4.6 · $0.0772" pill in `MessageBubble`'s role-row; (4) **@-file mention picker** — new cmd `assistant_list_workspace_files` (SKIP_DIRS mirror, 4000-cap), Composer detects `@` after ws/start, three-tier fuzzy (basename-substring → basename-fuzzy → full-path); (5) **code-block copy buttons** — `annotateCodeBlocks()` injects a `Copy` chip per `<pre>` (skip `.diff-block`), click handler in `.md` root scopes via `closest('.code-copy')`; (6) **conversation search** in `HistoryDrawer` — title-substring filter, empty-results state; (7) **context-aware empty-state** — detects FiveM/RedM via `fxmanifest.lua` in cached `workspaceFiles`, swaps to stack-aware prompts. Also: `setRoot`/`clearRoot` clear `workspaceFiles` so picker doesn't show stale paths. CDP-verified live (search 21→3 on "Glob"; `@Mes` ranks `MessageBubble.svelte` first against 58-file `src/` tree; real turn produced cost+model pill; code-block chip rendered). **Phase 5** (`claude --bg` background agents) is final session and probably wants fresh ctx for the new tab/dock design discussion.

## Session 73 — 2026-05-16 — Phase 3 Rift-Native Sprint (collapsed)

Three deliverables, one commit (2c81a5c). (a) Per-turn `WorkspaceContext` addendum — live foreign-locks + AutoSync queue + recent DiagBus events spliced onto `RIFT_SYSTEM_ADDENDUM_TOOLS` at spawn. (b) `mcp__rift__remote_bash(command, timeout_secs?)` tool — exec over the auto-sync engine's live russh session via a loopback NDJSON bridge (`assistant/remote_bridge.rs`); env-gated by `RIFT_REMOTE_SHELL_ENABLED=1`. (c) Workspace-scoped `<remote_root>/.rift-shell.rift-lock` advisory lock — bridge acquires before exec, releases after; foreign holders surface as a "trey (4m)" pill in `AssistantHeader.svelte`. **Solution to "CLI spawn is fresh per turn"**: loopback TCP bridge (`127.0.0.1:<port>`, base64 token in env), MCP child dials per-call. ~5ms loopback RTT vs ~500ms cold SSH dial. Settings card + first-fire banner wired.

## Sessions 69-72 — 2026-05-15/16 — see git log (collapsed)

S69 fixed Assistant blank-response (cmd-shim `--output-format` mangling) + ext-thinking surface (`MAX_THINKING_TOKENS=10000` env). S70 shipped CDP autonomous-verify infra (`scripts/cdp/serve.cjs`, port 9223). S71 (Phase 1) `AssistantConfig.use_full_config` default ON — drops `--strict-mcp-config` + `--disable-slash-commands` so user MCPs/slash layer alongside Rift's. S72 (Phase 2) native `--session-id` / `--resume` replaced hand-rolled history replay; `AssistantConfig.max_budget_usd` + Settings "Per-turn cost cap". `--max-turns` does not exist as a CLI flag — only `--max-budget-usd` shipped. Don't retry `--settings '{"thinking":...}'` / `--permission-mode plan` for thinking — only the env var works.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri at `C:/AI Workflow/projects/rift-tauri/`. Last shipped: **v0.2.56-alpha** (`687edb8` on main). Tauri 2 + Svelte 5 + Rust + russh. Velopack updater (release repo `Blazzer10200/rift-releases`), NSIS perUser installer.

**Source state:** S69 + S70 checkpoint-committed on main but NOT version-bumped/shipped. Next ship = v0.2.57-alpha — bump version (3 files: package.json/Cargo.toml/tauri.conf.json), write CHANGELOG entry, run `scripts/release.ps1`.

**CDP autonomous-verify is live for dev** — start dev via `scripts/run-dev.bat` (sets WebView2 CDP port), then `npm run cdp:serve` in another shell. Claude can then `bash scripts/cdp/c.sh state|eval|type|click|wait|shot|key`. See `scripts/cdp/README.md`. Use this BEFORE asking the user to screenshot anything.

**v0.2.57 queue:** (a) EACCES auto-fix-perms affordance; (b) auto-Mirror on detected rename only; (c) integration test phase 1 — needs SftpClient trait abstraction; (d) dry-run Mirror preview pre-confirm; (e) `lib.rs` split (1771L, 52 cmds) → per-domain `commands/*.rs`; (f) `reqwest`+`ureq` consolidation — blocked on velopack 0.0.1298 sync `UpdateSource`; (g) LocalPane/RemotePane shared-logic extraction; (h) connection.connecting pill desync (pill stuck "Connecting" while engine reports `watching` — derived guard); (i) Diagnostics page canonical-skeleton (Ctrl+Shift+D); (j) Appearance settings — fill or hide.

**Multi-user warning:** Trey: keep him OFF Mirror until on latest + fresh-Pulled baseline.

**Don't reintroduce:** OpRail, TopBar (merged), rail kbd hints as bulky chips, StatusBar ⌘K pip, titlebar gear, StatusHero big H1, S37+S39 dev seeds, S40 floating purple Terminal pill, Settings Design/Sync/Editor sections, `.btn.lg`/`.pill.warn` legacy dead CSS, `bg-backlog.sh`, `diag_*` cmd names, `drift_watcher::spawn`/`run_tick`/`flush_cycle`, FiveM-specific framing in Assistant copy.

**Ship pipeline:** `scripts/release.ps1` IS the full ship (build → vpk pack → upload to rift-releases). `/git-ship` is only source commit+push. Run release.ps1 directly via `powershell -NoProfile -File ./scripts/release.ps1` after version bump.

---

## CRITICAL DON'T-TOUCH

- russh `ring` backend + reqwest `rustls` only (NASM blocks aws-lc-rs). rustls dep is intentional dep-tree pin — 0 direct `use` is correct.
- `~/.rift/*.json` compat — never change rename rules; keep `serde(flatten) extra`.
- `VelopackApp::build().run()` MUST be first call in `lib.rs::run()`.
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver).
- DriftWatcher conflict-rename guard — never overwrite dirty local.
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it.
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo.
- `path_guard.rs` API frozen — `edit/in_place.rs` + lib cmds depend.
- `rename_via` strict; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `last_scan_entries` is `std::sync::Mutex` (NOT tokio) — notify handler context.
- `force_pull_now`/`force_push_now` invariants preserved (v0.2.43).
- **NEVER `FileAttributes::default()` for SETSTAT** — sends zeros → truncation + epoch mtime. Use `empty()`.
- `SftpClient::delete` routes by remote stat — dirs go through `delete_recursive_via`.
- `mkdir_p_via` chmods each segment to 2775 for shared-group pushes.
- Upload pre-flight SHA-collapse before raising CONFLICT (v0.2.32).
- `DriftBucket::ToDelete` = local+no-remote+has-baseline → delete LOCAL. `DriftBucket::ToDeleteRemote` (v0.2.53) = local-missing+remote-has+has-baseline + mirror-on → delete REMOTE.
- Time displays MUST pass `[], { hour12: true }` — locale-default emits 24h on non-US.
- Mass local-delete circuit breaker: `(file_count * 0.30).clamp(5, 25)`. ToDeleteRemote bypasses (user reached via typed-MIRROR gate).
- `spawn_frontend_pump` 200/s rate-limit; critical stages bypass.
- russh `Config { keepalive_interval: 20s, keepalive_max: 3, window_size: 2 MiB, maximum_packet_size: 32 KiB }` in both `sftp::open_session` + `tunnel::start`.
- **v0.2.46+ data-integrity stack:** `mkdir_p_strict_via`, batch pre-mkdir in `flush_batch`, lock release on every `process_entry` terminal path, `path.is_file()` gate in `queue_path`, `wait_for_readable` 6×exp-backoff.
- **v0.2.48:** `ignored_directory_names()` excludes `build`+`dist` for FiveM `web/build/`+`web/dist/` ui_page bundles; Created+Dir 500ms debounce + `pending_dir_reconcile` AtomicBool coalesce.
- **v0.2.50:** `sftp/transfer.rs::with_t` op timeouts (T_QUICK 10s / T_NORMAL 30s / T_BODY 120s) + LIST_T 120s; `ConnectionWedged` diag emit on timeout; `process_entry` terminal lock-release INLINE await w/ 5s timeout (NOT `tokio::spawn`); `sync/ignore.rs` `.tmp.<pid>.<hex>` rule tight-matched (pid ≤8 digits, hash ≥8 hex, no 3rd dot-seg); `sync_sweep_stale_locks` ONLY clears own-user locks.
- **v0.2.52:** explicit `Modify(ModifyKind::Name(RenameMode::From))→Deleted` + `RenameMode::To→Created` arms; `consecutive_failed_batches` threshold 3 before Error escalation; 5s watched-root-vanish poll w/ de-dup HashSet for notify-rs #403.
- **v0.2.53:** Mirror mode is session-scoped (`mirror_mode: AtomicBool`); resets on engine restart by design — don't persist. UI typed-confirm requires literal "MIRROR". Auto-reconnect rolling-window = 3 wedges in 60s w/ `reconnecting` guard.
- **v0.2.56:** Assistant tab self-execs MCP server via `RIFT_MCP_SERVER=1` env branch in `lib.rs::run()` BEFORE Tauri loop; `RIFT_MCP_ROOTS` scopes filesystem access; CLI spawn passes `--mcp-config` + `--allowed-tools mcp__rift__*`; system addendum split into `_TOOLS` / `_NO_WS`. TabRail `--rail-w` CSS var drives AppShell grid. Files tab drag uses pointer events, NOT HTML5 DnD.
