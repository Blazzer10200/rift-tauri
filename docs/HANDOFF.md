# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## Session 19 — 2026-05-09 — v0.2.9-alpha: Sync Inspector + bidirectional sync + bridge fix

The sync wheel is now closed in both directions and Rift has a one-button diagnostic surface so the next bug doesn't take a full session to triage. User went to bed mid-session; ship + push + power-down were autopiloted.

### Completed
- **Sync Inspector** ([Diagnostics.svelte](../src/lib/components/diagnostics/Diagnostics.svelte) + [diagnostics.svelte.ts](../src/lib/state/diagnostics.svelte.ts) + [diagnostics/mod.rs](../src-tauri/src/diagnostics/mod.rs)): hidden tab via Ctrl+Shift+D, 14 live state tiles, virtualized event stream w/ expandable JSON rows, one-button "Copy diagnostic report" that bundles state + last 200 diag events + last 100 activity rows + sanitized profile + locks + conflicts + ignored-by-rule histogram. Inline scan-interval picker.
- **Bidirectional sync** ([sync/drift_watcher.rs](../src-tauri/src/sync/drift_watcher.rs)): periodic remote-scan loop spawned on engine start, ticks every 30s (configurable). Auto-pulls `ToPull` entries, registers `Conflict` entries via existing UI, sidesteps to `<file>.rift-conflict.<user>@<host>-<ts>.<ext>` if local file is dirty (Syncthing's safety model). Respects cross-dev `LockPresence`. Tested live: server-side edit to `[endure]/endure_skills/README.md` pulled within ~30s.
- **Rescan handler**: `notify::Event::need_rescan()` now auto-fires drift reconcile. Fixes silent kernel/FSEvents drops.
- **Bridge URL fix** ([bridge/mod.rs](../src-tauri/src/bridge/mod.rs)): `BridgeClient` URLs now correctly include `/rift_bridge/` path prefix (FXServer `SetHttpHandler` routes under resource name). User's profile `bridgePort` corrected 30121 → 30120 (FXServer game port — `SetHttpHandler` does not bind its own port).
- **Trail-loop fix** ([sync/ignore.rs](../src-tauri/src/sync/ignore.rs)): `.rift-trail.jsonl` added to ignore patterns. Stops the pull→notify→push→trail-rewrite→loop forever pattern observed live.
- **20+ new diag emit points**: UploadStart/Done/Fail (with size+elapsed_ms+error), LockHeldByOther, BridgePing/Ack, RemoteScan, RemotePull. Every pipeline step traceable.
- v0.2.9-alpha bumped + shipped.

### Key Decisions
- Bidirectional polling (30s) is the architecturally correct answer — verified by WinSCP docs (SFTP has no notify channel) and Syncthing's own design (watcher + periodic scan because watchers miss events).
- Conflict-rename safety net is non-negotiable: NEVER overwrite a dirty local file. Pull to a sibling path, surface ConflictRecord.
- Stick with `log` crate, not `tracing` migration. `LogForwarder` mirrors every existing log macro into the diag bus + chains to env_logger. Zero existing-call-site invasion.

### Don't Touch
- `bridgePort` in profile MUST stay 30120 (FXServer game port). 30121 was a configuration drift; resource uses `SetHttpHandler` which doesn't bind a separate port.
- `.rift-trail.jsonl` ignore rule MUST stay in place. Removing it reintroduces the pull→push loop within seconds.
- Frontend pump uses `tauri::async_runtime::spawn`, NOT `tokio::spawn`. The `setup()` hook fires before the tokio runtime is attached to that thread.

### Next Steps
1. **Verify the bridge fix works in practice** — next session: connect, save a file, watch for `bridge_ack: success` instead of WARN. If still failing, check that `[endure]/rift_bridge` is `started` in txAdmin (was found on disk but not listening on 30120).
2. **Stale `.rift-lock` cleanup** — saw 3 orphans on the FXServer (`fxmanifest.lua.tmp.*.rift-lock`) from prior crash sessions. Sweep manually or add a stale-lock GC pass on engine start.
3. **Code-signing cert** (audit H4) — SmartScreen still flags every fresh Setup.exe.
4. **Optional polish**: edit-trail mtime fast-path in DriftWatcher (skip full scan if trail unchanged AND not on every-Nth full-scan tick). Saves SFTP roundtrips on quiet runs.

### Files Modified
- `src-tauri/src/diagnostics/mod.rs` (NEW)
- `src-tauri/src/sync/drift_watcher.rs` (NEW)
- `src-tauri/src/sync/auto_sync.rs` (emit points + DriftWatcher integration + accessors)
- `src-tauri/src/sync/ignore.rs` (`.rift-trail.jsonl` + `.rift-conflict.` patterns)
- `src-tauri/src/sync/mod.rs` (re-export DriftWatcher module)
- `src-tauri/src/bridge/mod.rs` (path prefix fix)
- `src-tauri/src/lib.rs` (diag commands + DriftWatcher state + setup hook spawn)
- `src/lib/components/diagnostics/Diagnostics.svelte` (NEW)
- `src/lib/state/diagnostics.svelte.ts` (NEW)
- `src/lib/components/AppShell.svelte` (Ctrl+Shift+D + tab routing + palette entry)
- `src/lib/components/shell/TabRail.svelte` (Tab type)
- `package.json` + `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` (0.2.9-alpha)
- `~/.rift/rift.json` (bridgePort 30121 → 30120, user-side data)
- `docs/CHANGELOG.md` + `docs/archive/CHANGELOG-archive.md`

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09. Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S19):** **v0.2.9-alpha shipped.** Bidirectional sync live (local→remote watcher + remote→local 30s polling). Sync Inspector tab at Ctrl+Shift+D — one button gives Claude every diagnostic signal. Bridge URL fix should restore FXServer hot-reload (verify next session).

## CRITICAL DON'T-TOUCH
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only
- npm runner, NOT pnpm
- `~/.rift/*.json` file-format compat — never change rename rules; never drop `serde(flatten) extra` on `RiftConfig`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- shadcn-svelte components own themselves in `src/lib/components/ui/`
- `bundle.targets: ["nsis"]` while versions carry `-alpha`/`-beta` (MSI rejects non-numeric semver)
- Tauri 2: `core:default` lacks `window:allow-*` — explicit perms required for custom titlebar
- WPF fingerprint format: `<keytype> <bits> <b64>` w/o `SHA256:` — substring match strips the prefix to handle both shapes
- russh-sftp `session::write()` is WRITE-only; use `session::create()` for any "write a new file" path
- DriftWatcher conflict-rename guard MUST stay — never overwrite a dirty local file
- `.rift-trail.jsonl` ignore rule MUST stay — pull→push loop reappears instantly without it
- `bridgePort: 30120` in profile — `SetHttpHandler` resources route through FXServer's main HTTP port, not a separate one
