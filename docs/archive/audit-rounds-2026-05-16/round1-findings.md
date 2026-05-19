# Round 1 — Inventory + version lockstep + docs surface

Date: 2026-05-16. Auditor: Claude (Opus 4.7).

## Pre-flight environment

- Tauri dev / cargo: NOT running (verified `tasklist`). `cargo check` safe to run.
- Git working tree: 5 modified files (S69 unshipped Assistant work — `assistant/mod.rs`, `MessageBubble.svelte`, `assistant.svelte.ts`, `Cargo.lock`, `HANDOFF.md`).
- Last shipped: `687edb8 v0.2.56-alpha`. Last commit: `229929e docs: HANDOFF trim`.
- Snapshot: `backups/snapshots/2026-05-16/{audit-baseline.tar.gz, memory-rift-baseline.tar.gz}` (98K + 8.6K).

## Surface 1 — Version lockstep ✅

| File | Version |
|---|---|
| `package.json` | 0.2.56-alpha |
| `src-tauri/Cargo.toml` | 0.2.56-alpha |
| `src-tauri/tauri.conf.json` | 0.2.56-alpha |
| `docs/CHANGELOG.md` (top entry) | v0.2.56-alpha |

All four aligned. `scripts/release.ps1` preflight (lines 27-37) enforces. **No drift.**

## Surface 2 — Docs

| File | Words | Cap | Status |
|---|---|---|---|
| `docs/HANDOFF.md` | 1330 | 600 | **OVER** by 730 |
| `docs/CHANGELOG.md` | 2501 | 600 | **OVER** by 1901 (currently shows full v0.2.56 + v0.2.55 entries) |
| `docs/AUDIT.md` | 1795 | (no cap) | OK |
| `docs/CONTRIBUTING.md` | 269 | (no cap) | OK |
| `docs/ONBOARDING.md` | 290 | (no cap) | **STALE** — see below |
| `docs/design/assistant-page.md` | 2846 | (no cap) | **STALE** — design brief for shipped feature |

### Doc drift details

- **ONBOARDING.md drift** — refs "Drift" tab + "AutoSync" tab + "Pull all" / "Push all" buttons. Renamed v0.2.55: tab is "Sync" w/ single one-button Sync. Add-server flow is "Sidebar → ＋ Add server" but TabRail moved that affordance to PageHeader/extras snippet (S67). User-facing instructions don't match current UI.
- **CONTRIBUTING.md drift** — line 52 says `release.ps1 drives bump → build → vpk pack → vpk upload` but the script (verified) does NOT bump versions; it reads them and bails on mismatch. Bumping is manual or via `/git-ship`. Same drift comment on line 48.
- **assistant-page.md** — labeled "Status: planning only (session 60)" but feature shipped in v0.2.56. Historical brief, candidate for `docs/archive/`.
- **AUDIT.md** — comprehensive, but ALL line numbers in "Open Findings" are pre-2026-05-11 tree. Doc explicitly warns: "re-verify against current HEAD before fixing." Findings still apply; line refs are stale (verified samples below).
- **rift.json.example** + **AUTHORIZED_KEYS.md** — not audit-touched (last modified May 9, references match current schema based on profile/mod.rs read).

### AUDIT.md line-anchor re-verify (samples)

| AUDIT.md anchor | Current line | Status |
|---|---|---|
| `lib.rs:diag_state_pump:176-222` | line 313 | exists, line moved |
| `lib.rs:editor_for:1034-1057` | line 1576 | exists, line moved |
| `RemotePane.svelte:44-47 async load` | line 59 | exists, line moved |
| `LocalPane.svelte:44-47 async load` | line 59 | exists, line moved |
| `AppShell.svelte:95 addEventListener` | line 184 | exists, line moved |

All sampled findings still valid. Line references stale across the board.

## Surface 3 — Hot file table in CLAUDE.md is severely stale

| File | CLAUDE.md says | Actual lines | Drift |
|---|---|---|---|
| `src-tauri/src/sync/auto_sync.rs` | 1208 | **1954** | grew +746 |
| `src-tauri/src/sftp/mod.rs` | 1100 | **302** | shrank -798 (split landed v0.2.49 — `list/ops/transfer/remote_exec`) |
| `src-tauri/src/lib.rs` | 811 | **1771** | grew +960 (52 cmds, 22 pub items) |
| `src-tauri/src/sync/drift_scanner.rs` | 397 | **555** | grew +158 |
| `src-tauri/src/state/sync_snapshot.rs` | 334 | **364** | ~stable |

### New hot files NOT in CLAUDE.md table (>400 lines)
- `src-tauri/src/assistant/mod.rs` — **775 L** (Assistant tab CLI integration)
- `src-tauri/src/sync/auto_sync/flush.rs` — **610 L** (split out 2026-05-13)
- `src-tauri/src/sftp/list.rs` — **454 L** (split out v0.2.49)
- `src-tauri/src/assistant/mcp_server.rs` — **447 L**
- `src-tauri/src/sync/ignore.rs` — **441 L**

### Frontend hot files (no entry in CLAUDE.md)
- `src/lib/components/sync/SyncPage.svelte` — 1223 L
- `src/lib/components/settings/Settings.svelte` — 1060 L
- `src/lib/state/assistant.svelte.ts` — 1012 L (DIRTY S69)
- `src/lib/components/activity/ActivityFeed.svelte` — 951 L
- `src/lib/components/terminal/TerminalPanel.svelte` — 818 L

**Recommendation (FLAG, not fix):** CLAUDE.md hot-file table needs full refresh. Don't touch — user's project file, defer to user-side update. Will note in REPORT-CARD.

## Surface 4 — Configs / capabilities / scripts

- `src-tauri/capabilities/default.json` — clean. Includes `core:window:allow-start-dragging` (gotcha #1 satisfied). AUDIT.md still flags `core:default` + `opener:default` as broad — both still present, low priority.
- `src-tauri/tauri.conf.json` — clean. `bundle.targets: ["nsis"]` matches alpha policy.
- `scripts/release.ps1` — clean. Comment block accurate. Preflight version sync enforced.
- `scripts/run-dev.bat` — 12-line wrapper around `npm run tauri dev`. Clean.
- `vite.config.js` / `svelte.config.js` / `vitest.config.ts` / `tsconfig.json` — small, will read in Round 3.

## Setup tally

- 30 backend `.rs` files. 243 pub items (top-level). lib.rs alone has 22 pub items (most are tauri commands).
- Backend total: 10,806 lines.
- Frontend top-5 hot: 5,064 lines.
- Snapshot dir + state dir + docs/archive dir created.

## Next: Round 2

Backend hot-file dead-code sweep + Cargo deps usage check (operator A in flight).
Frontend dead-component findings already in (frontend recon agent done):
- ✅ confirmed orphan: `assistant/ToolCallCard.svelte`
- ⚠️ likely orphan: `assistant/EmptyState.svelte` (verify path-disambiguation)
- ⚠️ unused-but-shipped: `shell/PageFooter.svelte` (CHANGELOG mentions, no consumers yet)
