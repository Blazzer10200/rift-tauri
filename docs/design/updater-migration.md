# Updater migration — Velopack → `tauri-plugin-updater`

> Status: **planned, not started**. Authored 2026-05-26 after v0.4.31-alpha ship. Locks the decision so the implementation arc can be transcribed, not re-debated.
>
> Scope: replace Velopack-rust w/ Tauri's first-party updater + NSIS installer for all post-v0.4.32 updates. v0.4.31 → v0.4.32 stays on Velopack (one-time legacy bridge); v0.4.32 → v0.4.33+ uses the new path.

---

## 1. Decision

**Adopt `tauri-plugin-updater` (Tauri-first-party) for all future updates.** Velopack-rust gets ripped out entirely. NSIS installer stays — Tauri builds it natively (`createUpdaterArtifacts: true`).

**Why this beats alternatives** (decided, not re-litigated here — see `chat 2026-05-26` for survey):

- Velopack's `apply_updates_and_restart` doesn't exit Tauri — bug source. Tauri-updater auto-exits ("On Windows the application is automatically exited when the install step is executed due to a limitation of Windows installers" — official docs).
- MSIX is cleanest *technically* but: ~$300/yr cert, packaging migration, store/sideload model — disproportionate for a small-user alpha.
- Omaha / Squirrel.Windows / WinSparkle — same generation as Velopack, no integration advantage, more effort to wire than the first-party plugin.
- WinGet — distribution channel, not in-app updater. Optional secondary path later.

**Trade-off accepted:** no delta updates. Rift ships ~20-30 MB; full re-downloads are 3-10s on broadband. The delta-vs-full logging stash (v0.4.31 era) becomes moot.

---

## 2. Current state (pre-migration, frozen at v0.4.31-alpha)

### Backend (Rust)

| File | Role | Action |
|---|---|---|
| [src-tauri/src/update_service.rs](../../src-tauri/src/update_service.rs) | 391L — wraps `velopack::UpdateManager`, custom `GithubSource` impl against GitHub REST API | **DELETE** entire file |
| [src-tauri/src/commands/update.rs](../../src-tauri/src/commands/update.rs) | 78L — `app_version`, `check_for_updates`, `download_update`, `apply_pending_update` Tauri commands | **REWRITE** to thin wrappers over `tauri_plugin_updater::UpdaterExt` |
| [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs) | L94 `velopack::VelopackApp::build().run()`; L147 `.manage(Arc::new(UpdateService::new()))`; L25 `pub mod update_service`; L215-217 command registrations | Drop Velopack init line; drop `UpdateService` managed state; drop `pub mod update_service`; add `.plugin(tauri_plugin_updater::Builder::new().build())`; add `tauri_plugin_process` plugin (needed for `relaunch()` from Rust path or JS) |
| [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) L64-67 | `velopack = "=0.0.1298"` + `ureq = "3"` (only used by GithubSource) | Remove both; add `tauri-plugin-updater = "2"` and `tauri-plugin-process = "2"` |

### Frontend (Svelte)

| File | Role | Action |
|---|---|---|
| [src/lib/state/updates.svelte.ts](../../src/lib/state/updates.svelte.ts) | 209L — `UpdateStore` w/ state machine, `invoke("check_for_updates")` / `download_update` / `apply_pending_update`, progress listener, snooze persistence | **REWRITE** internals to use `@tauri-apps/plugin-updater`'s `check()` + `update.downloadAndInstall(cb)` + `@tauri-apps/plugin-process` `relaunch()`. **Public API unchanged** — `state`, `info`, `progress`, `applyNow()`, `snooze()`, `checkOnLaunch()` all keep their signatures so callers untouched |
| [src/lib/components/dialogs/UpdateDialog.svelte](../../src/lib/components/dialogs/UpdateDialog.svelte) | Consumer of UpdateStore | No change (store API preserved). Drop the stashed `applyingHint`/`applyingStuck`/Force-Quit branches — not needed when apply is fast |
| [src/lib/components/UpdateToast.svelte](../../src/lib/components/UpdateToast.svelte) | Consumer | No change |
| [src/lib/components/shell/StatusBar.svelte](../../src/lib/components/shell/StatusBar.svelte) | Consumer (pill) | No change |
| [src/lib/components/AppShell.svelte](../../src/lib/components/AppShell.svelte) | Calls `updates.checkOnLaunch()` | No change |
| [src/lib/components/settings/Settings.svelte](../../src/lib/components/settings/Settings.svelte) | Reads `currentVersion` via store | No change |
| package.json | Add `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process` | Edit |

