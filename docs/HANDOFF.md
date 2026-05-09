# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** WPF→Tauri migration of Rift. Sibling repo to `Blazzer10200/rift` (WPF v13.55.x line still ships from there). User's daily app = WPF Rift. This project = the v14.0.0 future.

**🚨 PATH SANITY CHECK — DO THIS FIRST:** Confirm `pwd` is `C:/AI Workflow/rift-tauri/`. The sibling WPF repo at `C:/AI Workflow/Rift Project/` is **stable port-from reference** — DO NOT refactor it unless the user explicitly says "WPF" or "v13.x". When user says "rift" or "the rift project", default to `rift-tauri`.

**🚨 FIRST THING NEXT SESSION — AWAIT CLAUDE DESIGN HANDOFF.** UI foundation (Tailwind v4 + shadcn-svelte) shipped in v0.1.5-alpha. User opened a "Rift App UI" project in Anthropic Labs' Claude Design (claude.ai/design) using `docs/design/CLAUDE-DESIGN-BRIEF.md` as the foundational context, took screenshots of the running dev build, and is iterating on direction prompts there. **Once they paste/drop the exported deliverable bundle here, translate the OKLCH tokens into `src/app.css` and apply per-component visual decisions across the existing dialogs + shell.** Suggested order: (1) Confirm dialog as proof-of-concept → (2) remaining 5 dialogs → (3) AppShell/TopBar/ServerPicker → (4) Browser two-pane.

**Current state (post Session 11, v0.1.6-alpha):** Migration core complete + backend audit sweep landed (85 findings cataloged, critical/high/medium fixed). UI foundation + Claude Design brief in place from v0.1.5. Phase 6 (v14.0.0 ship) still deferred per user.

**Release policy:** No public ship until user explicitly says go. Iterate via `npm run tauri dev`.

## Verified phase status

| Phase | Scope | Status |
|---|---|---|
| 0 + 1a–1h, 1j | Backend (state, sftp, sync engine, tunnel, tail services) | ✅ |
| 1i | ConfigStore write-back + TOFU fingerprint persist | ✅ |
| 2 | UI shell — MainWindow port (Svelte 5 runes) | ✅ |
| 3 | Two-pane browser | ✅ |
| 4 | Sync surfaces (activity, drift, conflicts, locks, edit-in-place) | ✅ |
| 5 | Dialogs — AddServer / Bootstrap / Keygen / Reupload / Confirm / CommandPalette | ✅ |
| **5.5** | **UI redesign foundation — Tailwind v4 + shadcn-svelte + Claude Design brief** | ✅ Session 10 |
| **5.6** | **Backend audit sweep — 85 findings cataloged, critical/high/medium fixed** | ✅ Session 11 |
| 6 | v14.0.0 ship | ⏳ deferred |

## Session 11 — 2026-05-09 — Backend audit sweep (v0.1.6-alpha)

While user iterated on Claude Design, ran a comprehensive backend audit across all `src-tauri/src/` (23 files, 6204 LOC). Audit agent produced 85 findings into `docs/audit/BACKEND-AUDIT-2026-05-09.md` — full Critical/High/Medium/Low breakdown. Critical/High/Medium tier landed this session.

**Top fixes:**
- Mutex-poison cascade hardened across 3 state caches (`SyncSnapshot`, `RemoteStateCache`, `ResourceDiscoveryCache`).
- `SaveLocalCopy` data-loss path closed — `let _ = std::fs::rename(...)` now bails before download on rename failure.
- `RiftConfig::save()` made atomic (tmp+rename) — `~/.rift/rift.json` survives mid-write crashes.
- Fire-and-forget JoinHandle tracking — bridge ping / lock acquire/release / edit-trail append now abort on `engine.stop()`.
- `stat_local` returns `Option` so metadata errors no longer poison snapshot baselines with `(0, now())`.
- 3-way SHA1_MAX_BYTES split (5MB/64MB/64MB) collapsed to single canonical 64MB const in `state::sync_snapshot`.
- Shared transport modules added: `transport::ssh_handler::PinningHandler` (de-dups `Handler` between sftp + tunnel), `transport::env::{current_user, hostname, short_id}` (replaces 3 duplicate impls + nanosecond-temp-dir collisions).
- Tokio features narrowed `["full"]` → 6 specifics. Dead deps dropped: `anyhow`, `thiserror`, `notify-debouncer-full`, `async-trait`.
- `spawn_blocking` for the 3 walkdir/read_dir hotspots that ran sync I/O on the tokio runtime.
- Per-file extras: `editor_for` TOCTOU lock, `get_remote_sha1` stderr capture + debug-log, `mark_failed` permanent-drop log warn, `ensure_workers` FuturesUnordered short-circuit, key-load-once via Arc, `hex_upper` write!, `ignore` needle pre-lower, `edit_trail::trim_to_tail` newline preservation.
- ~12 inline doc additions for non-obvious invariants (advisory lock window, plaintext bridge token, autosync latency tradeoff, etc.).

