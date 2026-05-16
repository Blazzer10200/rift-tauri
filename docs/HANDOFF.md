# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 70 — 2026-05-16 — CDP autonomous-verify infra + UX polish

**Committed mid-session (pre-compact checkpoint), not shipped/version-bumped.** Bundles into v0.2.57-alpha alongside S69.

**CDP infra (`scripts/cdp/`)** — Claude can drive + observe the running Rift UI autonomously. WebView2 CDP enabled in `scripts/run-dev.bat` via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`. Persistent Node wrapper (`serve.cjs`) on `127.0.0.1:9223` holds one ws to WebView2 → ~40-60ms/call vs ~700ms PS cold-start. Endpoints: `/health /eval /type /click /wait /screenshot /key /state /shutdown`. Bash helper: `bash scripts/cdp/c.sh <cmd>`. `npm run cdp:serve`. `/key` uses CDP `Input.dispatchKeyEvent` (OS-trusted) — synthetic JS KeyboardEvents won't close palette by design. Workflow in `CLAUDE.md` + `scripts/cdp/README.md`. **Dev-only**, prod doesn't expose port.

**S69 live-verified end-to-end via CDP** — Sonnet 4.6 emits thinking blocks under `MAX_THINKING_TOKENS=10000` ("Reasoned 5.5s", click-to-expand `.reasoning-body` w/ real text, brain icon, `aria-expanded` toggles). Bundled-`.exe` resolver fired cleanly (no `.cmd` shim corruption). Open question from S69 closed.

**UX polish batch (4 fixes)** —
- `ActivityFeed.svelte:459` copy unified ("drift scan" → "rescan")
- `TabRail.svelte:61` rail-pin `aria-label` added alongside `title`
- `Settings.svelte` Assistant section: API-key field gained Eye/EyeOff show/hide toggle w/ `aria-label` flip
- `Settings.svelte:113` Appearance hidden from nav (dormant render branch kept for fast re-enable); `initialSection` default → `"terminal"`. Queue item (j) resolved by hiding.

**Tour observations not yet acted on** — Sync page stat cards (item 5 from tour), Conflicts default-education (item 6), titlebar hostname leak toggle (item 11), `Sync Inspector` (Ctrl+Shift+D) surfacing.

## Session 69 — 2026-05-15 — Assistant blank-response fix + thinking surface

**Status: committed S70 checkpoint, not shipped.** Bundle into v0.2.57-alpha.

**Root cause fix** — Rift's CLI resolver fell through to `claude.cmd`; cmd.exe `%*` mangled `--output-format stream-json` → CLI silently downgraded to plain text → parser dropped everything. Fix: probe bundled `%APPDATA%\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe` directly between LOCALAPPDATA + .cmd fallback (`mod.rs:resolve_claude_exe` step 2b).

**Defense-in-depth (`assistant.svelte.ts:onStream`):** non-JSON lines render as assistant text; envelope text-blocks buffered as fallback when zero deltas fire; per-turn delta count + ring-logged raw NDJSON; blank-turn banner fingerprints envelope types.

**Thinking surface** — new `ThinkingBlock` type in Block union; parser handles `content_block_start/_delta/_stop` for `thinking`, `thinking_delta`, `signature_delta`. Backend sets `MAX_THINKING_TOKENS=10000` env for non-haiku. UI: animated "🧠 Thinking" pill w/ live elapsed timer (250ms tick), transitions to "Reasoned · 5.3s" w/ collapsed first-line preview; click-to-expand panel; encrypted-only blocks show "· encrypted" suffix.

**Don't retry:** `--settings '{"thinking":{...}}'` / `--settings '{"maxThinkingTokens":N}'` / bare `MAX_THINKING_TOKENS` env alone — none triggered thinking emission. ONLY `MAX_THINKING_TOKENS=10000` env + opus model reliably fires blocks. `--permission-mode plan` triggers but restricts edits.

**Next:** live-test against sonnet (does it emit `thinking` under `MAX_THINKING_TOKENS` or only opus?). Bundle into v0.2.57-alpha w/ queue items. Pre-expand first thinking block in fresh convos for discoverability?

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