### Capabilities

[src-tauri/capabilities/default.json](../../src-tauri/capabilities/default.json) — add `"updater:default"` and `"process:default"`.

### Release pipeline

| File | Role | Action |
|---|---|---|
| [scripts/release.ps1](../../scripts/release.ps1) | 266L — vpk pack + vpk upload github, ASCII-safe release notes | **REWRITE** — drop all `vpk` calls; produce NSIS bundle via `npm run tauri build` (already does it); upload `*-setup.exe` + `*-setup.exe.sig` + `latest.json` via `gh release create` |
| [scripts/bump.ps1](../../scripts/bump.ps1) | 3-file version sync | No change |
| `.github/workflows/*` | None present | Could add `tauri-action` CI later; not in scope for v0.4.32-bridge |

### Stash to discard

`updater-overhaul-awaiting-2machine-test` (5 files) — patches the velopack apply path. Becomes dead code after migration. **Drop the stash; do not pop.** Salvageable bits (already in v0.4.31's UX via separate work or rewritten):

- `kill_child_processes_on_exit` taskkill helper — keep ONLY if Tauri's auto-exit on NSIS install proves insufficient (see Risk R3 below). Default: drop.
- `applyingHint` / `applyingStuck` / Force-Quit UI — drop. Tauri NSIS apply is sub-30s; no hint needed.
- Delta-vs-full path log in `UpdateService.download` — drop. No deltas in new path.
- `checkOnLaunch` background pre-download — **SALVAGE.** Useful regardless of backend. Re-implement in the new `updates.svelte.ts` against `update.download()` (without `install()`).
- `#263` listener-singleton doc comment — moot, listeners are managed by plugin SDK now.

---

## 3. Target architecture (post-migration)

```
GitHub Releases (Blazzer10200/rift-releases)
  Tag: v0.4.33-alpha
  Assets:
    Rift_0.4.33-alpha_x64-setup.exe       <- NSIS installer (Tauri-built)
    Rift_0.4.33-alpha_x64-setup.exe.sig   <- ed25519 signature (Tauri-signed)
    latest.json                            <- updater feed (hand-rolled by release.ps1)

Rift v0.4.32 client (already installed)
  └─ tauri-plugin-updater
       ├─ on launch: GET latest.json from rift-releases
       ├─ if newer:  download .setup.exe + verify .sig against embedded pubkey
       └─ user clicks "Install": Tauri exits → NSIS Setup.exe runs (passive mode,
            small progress window) → file swap → NSIS launches new exe → done
```

### `latest.json` shape (static, on GitHub releases)

```json
{
  "version": "0.4.33-alpha",
  "notes": "...top CHANGELOG.md entry, ASCII-safe...",
  "pub_date": "2026-05-30T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<contents of .sig file, literal — not a path>",
      "url": "https://github.com/Blazzer10200/rift-releases/releases/download/v0.4.33-alpha/Rift_0.4.33-alpha_x64-setup.exe"
    }
  }
}
```

### Endpoint config in `tauri.conf.json`

```jsonc
"plugins": {
  "updater": {
    "pubkey": "<base64 pubkey from rift.key.pub>",
    "endpoints": [
      "https://github.com/Blazzer10200/rift-releases/releases/latest/download/latest.json"
    ],
    "windows": { "installMode": "passive" }
  }
}
```

`installMode: "passive"` — small progress dialog, no user interaction needed. Spec'd default; do not use `"quiet"` (NSIS can't elevate from quiet) or `"basicUi"` (interactive — bad UX).

---

## 4. Signing keys — CRITICAL DON'T-TOUCH class

Tauri-updater's signature check **cannot be disabled** (per docs). If the private key is lost, **no existing v0.4.32+ install can ever receive an update again** — they'd have to be told to manually re-install from a fresh Setup.exe. This is a permanent ship-blocker.

### Storage plan