**Frontend untouched** per the "don't refactor UI mid-redesign" rule. `local_list_dir` Tauri command kept its legacy `{path, mtime: secs}` shape via an adapter; canonical walker is now `local_fs::list_directory`.

**Verified:**
- `cargo clippy --lib --tests -- -D warnings` ✓ zero warnings
- `cargo test --lib` ✓ 47/47 pass (+ 1 new `segments_are_lowercase` invariant test = 48 total)
- `npm run check` ✓ 318 files / 0 / 0

**Bumped** 0.1.5 → 0.1.6 across Cargo.toml / package.json / tauri.conf.json. v0.1.5 entry archived.

**Audit doc retained** at `docs/audit/BACKEND-AUDIT-2026-05-09.md` for the LOW-tier nitpicks that were deliberately not addressed this pass (cosmetic naming, `hostname()` subprocess vs env-var, etc.).

## Session 10 — 2026-05-09 — Backlog commit + UI foundation + Claude Design brief

### Backlog commit (the FIRST THING from Session 9 HANDOFF)
Sessions 2–9 bundled into `5b9f5f7 Phases 1-5 + 1i — migration core complete (v0.1.4-alpha)` on top of bare Phase 0 scaffold. `build/` (SvelteKit static output) added to `.gitignore`.

### UI redesign foundation
After web research (cross-referenced r/sveltejs Oct 2025–Apr 2026, dev.to, svelte.dev blogs), settled on **shadcn-svelte (nova style, zinc base) + Tailwind v4 + Bits UI primitives + Svelte 5 native transitions**, with motion-sv held in reserve for layout animations. Skipped Skeleton/Flowbite per community sentiment.

Files touched:
- `vite.config.js` — `@tailwindcss/vite` plugin
- `src/app.css` — full OKLCH zinc theme (light + dark) + `@theme inline` mappings
- `src/app.html` — `class="dark"` for first-paint
- `src/routes/+layout.svelte` — `<ModeWatcher defaultMode="dark" />`
- `src/lib/utils.ts` (NEW) — `cn` + `WithElementRef`/`WithoutChild` types
- `components.json` (NEW) — shadcn-svelte CLI config
- `src/lib/components/ui/button/` (NEW, smoke-test only) — proves CLI pipeline works

### Claude Design brief
`docs/design/CLAUDE-DESIGN-BRIEF.md` — single-shot foundational context for claude.ai/design. Avoids the token-burn of "Attach codebase" by pre-digesting product summary, tech constraints, component inventory, current OKLCH tokens, non-goals, and 4 direction prompts (Linear / Raycast / Sublime / Win11-Mica). User created a "Rift App UI" project there using the brief as the Design System input, screenshots as the visual reference.

### Verified
`npm run check` 318 files 0/0 ✓ · `npm run tauri dev` boots clean ✓ · cargo left untouched.

### Bumped
0.1.4 → 0.1.5 across Cargo.toml / package.json / tauri.conf.json. v0.1.4 entry archived.

## CRITICAL DON'T-TOUCH (carries forward)
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only — never enable `default-tls`/`native-tls` (drags openssl)
- npm runner, NOT pnpm
- File-format compat w/ WPF `~/.rift/*.json` — never change PascalCase/camelCase rename rules; never drop the `serde(flatten) extra` bag on `RiftConfig`
- Velopack `VelopackApp::build().run()` first call in `run()`
- shadcn-svelte components own themselves in `src/lib/components/ui/` — don't refactor into a shared abstraction; that defeats the code-ownership model
