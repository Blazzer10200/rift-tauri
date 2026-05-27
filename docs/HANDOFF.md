# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## v0.4.33 work — COMMITTED to `updater-migration` (2026-05-27), NOT released

Big verified batch, gated on two-machine confirm (see Resume). Full detail → CHANGELOG.

- **Permission modes (Piece 1+2).** All 5 modes functional via `--permission-prompt-tool stdio` + control channel (`PermissionRegistry` in permission.rs, `assistant_answer_permission`, `PermissionBar.svelte`; `--allowed-tools` mode-aware). **All 3 prompting modes VERIFIED 2026-05-27** (stdio-control probe, `%TEMP%/rift-perm-probe.cjs`): default gates Write+Bash; acceptEdits auto-allows edits + safe Bash, **gates risky Bash** (`curl`); plan redirects mutations to `.claude/plans/` + gates ExitPlanMode. Correction to earlier note: acceptEdits does NOT prompt for *all* Bash — only risky. Zero backend bugs.
- **Enhancer streaming + actionable retune.** Streams token-by-token now — fix: text input (`-p`, null stdin) + `--output-format stream-json`; `--input-format stream-json` block-buffers the bundled exe's stdout (documented in code). Meta-prompt retuned to "add actionable detail" (no more no-op on clear drafts). CDP-verified streaming. ~6-8s TTFT = CLI cold-start, inherent.
- **Ctx-pill fix.** Pill off last `assistant` envelope, not cumulative `result`. +1 test (39 pass). **Not yet live-verified vs a real long task — queue CDP multi-step run before ship.**
- **THIS MACHINE manually patched (NOT a release):** `…\Rift\current\rift-tauri.exe` runs 0.4.33 code, reports v0.4.32. Rollback: restore `rift-tauri.exe.bak-pre-enhancer`. Other machine = genuine 0.4.32.

---

## Branch in flight — `updater-migration` (Velopack → tauri-plugin-updater)

**v0.4.32-alpha SHIPPED 2026-05-26** to `Blazzer10200/rift-releases` (bridge already ran — all assets live, `latest.json` polled 108×). Branch version files still read 0.4.32-alpha. Brief: [docs/design/updater-migration.md](design/updater-migration.md). Signing key `C:/Users/BLAZZER/.tauri/rift.key` — **backed up 2026-05-27 to OneDrive + iCloud** (`rift-signing-key-backup/`). `release.ps1` = Tauri-only path for v0.4.33+. `release-bridge.ps1` = the one-time v0.4.32 bridge, now spent (retire it).

Audit 2026-05-27 RESOLVED (prior session) — [docs/audit-2026-05-27.md](audit-2026-05-27.md). Open queue: 10 issues (#4 #7 #14 #15 #17 #20-M6-M9 #21 #29 #89 #265) → ISSUES.md.

---

## Ship v0.4.33 (all feature work committed, NOT released)
1. Key backup DONE (OneDrive + iCloud `rift-signing-key-backup/`); verify cloud copies synced. Feature commits DONE on `origin/updater-migration` (`4d669bf` + this session's v0.4.33 batch).
2. **THE GATE — confirm BOTH machines on v0.4.32 before shipping v0.4.33.** v0.4.33 ships via Tauri-only `release.ps1` with NO Velopack assets; a machine still on v0.4.31 would be permanently stranded. Setup.exe downloads on the live 0.4.32 release = 0, so the Tauri install path may not have run on either — verify both first.
3. **Ship v0.4.33-alpha** (after gate): `pwsh scripts/bump.ps1 0.4.33-alpha` (3 version files) → set date on v0.4.33 CHANGELOG entry → quit dev (frees `C:\cargo-targets`) → `pwsh scripts/release.ps1` (NOT release-bridge). Then retire `release-bridge.ps1`.
4. Optional pre-ship: live-verify the ctx-pill fix vs a real long task (CDP multi-step).

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. Latest public release = **v0.4.32-alpha** (shipped 2026-05-26, bridge). Next ship = v0.4.33 on `updater-migration` — gated on two-machine confirm (see Resume). v0.4.33 work (permission modes Piece 1+2, ctx-pill fix, enhancer streaming+actionable) all COMMITTED, tree clean. Tauri 2 + Svelte 5 + Rust + russh.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- `C:/Users/BLAZZER/.tauri/rift.key` — Tauri-updater signing key. Lose it and no v0.4.32+ install can update. Pubkey in `tauri.conf.json::plugins.updater.pubkey`; do NOT regenerate.
- russh `ring` + reqwest `rustls`. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `bundle.targets:["nsis"]`. `createUpdaterArtifacts: true`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations`. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: always `--input-format stream-json` + `--permission-prompt-tool stdio` + initialize handshake + stdin kept open for turn. `--allowed-tools` is mode-aware: bypass/auto = full BUILTINS; prompting modes = SAFE_BUILTINS only.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` w/ attachments. 20MiB cap.
- Settings is workspace (kbd **5** post-v0.4.30 rail trim), `Ctrl+,` flips; no slideover scrim.
- `tauri.conf.json` `dragDropEnabled: false` — required for HTML5 DnD.
- AssistantPane drop handlers on `.pane` outer only — inner overlays break preventDefault chain.
- `compactionHistory[]` is camelCase in persisted JSON. Don't rename.
- `.shell` MUST be `position: fixed; inset: 0` (AppShell). `body.win-maximized .shell { inset: 8px }` for borderless-maximized.