- Generate via `npm run tauri signer generate -- -w ~/.tauri/rift.key` (one-time).
- **Private key (`rift.key`)** — store in two places minimum:
  1. `~/.tauri/rift.key` on the build machine (`C:/Users/BLAZZER/.tauri/`).
  2. Encrypted copy in a separate location off-machine — recommend a password-managed secrets store (1Password, Bitwarden) OR an encrypted `.7z` on a separate drive. The user picks; just *do not skip step 2*.
- **Password (`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`)** — set during `signer generate`. Store next to the key copy (same vault). Not strictly required by Tauri but recommended (prevents key-file leak from being game-over).
- **Public key (`rift.key.pub`)** — embedded in `tauri.conf.json` (committed). No secrecy needed.

### Release-time env

Add to `scripts/release.ps1` preflight (after dirty-tree check):

```powershell
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    throw "TAURI_SIGNING_PRIVATE_KEY not set — load the key before releasing"
}
```

Set via `.secrets/env.sh` (workspace-level secrets file already referenced in `C:/AI Workflow/CLAUDE.md`) OR a per-release `$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~/.tauri/rift.key -Raw`. The plain `$env:...` path is fine; the secret never logs.

### Key rotation policy

Not implementing now. Theoretical path: ship a new pubkey via a regular update (clients pick up the new one from `tauri.conf.json`), then start signing future releases with the new key. If both old and new clients exist at the same time, you need to sign with the old key one final time before rotating. **Don't rotate unless compromised** — the rollover window is fragile.

### Drop into `CRITICAL DON'T-TOUCH` (HANDOFF.md)

After implementation, add to the CRITICAL list:

> `~/.tauri/rift.key` is the updater signing key — lose this and no installed client can ever update again. Backed up in <vault>. Pubkey lives in `tauri.conf.json:plugins.updater.pubkey`; do not regenerate.

---

## 5. Release pipeline rewrite

### `scripts/release.ps1` — new shape (target ~120L, down from 266L)

```powershell
# 1. Preflight (UNCHANGED)
#    - version sync across 3 files
#    - clean working tree (unless -Force)
#    - tools on PATH: npm, gh  (NOTE: vpk dropped)
#    - tag does not already exist in rift-releases
#    - TAURI_SIGNING_PRIVATE_KEY env set  (NEW)
#    - CHANGELOG top entry version matches  (UNCHANGED)
#    - Convert-ToAsciiSafe still used for release notes (latest.json `notes`)

# 2. Build
npm run tauri build
# Output: src-tauri/target/release/bundle/nsis/Rift_<ver>_x64-setup.exe
#       + src-tauri/target/release/bundle/nsis/Rift_<ver>_x64-setup.exe.sig

# 3. Generate latest.json
$setupPath = "src-tauri/target/release/bundle/nsis/Rift_${version}_x64-setup.exe"
$sigPath   = "$setupPath.sig"
$sigContent = [System.IO.File]::ReadAllText($sigPath).Trim()
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$downloadUrl = "https://github.com/Blazzer10200/rift-releases/releases/download/$tag/Rift_${version}_x64-setup.exe"

$latest = @{
    version    = $version
    notes      = $bodyAscii  # already prepared
    pub_date   = $pubDate
    platforms  = @{
        "windows-x86_64" = @{ signature = $sigContent; url = $downloadUrl }
    }
} | ConvertTo-Json -Depth 5

$latestPath = "Releases/latest.json"
[System.IO.File]::WriteAllText($latestPath, $latest, [System.Text.UTF8Encoding]::new($false))

# 4. Create GitHub release + upload
gh release create $tag `
    --repo Blazzer10200/rift-releases `
    --title $tag `
    --notes-file $releaseNotesFile `
    --prerelease:($version -match '-(alpha|beta|rc)') `
    $setupPath $sigPath $latestPath
