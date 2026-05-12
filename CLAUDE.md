# Rift — Project Instructions

> Project-specific only. Global rules live in `~/.claude/CLAUDE.md` + `~/.claude/rules/`. Don't duplicate.

## Stack

Tauri 2 (Rust backend) + SvelteKit 2 + Svelte 5 (runes) + Tailwind 4. SSH/SFTP via `russh` (pure Rust, no libssh2/C deps). Velopack updater wired. NSIS installer (perUser).

Versions in lockstep: `package.json` + `src-tauri/Cargo.toml` (currently `0.2.4-alpha`).

## Canonical paths

| What | Where |
|---|---|
| Frontend | `src/` (routes/, lib/, app.css, app.html) |
| Backend | `src-tauri/src/` |
| Live state — read first each session | `docs/HANDOFF.md` |
| Versioned changelog | `docs/CHANGELOG.md` |
| Audit notes | `docs/audit/` |
| Design briefs | `docs/design/` |
| Dev launcher | `scripts/run-dev.bat` |

Skip in every agent scope: `node_modules/`, `.svelte-kit/`, `build/`, `src-tauri/target/`.

## Hot files (measured 2026-05-09)

Files large enough to matter for agent scoping. Everything else is small enough for inline or `recon`.

| File | Lines | Notes |
|---|---|---|
| `src-tauri/src/sync/auto_sync.rs` | 1208 | watcher → debounce → upload |
| `src-tauri/src/sftp/mod.rs` | 1100 | russh-sftp wrapper, connection pool |
| `src-tauri/src/lib.rs` | 811 | tauri command registry |
| `src-tauri/src/sync/drift_scanner.rs` | 397 | local↔remote diff |
| `src-tauri/src/state/sync_snapshot.rs` | 334 | snapshot persistence |

None exceed the 2000-line agent-split threshold yet, but `auto_sync.rs` and `sftp/mod.rs` are climbing. When either crosses 1500, scope agents by line range, not "audit X."

## Agent routing

| Area / task | Default | Why |
|---|---|---|
| `sync/`, `sftp/` multi-file changes | **operator** | Coupled state across watcher + queue + transport |
| `bootstrap/`, `profile/`, `state/`, `tunnel/`, `transport/`, `bridge/`, `edit/`, `local_fs.rs`, `update_service.rs` | inline | All <400 lines, single-file edits typical |
| Svelte/TS edits | inline + cclsp | LSP covers TS/JS — no agent needed |
| Rust symbol lookup | inline + cclsp | LSP covers Rust |
| "Where does X live" (LSP miss) | **recon** | Terse `path:line :: snippet` output |
| IPC contract change (bridge ↔ Svelte) | **architect** | Genuinely ambiguous tradeoffs |
| Tauri 2 / russh / Velopack docs lookup | **scout** if 3+ sources, else inline `WebFetch` | Batch threshold |
| Spot-check `operator`/`recon` claims before acting | **verifier** | NOT for verifying my own output (Opus 4.7 self-verifies) |

## Verification

- Frontend edits → `npm run check` (= `svelte-kit sync && svelte-check`)
- Rust edits → `cargo check --manifest-path src-tauri/Cargo.toml`
- `/check` skill auto-routes both

`operator` returning "done" on Rust changes MUST paste verbatim `cargo check` exit + zero `error[E\d+]` lines. Absent or summarized = UNVERIFIED — re-run before accepting.

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
