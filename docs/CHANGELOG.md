# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.9-alpha — 2026-05-09 — Sync Inspector + bidirectional sync + bridge URL fix

Closes the sync wheel both directions and ships a one-button diagnostic surface so the next bug doesn't take a full session to triage.

### Landed

- **Sync Inspector** — new hidden tab at Ctrl+Shift+D. 14 live state tiles (autosync, queue, ignored, locks, conflicts, last remote scan, pulled, last drift, bus lag, total emitted, …). Live event stream w/ expandable JSON rows. One big "Copy diagnostic report" button bundles state + last 200 diag events + last 100 activity rows + profile (sanitized) + locks + conflicts + ignored-by-rule histogram into clipboard JSON. Inline scan-interval picker. ([Diagnostics.svelte](../src/lib/components/diagnostics/Diagnostics.svelte), [diagnostics.svelte.ts](../src/lib/state/diagnostics.svelte.ts), [diagnostics/mod.rs](../src-tauri/src/diagnostics/mod.rs))
- **Bidirectional sync** — new `DriftWatcher` task on the engine, ticks every 30s (configurable 15s/30s/1m/2m/5m/off). Auto-pulls `ToPull` entries, registers `Conflict` entries in the existing UI, **never overwrites a dirty local file** — sidesteps to `<file>.rift-conflict.<user>@<host>-<ts>.<ext>` per Syncthing's safety model. Respects cross-dev `LockPresence`. ([sync/drift_watcher.rs](../src-tauri/src/sync/drift_watcher.rs))
- **Rescan handler** — `notify::Event::need_rescan()` now auto-fires a drift reconcile. Kernel/FSEvents drops are no longer silent.
- **Bridge URL fix** — `BridgeClient` URLs now correctly prefix `/rift_bridge/` (FXServer `SetHttpHandler` routes under the resource name). Profile `bridgePort` corrected 30121 → 30120 (FXServer game port). Hot-reload pings stop failing. ([bridge/mod.rs](../src-tauri/src/bridge/mod.rs))
- **Trail-loop fix** — `.rift-trail.jsonl` added to ignore patterns. Stops the pull → notify → push → EditTrail-rewrite-trail → drift-sees-newer → loop.
- **20+ new diag emit points** across upload/lock/bridge/remote-scan/remote-pull paths; every step traceable in the panel.

### Verify

- `cargo check`: clean. `svelte-check`: 0 errors. Bidirectional pull tested live against homelab FXServer.

v0.2.8-alpha archived.