```

### Things release.ps1 stops doing

- `vpk pack`, `vpk upload github` — gone
- Staging dir creation (`Releases/staging-$version`) — gone
- `--channel win` — gone
- nupkg/delta upload — gone
- RELEASES + releases.win.json generation — gone
- Splash image bundling — gone (NSIS uses its own template; customizable later via `installerTemplate` if wanted)

### Things it keeps

- `Convert-ToAsciiSafe` — still used to sanitize release notes for `latest.json` and GitHub release body
- 3-file version sync preflight
- Dirty-tree refusal w/ `-Force` override
- Tag-exists-already preflight
- CHANGELOG top-entry extraction

---

## 6. Migration strategy — existing users

### The transition window

- **Currently shipped:** v0.4.31-alpha, Velopack-installed at `%LocalAppData%/Rift/` w/ Velopack layout (`current/rift-tauri.exe`, `Update.exe`, `packages/`, etc.).
- **Goal state:** v0.4.33+ installed at `%LocalAppData%/Rift/` (Tauri NSIS perUser default — same parent dir but flat layout) w/ Tauri-updater wired.

### The bridge release: v0.4.32-alpha

v0.4.32 is the **last Velopack release AND the first Tauri-updater-aware build.** Two-pronged build:

**Inside the binary:**
- `tauri-plugin-updater` is integrated.
- `velopack` crate is **removed** (the line `velopack::VelopackApp::build().run()` is dropped from `lib.rs`).
- `update_service.rs` is deleted.
- `commands/update.rs` is the new thin Tauri-updater wrapper.
- `tauri.conf.json` has `createUpdaterArtifacts: true` + updater plugin config.

**Outside (release artifacts):**
- `release.ps1` produces BOTH:
  - The new Tauri-updater bundle (`Rift_0.4.32-alpha_x64-setup.exe` + `.sig` + `latest.json`) for v0.4.32+ clients.
  - **A one-time legacy Velopack package** — repackaged via a stripped-down `vpk pack` call wrapping the same Setup.exe — so v0.4.31's Velopack client can pull v0.4.32.
- The Velopack `Update.exe` swap on the v0.4.31→v0.4.32 transition still has the slow-apply bug (we cannot patch v0.4.31's *installed* behavior). User waits 5-10 min once. After that, v0.4.32+ updates apply in ~30s via Tauri-NSIS.

### How v0.4.31 finds v0.4.32

Same path it uses today — v0.4.31's `GithubSource` polls `rift-releases` for the newest tag and looks for `releases.win.json` + `*-full.nupkg`. We keep those assets on the v0.4.32 release **for this release only**.

### How v0.4.32 finds v0.4.33

`tauri-plugin-updater` polls `https://github.com/Blazzer10200/rift-releases/releases/latest/download/latest.json` — GitHub's automatic redirect to the latest release. As soon as v0.4.33 ships w/ `latest.json`, all v0.4.32+ clients pick it up.

### Install-path collision

Both Velopack and Tauri NSIS `installMode: "currentUser"` install under `%LocalAppData%/<productName>/`, but with different layouts:

- Velopack: `%LocalAppData%/Rift/current/rift-tauri.exe`, `%LocalAppData%/Rift/Update.exe`, `%LocalAppData%/Rift/packages/`
- Tauri NSIS perUser: `%LocalAppData%/Rift/rift-tauri.exe` (flat)

When v0.4.31's Velopack swaps in v0.4.32, the Velopack layout is preserved — v0.4.32 runs from `current/rift-tauri.exe`. Tauri-updater inside v0.4.32 then polls for v0.4.33; when it installs, the NSIS setup writes to `%LocalAppData%/Rift/rift-tauri.exe` (flat). After the v0.4.33 swap:

