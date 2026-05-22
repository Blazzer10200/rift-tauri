# Rift — Project Instructions

> Project-specific only. Global rules live in `~/.claude/CLAUDE.md` + `~/.claude/rules/`. Don't duplicate.

## Stack

Tauri 2 (Rust backend) + SvelteKit 2 + Svelte 5 (runes) + Tailwind 4. SSH/SFTP via `russh` (pure Rust, no libssh2/C deps). Velopack updater wired. NSIS installer (perUser).

Versions in lockstep across THREE files: `package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json`. Source of truth for current version: `docs/CHANGELOG.md` (don't hard-code in this file — went stale at v0.2.4 vs v0.2.48 reality before 2026-05-13 cleanup).

## Canonical paths

| What | Where |
|---|---|
| Frontend | `src/` (routes/, lib/, app.css, app.html) |
| Backend | `src-tauri/src/` |
| Live state — read first each session | `docs/HANDOFF.md` |
| Versioned changelog | `docs/CHANGELOG.md` |
| Live issue tracker | `docs/ISSUES.md` (single source — open AUDIT findings folded in 2026-05-19) |
| Design briefs | `docs/design/` (3 active: `assistant-compaction.md`, `git-rcon-tools.md`, `ui-audit-2026-05-21.md`) |
| Dev launcher | `scripts/run-dev.bat` |

Skip in every agent scope: `node_modules/`, `.svelte-kit/`, `build/`, `src-tauri/target/`.

## Hot files (measured 2026-05-17)

Files large enough to matter for agent scoping. Everything else is small enough for inline or `recon`.

| File | Lines | Notes |
|---|---|---|
| `src-tauri/src/sync/auto_sync.rs` | 1966 | engine orchestrator; FSW + dirty queue + drift reconcile + force_push/pull |
| `src-tauri/src/lib.rs` | 1790 | tauri command registry (51 cmds) |
| `src-tauri/src/assistant/mod.rs` | 1167 | claude CLI integration + auth + workspace |
| `src-tauri/src/sync/auto_sync/flush.rs` | 653 | flush_batch pipeline (split out 2026-05-13) |
| `src-tauri/src/assistant/mcp_server.rs` | 587 | stdio JSON-RPC MCP server |
| `src-tauri/src/sync/drift_scanner.rs` | 555 | 3-way drift diff |
| `src-tauri/src/sftp/list.rs` | 454 | exec-fast-path + worker fallback (split v0.2.49) |
| `src-tauri/src/diagnostics/mod.rs` | 446 | DiagBus + LogForwarder + frontend pump |
| `src-tauri/src/sync/ignore.rs` | 441 | full WPF ignore-rule parity + tests |
| `src-tauri/src/sync/lock_presence.rs` | 386 | .rift-lock advisory presence |
| `src-tauri/src/sync/drift_watcher.rs` | 380 | pull_one / delete_local_one / register_conflict (auto-poll loop removed v0.2.38) |
| `src-tauri/src/state/sync_snapshot.rs` | 375 | snapshot persistence |
| `src-tauri/src/sftp/transfer.rs` | 368 | atomic upload/download + `with_t` op timeouts |
| `src-tauri/src/sync/auto_sync/watch.rs` | 349 | notify lifecycle + queue_path |
| `src-tauri/src/sftp/mod.rs` | 307 | session core (split v0.2.49 from 1100L → 307L) |

Frontend hot files: `assistant.svelte.ts` 1585L, `Settings.svelte` 1505L, `SyncPage.svelte` 1307L, `ActivityFeed.svelte` 941L, `TerminalPanel.svelte` 851L.

`auto_sync.rs` + `lib.rs` are approaching the 2000-line agent-split threshold — `lib.rs` split into per-domain `commands/*.rs` is queue item (e).

## Agent routing

Only `operator` + `recon` are defined as local subagents. `architect` / `scout` / `verifier` were archived 2026-05-02 — use built-ins of the same name if needed, else inline.

| Area / task | Default | Why |
|---|---|---|
| `sync/`, `sftp/` multi-file changes | **operator** | Coupled state across watcher + queue + transport |
| `bootstrap/`, `profile/`, `state/`, `tunnel/`, `transport/`, `bridge/`, `edit/`, `local_fs.rs`, `update_service.rs` | inline | All <400 lines, single-file edits typical |
| Svelte/TS / Rust symbol lookup | inline + LSP tool | TS/JS native; Rust via `rust-analyzer-lsp` plugin (installed 2026-05-21). Svelte: no LSP — grep + `/check` (skipped Piebald `svelte-lsp` — needs tweakcc patch) |
| "Where does X live" (LSP miss) | **recon** | Terse `path:line :: snippet` output |
| IPC contract change, tradeoff calls | inline (or `Plan` skill for multi-track) | Built-in `architect` agent is available but heavyweight; usually inline reasoning is enough |
| Tauri 2 / russh / Velopack docs lookup | `blazzer-search` MCP or inline `WebFetch` | Per `rules/tools.md` |

## Verification

- Frontend edits → `npm run check` (= `svelte-kit sync && svelte-check`)
- Rust edits → `cargo check --manifest-path src-tauri/Cargo.toml`
- `/check` skill auto-routes both

`operator` returning "done" on Rust changes MUST paste verbatim `cargo check` exit + zero `error[E\d+]` lines. Absent or summarized = UNVERIFIED — re-run before accepting.

## Live UI verification — CDP (2026-05-16, S69)

Claude can drive + observe the running Rift UI autonomously. No manual screenshots needed.

1. `scripts/run-dev.bat` already sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`. Always start dev via this batch (not raw `npm run tauri dev`) so CDP is exposed.
2. Start the wrapper once per dev session: `npm run cdp:serve` (background). It holds one persistent ws to WebView2 and exposes `127.0.0.1:9223`. ~40-60ms per call.
3. Drive via `bash scripts/cdp/c.sh {health|state|page|eval|type|click|wait|shot|shot-sel|batch|key|shutdown}`. Full docs: `scripts/cdp/README.md`.
4. **`shot` workflow** — `bash scripts/cdp/c.sh shot` prints just the path on stdout (use `--json` for `{path,bytes}`). `f=$(bash scripts/cdp/c.sh shot)` → `Read` $f; image renders inline (multimodal). `shot-sel "<selector>"` clips to a bounding rect. Server auto-prunes `.tmp/snap-*` to last 20 on boot. ~$0.07/shot at Opus 4.7 rates.
5. **DOM vs shot decision table — DON'T skip the shot when these triggers fire:**

   | Trigger | Tool | Why |
   |---|---|---|
   | "what is mounted / how many bubbles / current model" | `state` | DOM probe is free + sufficient |
   | "did element X mount / what's its className / offsetHeight" | `eval` | Same — DOM has the answer |
   | **User pastes a screenshot of the UI** | **`shot` immediately** | Compare your current state to what they see before reasoning |
   | **CSS edit (animation, spacing, color, layout, size)** | **`shot` after HMR** | DOM `getComputedStyle` proves the property changed; only pixels prove it LOOKS right |
   | **"look at the chat / see how it looks / examine the UI"** | **`shot` first, then probe** | The user is asking about pixels, not structure |
   | **Before claiming a UI change "done"** | **`shot` once** | DOM-only verification on visual work = unverified |
   | "Is component overflowing / wrapping / colliding" | **`shot`** | Layout bugs are pixel-level; DOM lies about visual collision |

   Rule of thumb: if the answer is "looks right" / "looks wrong", you need pixels. If the answer is "yes mounted" / "value is 360ms", DOM is fine.
6. Webview only — titlebar / drag region / OS dialogs are invisible to CDP. If a native-chrome bug appears, ask the user OR queue a Windows-MCP setup.
7. CDP is dev-only. Prod builds don't expose the port.

## Project gotchas

1. **Tauri 2 drag-region** — `data-tauri-drag-region` is a silent no-op unless the capability grants `core:window:allow-start-dragging`. `core:default` does NOT include it. Check `src-tauri/capabilities/` before debugging drag bugs. (memory: `reference_tauri2_drag_region`)
2. **Self-replace dance** — `tauri build` fails (Win file-lock) if Rift is running. Quit instance → sleep 1 → build to temp → `cp` over → relaunch. (memory: `reference_self_replace_dance`)
3. **Doc size cap** — `CHANGELOG.md` + `HANDOFF.md` ≤600 words each. Truncate older entries when extending — `git log` preserves history. (Raised 300 → 600 on 2026-05-12 after the v0.2.46 → v0.2.48 arc; 300 was too tight for an arc summary + RESUME HERE + CRITICAL DON'T-TOUCH inline.)
4. **Build = batch only** — dev server (`npm run dev` or `scripts/run-dev.bat`) is the default loop. The full "build" pipeline (bump → changelog → check → build → install → shortcut → iconcache → commit) only runs when a batch is ready to ship. Never trigger mid-session.
5. **Version lockstep — THREE files** — `package.json`, `Cargo.toml`, AND `src-tauri/tauri.conf.json` must all match. `scripts/release.ps1` preflight bails on any mismatch. Bumping only two is the most common failure mode (2026-05-12: v0.2.49 first ship attempt died here). All three, always.

## Don't-do

- Don't auto-bump versions or edit `CHANGELOG.md` mid-session — `/git-ship` territory.
- Don't run `npm run build` or `npm run tauri build` unless told.
- Don't use Playwright MCP (global ban — user verifies UI in browser).
- Don't `EnterPlanMode` — use the `/plan` skill instead.
- **Don't run `cargo check` while `npm run tauri dev` is alive.** Tauri dev watches Rust files and triggers incremental rebuilds; an external `cargo check` collides with its lock state and kills the running Rift Dev process w/o re-launching. Run `/check` before launching dev, or quit dev first. (S16 2026-05-09)

## Live state pointer

Before assuming current state, read `docs/HANDOFF.md`. Branches in flight, in-progress work, recent decisions — that's where they live, not here.
