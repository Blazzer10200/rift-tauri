# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.6.3 — 2026-06-06 — chore: hotfix release to live-verify in-app auto-update

No code changes — a version-only bump shipped to confirm the v0.6.2 apply fix works end-to-end from inside the app (download → swap → relaunch on the new build). This is the live-verify owed in v0.6.2's notes. If you can read this from an app that updated itself without a manual reinstall, the fix holds.

## v0.6.2 — 2026-06-06 — fix: in-app update could never apply (child-process file lock)

> **Why.** Clicking **Update** in the app downloaded fine but the new version never took — the app would relaunch on the *same* build. Auto-update was effectively dead from inside the app, even though the Velopack machinery itself was sound.

**Root cause.** Velopack applies an update by renaming the whole `current/` directory, and its `Update.exe` waits only for the *main* process to exit before doing so. But each turn the Claude CLI spawns `rift-tauri.exe` as its MCP server (`RIFT_MCP_SERVER=1`), and *any* live `rift-tauri.exe` holds an exclusive lock on `current/` (a rename fails with a sharing violation). `app.exit(0)` is `std::process::exit`, which skips `Drop`, so `kill_on_drop` never reaped those children — they orphaned and kept the directory locked, so the swap silently no-op'd and the relaunch brought back the old build.

**Fix.** On the apply path, reap the lockers before exiting: tree-kill every tracked `claude` child first (so none can respawn an MCP child in the exit window), then sweep any stray `rift-tauri.exe` MCP child orphaned from an earlier turn. Now `current/` is unlocked the instant the main PID dies and Velopack's swap + relaunch land on the new version. New `kill_all_session_children()` helper. Files: `update_service.rs`, `assistant/mod.rs`.

> **One-time note.** Clients on v0.6.1 or earlier still run the old apply path, so updating *to* this build may not stick from inside the app — download **Rift-win-Setup.exe** from the release and run it once. Every in-app update from v0.6.2 onward applies cleanly.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4062 files) · root cause proven directly: a live `rift-tauri.exe` blocks renaming `current/`, and the standalone Velopack download+apply path was confirmed working end-to-end.

## v0.6.1 — 2026-06-06 — feat: CLI multi-install awareness + unified update UI

> **Why.** A box with both an npm-global and a native `claude` install (they drift to different versions) could show "out of date" while **Update** did nothing — Rift only ever resolved and bumped *one* install. Now Rift sees every install, runs the newest, and updates them all. The surfaces that report this were also unified so they can no longer drift apart.

**CLI multi-install detection + update-all.** `enumerate_claude_installs()` finds every `claude` on the machine — PATH entries, native install sites, the npm-bundled exe, and `.cmd` shims (deduped shim→exe) — and probes each `--version`. The newest wins and is the binary Rift spawns; **Update now** updates them all (npm once, native per-binary) so versions can't skew. If a copy is still behind afterward (a native no-op that reports success without bumping), a "still behind" hint points at a manual reinstall. A new `ClaudeInstall` DTO rides the auth probe, and **Settings → Assistant** lists each install with active / behind tags and a per-row, method-aware copy button (npm users are never handed a `claude update` command, or vice-versa). Files: `assistant/mod.rs`, `cliUpdate.svelte.ts`, `assistant/types.ts`.

**Unified update UI.** The CLI-update notice was hand-authored three times (Home banner, tab-bar popover, Settings row) and drifted; the contextual line — npm / native / multi-install / stuck / error — now comes from one `summary()` source, and the Home banner + tab-bar popover share a tone-aware treatment (accent / warn / danger). Separately, the Velopack **app**-update dialog's status tints were fixed from `oklch` to `oklab` color-mixing, so a non-default accent hue no longer wraps warm status tones toward purple. Files: `UpdateDialog.svelte`, `HomePage.svelte`, `ChatTabsBar.svelte`, `SettingsPage.svelte`.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4062 files) · every update surface — Home banner, tab-bar popover, Settings row, and the Velopack dialog across all states + tones — CDP-verified live.

## v0.6.0 — 2026-06-06 — feat: in-app browser dock + harness / model-picker polish

In-app browser dock (Ctrl+Shift+B) Rift can *see* + hand to the assistant; harness one-viewport redesign; model-picker capability matrix; onboarding beta-notice step; fixes #31–#36. Detail in `git log -- docs/CHANGELOG.md`.

## v0.5.0 — 2026-06-04 — feat: Harness telemetry workspace + Steer (mid-turn redirect)

Harness telemetry workspace (Ctrl+3) + mid-turn Steer (Alt+Enter injects into the live CLI's stdin so a turn course-corrects without a restart). Detail in `git log -- docs/CHANGELOG.md`.

_Older entries (v0.4.48 and earlier) live in `git log -- docs/CHANGELOG.md`._