- **Old velopack files exist as orphans:** `current/`, `Update.exe`, `packages/`. ~20-40 MB wasted disk but harmless.
- **Add/Remove Programs has TWO entries:** "Rift" (velopack-registered) and "Rift" (NSIS-registered).
- The NSIS install runs the new exe; the old velopack-pathed exe is dead (NSIS doesn't relaunch from `current/`).

### Cleanup options (decide at v0.4.33 ship time)

**Option A — passive ignore (recommended).** Document the orphans in CHANGELOG and HANDOFF; users can manually run the Velopack uninstaller from Add/Remove Programs ("Rift" entry that's NOT the latest version). Simple, no migration code to maintain.

**Option B — auto-cleanup on first launch.** v0.4.33 first-launch detects `%LocalAppData%/Rift/Update.exe` + `%LocalAppData%/Rift/current/` and deletes them. Risk: deleting from `%LocalAppData%/Rift/Update.exe` while another user account has Velopack-Rift installed could cross streams. Acceptable risk given Rift is `currentUser`-scoped.

**Option C — NSIS preinstall hook.** Add `installerHooks` to `tauri.conf.json` pointing at `windows/hooks.nsh` with a `NSIS_HOOK_PREINSTALL` macro that calls the velopack uninstaller silently. Risk: velopack uninstaller may not be silent-friendly; depends on installation registry layout.

**Decision: Option A for v0.4.33. Revisit Option B at v0.4.34 if the orphans bother anyone.**

### Failure-mode safety net — manual install link

The CHANGELOG entry for v0.4.32 ships with: *"If the auto-update apply phase exceeds 15 min, download Setup.exe directly from <release URL> and run it manually. This is the last Velopack-managed update."* Mirror in the in-app dialog's `error` state.

---

## 7. Risk register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Private signing key lost | Low | **Catastrophic** — all v0.4.32+ installs orphaned forever | Two-place backup (build machine + off-machine vault). Documented in CRITICAL DON'T-TOUCH after impl. |
| R2 | v0.4.31 → v0.4.32 apply still hangs (Velopack bug) | High | Medium — one-time 5-10 min wait per user | Documented in CHANGELOG + in-app dialog. Manual Setup.exe link as escape hatch. |
| R3 | Tauri-updater's auto-exit doesn't kill child processes (claude CLI, etc.) → NSIS Setup hits file-lock on a child-held resource | Medium | Medium — install partial, recoverable by relaunch | Wire `updater_builder().on_before_exit(|| crate::assistant::kill_child_processes_on_exit())` — salvages the stashed taskkill helper *only* for this hook. Single call site, ~20L. |
| R4 | NSIS perUser install over an open Rift binary fails (file lock) | Low | Low — Tauri auto-exits before install | Already handled by plugin. Verify on first end-to-end test. |
| R5 | `latest.json` UTF-8 BOM tripping Tauri's JSON parser | Low | Medium — silent update break | Write `latest.json` w/ `UTF8Encoding(false)` (BOM-less) — explicit in `release.ps1` rewrite (see §5). |
| R6 | GitHub `releases/latest` redirect points at a non-latest prerelease | Low | Medium — wrong version served | The `latest` redirect prefers full releases over prereleases. Since Rift is alpha-only, **all** releases are prereleases — GitHub picks the most-recent prerelease as `latest` when there are no stable releases. Verified by the current rift-releases setup where v0.4.31-alpha shows up via the existing Velopack feed. Re-verify post-migration on first v0.4.33 ship. |
| R7 | `tauri build` fails to emit `.sig` (env var unset) | Low | High — release ships unsigned, clients reject | Preflight in `release.ps1` (env-var check). Verify `.sig` file exists post-build before `gh release create`. |
| R8 | Velopack's NSIS Setup.exe (v0.4.32) and Tauri's NSIS Setup.exe (v0.4.32) collide in name/identity in Add/Remove Programs during the bridge release | Medium | Low — two registry entries during the bridge | Documented in §6 "Install-path collision". Option A cleanup. |
| R9 | `dragDropEnabled: false` + `decorations: false` + Tauri-updater progress window (separate NSIS window) z-order issue | Low | Cosmetic | Tauri-updater's progress window is a native NSIS dialog, owned by the installer process — not affected by Rift's webview window config. Verify on first test. |
| R10 | `tauri-plugin-updater` plugin version drift vs `tauri = "2"` core | Low | Medium — build break or runtime error | Pin both to compatible majors. Updater plugin is in Tauri's plugins-workspace; matches Tauri 2.x. Re-check Cargo.lock after add. |
| R11 | `npm run tauri build` previously succeeded only because Velopack didn't gate on signing; new path requires `TAURI_SIGNING_PRIVATE_KEY` always. Dev builds may fail. | Medium | Low — dev workflow break | `createUpdaterArtifacts` only triggers `.sig` generation. Set `false` for dev. Two-mode config: dev=`false`, release=`true`. Honor via env var in `tauri.conf.json` or pass `--config` override at build time. Simpler: leave `createUpdaterArtifacts: true` always, set `TAURI_SIGNING_PRIVATE_KEY` in `.bashrc` (read from `~/.tauri/rift.key`). Each `tauri build` then signs. Dev (`npm run tauri dev`) doesn't bundle, so unaffected. |
| R12 | User on second machine (buddy) doesn't get the v0.4.31→v0.4.32 update before the v0.4.32→v0.4.33 release fires — they'd see v0.4.33 in the GitHub releases but their v0.4.31 Velopack client wouldn't find `releases.win.json` for v0.4.33 (it only has `latest.json` + Tauri-updater assets) | Medium | High — stuck on v0.4.31 forever | **Mitigation:** Hold v0.4.33 ship until buddy confirms they're on v0.4.32. Also keep v0.4.32's Velopack feed assets on the GitHub release indefinitely (they're harmless to new clients, life-saving to laggards). |
| R13 | Backend test build w/ `tauri-plugin-updater` + `createUpdaterArtifacts: true` first time requires keys generated — block at integration step | Certain | Low — one-time setup | Step 1 of implementation = generate keys. Cannot start coding migration without them. |

---

## 8. Implementation checklist — strict order

### Phase A — Pre-impl (do this BEFORE any code edit)

- [ ] **A1.** Generate signing keys: `npm run tauri signer generate -- -w C:/Users/BLAZZER/.tauri/rift.key`. Set password when prompted (record in vault).
- [ ] **A2.** Verify `rift.key` and `rift.key.pub` exist. `rift.key.pub` content is needed for `tauri.conf.json` pubkey field.
- [ ] **A3.** Back up `rift.key` + password to off-machine vault. **Until this is done, do not proceed.**
- [ ] **A4.** Add `TAURI_SIGNING_PRIVATE_KEY` to user environment (read from key file at shell init OR set in `.secrets/env.sh`). Verify `echo $TAURI_SIGNING_PRIVATE_KEY` returns key content in a fresh shell.
- [ ] **A5.** Add private key path + password storage notes to CRITICAL DON'T-TOUCH in HANDOFF.md (don't commit until impl is on a branch).

### Phase B — v0.4.32 bridge release (single arc)

Work on branch `updater-migration` to keep main clean.

- [ ] **B1.** `git checkout -b updater-migration`. Drop the stash WITHOUT popping: `git stash drop stash@{0}` (the salvageable `checkOnLaunch` background pre-download will be re-added in B7 instead of from the stash). **Or** keep the stash dropped only after the salvage line is transcribed.
- [ ] **B2.** `cargo remove velopack ureq --manifest-path src-tauri/Cargo.toml`. Confirm no other crate references `velopack::` or `ureq::` (`grep -r "use velopack\|use ureq" src-tauri/src/`).
- [ ] **B3.** `cargo add tauri-plugin-updater tauri-plugin-process --manifest-path src-tauri/Cargo.toml --target 'cfg(any(target_os = "macos", windows, target_os = "linux"))'`. Verify Cargo.lock resolves.
- [ ] **B4.** Delete [src-tauri/src/update_service.rs](../../src-tauri/src/update_service.rs). Remove `pub mod update_service;` from [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs):25.
- [ ] **B5.** Rewrite [src-tauri/src/commands/update.rs](../../src-tauri/src/commands/update.rs):
  - Drop `UpdateService` state arg.
  - `check_for_updates` → `app.updater()?.check().await?` returning a new `UpdateInfoDto` w/ `version`/`releaseName`/`sizeBytes`/`notesMarkdown`/`releaseUrl`/`publishedAt`. Note: Tauri-updater's `Update` struct has `version`, `body` (notes), `date`. No size, no releaseUrl. Resolve by leaving `sizeBytes: 0` and `releaseUrl: ""` initially; consider a separate HTTP HEAD to `update.download_url` for size if desired (not v0.4.32 scope).
  - `download_update` → `update.download(|chunk, total| ...)` w/ Tauri `Channel` for progress, or `app.emit("update-progress", pct)`.
  - `apply_pending_update` → stop autosync/tunnel (as today), then `update.install(bytes)`. Tauri exits the app automatically post-install on Windows; the command may not return. Wire `updater_builder().on_before_exit(|| kill_child_processes_on_exit())` in setup (R3 mitigation).
  - The `Update` instance from `check()` must be held in managed state between `download_update` and `apply_pending_update` — mirror current `pending: Option<UpdateInfo>` pattern w/ `Mutex<Option<Update>>`. (Tauri docs' sample uses this exact pattern — see §"Updater command" in the doc fetch.)
- [ ] **B6.** Edit [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs):
  - Remove L94 `velopack::VelopackApp::build().run();`
  - Remove L147 `.manage(std::sync::Arc::new(update_service::UpdateService::new()))`
  - Add `.plugin(tauri_plugin_updater::Builder::new().build())` after existing `.plugin(tauri_plugin_dialog::init())`
  - Add `.plugin(tauri_plugin_process::init())` likewise
  - Add `.manage(PendingUpdate(Mutex::new(None)))` (new managed type defined in `commands/update.rs`)
  - Keep the rest of `setup` block intact.
- [ ] **B7.** Rewrite [src/lib/state/updates.svelte.ts](../../src/lib/state/updates.svelte.ts) internals (keep public API). Use `import { check, type Update } from '@tauri-apps/plugin-updater'` + `import { relaunch } from '@tauri-apps/plugin-process'`. Background pre-download in `checkOnLaunch` (salvage from stash spec). Drop applyingHint/Stuck/timer fields.
- [ ] **B8.** Edit [src-tauri/capabilities/default.json](../../src-tauri/capabilities/default.json) — add `"updater:default"` and `"process:default"`.
- [ ] **B9.** Edit [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json):
  - Add `"createUpdaterArtifacts": true` to `bundle`.
  - Add `"plugins": { "updater": { "pubkey": "<paste from rift.key.pub>", "endpoints": ["https://github.com/Blazzer10200/rift-releases/releases/latest/download/latest.json"], "windows": { "installMode": "passive" } } }`.
- [ ] **B10.** Edit [package.json](../../package.json) — add `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process` to dependencies. Run `npm i`.
- [ ] **B11.** `/check` — must be clean.
- [ ] **B12.** `cargo check --manifest-path src-tauri/Cargo.toml` — must be clean. (Avoid while `npm run tauri dev` is alive per CLAUDE.md.)
- [ ] **B13.** First end-to-end smoke test (local-only):
  - Set `TAURI_SIGNING_PRIVATE_KEY` (and password if used).
  - `npm run tauri build` — verify NSIS produces `*-setup.exe` AND `*-setup.exe.sig`.
  - Install Setup.exe locally. Confirm Rift launches.
  - Tweak the local install's `tauri.conf.json` pubkey OR craft a fake `latest.json` served via a local file server (Tauri-updater supports `dangerousInsecureTransportProtocol: true` for `http://localhost:...` testing only) to point at a higher-versioned build.
  - Click "Install" in dialog → confirm sub-30s apply + relaunch.
- [ ] **B14.** Bridge-release `release.ps1` for v0.4.32 — produces BOTH Tauri-updater artifacts AND Velopack artifacts. Spec:
  - Build NSIS Setup.exe + .sig (Tauri).
  - Then `vpk pack -p <staging w/ same exe> -e rift-tauri.exe ...` to wrap the same exe in a Velopack package.
  - Generate `latest.json`.
  - `gh release create` w/ ALL assets: Setup.exe, Setup.exe.sig, latest.json, *.nupkg, releases.win.json, RELEASES.
  - Mark as `--prerelease`.
- [ ] **B15.** Bump version to v0.4.32-alpha via `scripts/bump.ps1 0.4.32-alpha`.
- [ ] **B16.** Add CHANGELOG entry: explains this is the last Velopack release, names the apply-time concern, links to manual Setup.exe in case of failure.
- [ ] **B17.** Smoke-test v0.4.31 → v0.4.32 apply on the second machine. Time the apply phase. Expect 5-10 min (the bug we're escaping); confirm v0.4.32 runs after relaunch.
- [ ] **B18.** Ship v0.4.32-alpha via bridge `release.ps1`.
- [ ] **B19.** Wait — confirm both user machines are on v0.4.32. **Do not ship v0.4.33 until this is true** (R12).

### Phase C — v0.4.33 clean release

- [ ] **C1.** Rewrite [scripts/release.ps1](../../scripts/release.ps1) to the §5 spec (Tauri-only path, no vpk).
- [ ] **C2.** Verify the new release.ps1 by dry-running locally (skip the `gh release create` step on first dry-run; inspect generated `latest.json` for shape).
- [ ] **C3.** Bump to v0.4.33-alpha. CHANGELOG entry: "first Tauri-updater-managed release, Velopack removed."
- [ ] **C4.** Ship via new release.ps1.
- [ ] **C5.** Confirm v0.4.32 → v0.4.33 apply on both machines. Expect ~30s apply.
- [ ] **C6.** Update HANDOFF.md CRITICAL DON'T-TOUCH: add the signing-key entry; remove the Velopack-related entries (`bundle.targets:["nsis"]` stays, the `VelopackApp::build().run()` mention is gone).

### Phase D — Polish (optional, post-v0.4.33)

- [ ] **D1.** Decide on Option A/B/C orphan cleanup per §6. If B: add a one-shot cleanup on first launch of v0.4.34. (Ship in v0.4.34, not v0.4.33 — keep v0.4.33 surgical.)
- [ ] **D2.** Consider adding WinGet manifest to `winget-pkgs` for users who prefer `winget upgrade rift`. Strictly optional; secondary path.

---

## 9. Verification commands

Per CLAUDE.md verification rules:

- Backend: `cargo check --manifest-path src-tauri/Cargo.toml` (when dev server NOT running)
- Frontend: `/check` (= `svelte-kit sync && svelte-check`)
- End-to-end: build + install + manual updater trigger w/ local feed
- Post-release: smoke-test apply on a second machine before declaring done

---

## 10. Rollback

If something breaks mid-impl:

- **Pre-B18 (before v0.4.32 ships):** `git checkout main` — branch was isolated. Nothing user-facing affected.
- **Post-B18, pre-C4 (v0.4.32 shipped, v0.4.33 not yet):** v0.4.32 already has Velopack stripped. If a bug surfaces, ship a v0.4.32.1 via the bridge release.ps1 to patch it (Velopack feed still alive for v0.4.31 users; Tauri-updater feed live for v0.4.32 users).
- **Post-C4 (clean Tauri-updater path live):** broken release? Ship a fixed v0.4.33.1. If the updater path itself is broken (e.g. signature rejection across all clients), users must download Setup.exe manually from rift-releases. The manual link in v0.4.32's CHANGELOG covers this.

**There is no path back to Velopack after v0.4.33 ships.** Velopack relies on `Update.exe` being on disk; v0.4.33's NSIS install doesn't place it. Accept this — that's the point of the migration.

---

## 11. Open questions to verify during implementation

These need a real Tauri-updater build + first install to confirm; not blocking the plan:

1. **`Update` size field.** Does Tauri-updater's `check()` response include the download size (for the existing `sizeBytes` UI)? If not, decide whether to drop the size label or fetch it via HTTP HEAD. (My read: not exposed — drop the label. Verify in B5.)
2. **`pub_date` formatting.** Tauri-updater wants RFC 3339. PowerShell `(Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")` is RFC 3339 — verify Tauri accepts it (it should; that's exactly what tauri-action emits).
3. **`on_before_exit` timing.** Does it fire BEFORE the NSIS process spawns (good) or AFTER (too late)? Per docs: "Tauri will automatically quit your application before installing updates on Windows. To perform an action before that happens, use the `on_before_exit` function." → fires before quit. Verified by docs.
4. **NSIS `installMode: "currentUser"` vs updater `installMode: "passive"`** — confirm these are orthogonal (bundle vs updater). Reading the docs: yes, separate keys, separate effects. Bundle's `installMode` controls *initial install* behavior; updater's `installMode` controls *update* progress UI.

---

## TL;DR for future me

1. Generate + back up signing key. (Phase A — single-day blocker.)
2. Build v0.4.32 as a hybrid that publishes BOTH Velopack and Tauri-updater artifacts. v0.4.31 users get one last slow Velopack apply.
3. Verify both machines on v0.4.32 before shipping v0.4.33.
4. v0.4.33 = pure Tauri-updater path. Velopack gone forever.
5. Lose the signing key = lose the ability to update existing installs forever. Two-place backup. Non-negotiable.
