# Auto-update — return to Velopack (one-click, then unattended)

> Status: **IMPLEMENTED 2026-06-04 (cont.44), compile-verified, NOT ship-tested**.
> `cargo check` 0/0 (velopack 1.2.0), `npm run check` 0/0, `release.ps1`
> PS-parses clean. Ships v0.4.47. Remaining: the 2-machine ship test (§6 R1) —
> auto-apply only proves out across two real Velopack releases. All signatures
> below verified against docs.rs/velopack/latest (1.2.0) and matched the built
> code. Implementation notes vs this plan: native `GithubSource` replaced the
> custom source (module ~390→~135L); `UpdateCheck::UpdateAvailable` wraps a
> `Box<UpdateInfo>` and the asset is `info.TargetFullRelease`; dropped
> `release_url`/`published_at` from the DTO (synthesized `releaseUrl` client-side
> from the tag instead).
>
> Goal: restore **automatic** updates. User clicks "Update" once to consent;
> Velopack then downloads, applies, and relaunches with no further interaction.

---

## 1. Why Velopack (again), and the full lineage

Three generations of updater, each removed for a concrete reason:

1. **Velopack** (original, Phase-0 → v0.4.31). Dropped in the v0.4.32 migration.
   Stated reason: `apply_updates_and_restart` "exits your app immediately" —
   Velopack tries to kill the process itself, which fought Tauri/WebView2's
   child processes → flaky apply (file-lock during swap). **This is fixable**
   (see §3) and was never the framework's fault.
2. **`tauri-plugin-updater`** (v0.4.32 → v0.4.33). **Bricked every client.**
   Its ed25519 signature check cannot be disabled; the signing key was lost, so
   no installed client could validate any future update — permanent. The
   migration brief flagged this exact risk in a "CRITICAL DON'T-TOUCH" section
   and it happened anyway.
3. **GH-release-API path** (v0.4.34+, current). Deliberate retreat to safety:
   no key to lose, but **manual by design** — detection auto, install needs a
   click that opens Setup.exe in the browser. This is what we're replacing.

**Decision:** return to Velopack. It is purpose-built for background
auto-update (download-in-background, apply-on-restart, delta packages, staged
rollouts) and — crucially — its update verification is **not** a mandatory-key
model like tauri-updater, so it does **not** carry the brick-on-key-loss risk
that killed gen 2. `tauri-plugin-updater` is explicitly rejected for that
reason.

---

## 2. What changed since last time — v1.2.0 is far simpler

Old integration pinned `velopack = "=0.0.1298"`. Latest is **`1.2.0`** (crates.io,
2026-06). The 0.0.x → 1.x jump removes the biggest chunk of old code:

- **Native `velopack::sources::GithubSource::new(repo_url, access_token, prerelease)`**
  now exists. The old module hand-rolled a ~200-line custom `GithubSource`
  (impl `UpdateSource`) *because 0.0.1298 had none*. **Delete all of it.** The
  module drops ~390 → ~120 lines.
- `UpdateManager::new(source, None, None)` — unchanged 3-arg shape.
- `check_for_updates() -> UpdateCheck` (`UpdateAvailable(UpdateInfo)` / etc.) — same.
- `download_updates(&UpdateInfo, Some(Sender<i16>))` — same progress contract.
- `VelopackAsset` fields (PascalCase): `Version, FileName, Size, NotesMarkdown`
  (+ `PackageId, Type, SHA1, SHA256, NotesHtml`). DTO mapping unchanged.
- `UpdateInfo: AsRef<VelopackAsset>` — `info.as_ref()` still yields the asset.

What we lose vs. the custom source: per-release `html_url` / `published_at`
(the custom impl scraped these from the GH REST API). Drop them from
`UpdateInfoDto` or leave empty; the toast/dialog only need version + size + notes.

---

## 3. The bug fix — never call `apply_updates_and_restart` from a GUI

The original apply-exit bug: `apply_updates_and_restart` exits the process
*itself*, racing WebView2's child processes. Use the GUI-friendly path instead:

```
um.wait_exit_then_apply_updates(asset, /*silent*/ true, /*restart*/ true, NO_ARGS)?;
app.exit(0); // let Tauri shut down cleanly; Update.exe waits ≤60s for our PID
```

`wait_exit_then_apply_updates` launches Velopack's Update.exe, tells it to wait
for *this* PID to exit gracefully (≤60s), then it swaps files + relaunches.
`silent=true` → no Velopack UI → unattended apply. We trigger Tauri's own clean
exit, so WebView2 children die in order and no file lock occurs.

`NO_ARGS` = `Vec::<&str>::new()` (concrete type for the `C: IntoIterator<Item=S>,
S: AsRef<OsStr>` bound; `std::iter::empty()` can't infer `S`).

### Resolved code details (closed during research)

- **`VelopackApp::build().run()` ordering.** Must be near-first, BUT after the
  `RIFT_MCP_SERVER` early-return in `run()` — when Rift is spawned as an MCP
  stdio server, *nothing* may touch stdout (corrupts JSON-RPC). The Velopack
  installer never launches with `RIFT_MCP_SERVER` set, so install/update hooks
  still fire on normal + installer launches. Order: `LogForwarder::install()`
  → `RIFT_MCP_SERVER` check/return → `VelopackApp::build().run()` → rest.
  (Old code had VelopackApp right after the logger; the MCP early-return was
  added later in the pure-assistant rip, so it must now precede VelopackApp.)
- **All `UpdateManager` calls are synchronous** in 1.2.0 (`check_for_updates`,
  `download_updates`, `wait_exit_then_apply_updates`). Keep the old
  `tokio::task::spawn_blocking` wrappers in `commands/update.rs`. No `async`
  cargo feature needed.
- **`download_updates(&UpdateInfo, Some(Sender<i16>))`** — progress is `0..=100`
  as `i16`; pump thread forwards to the `update-progress` Tauri event (old
  contract, unchanged).

---

## 4. File-by-file (compile-verifiable: steps 1–5)

| # | File | Change |
|---|------|--------|
| 1 | `src-tauri/Cargo.toml` | add `velopack = "1"` (keep reqwest/futures — used by `stt/model_manager.rs`) |
| 2 | `src-tauri/src/update_service.rs` | **new, slim** (~120L): `UpdateService` wrapping `UpdateManager::new(GithubSource::new("https://github.com/Blazzer10200/rift-releases", None, true), None, None)`. `check`/`download(Sender)`/`apply(app)`. apply → `wait_exit_then_apply_updates(asset, true, true, empty)` + `app.exit(0)`. Keep dev `RIFT_UPDATE_FEED` → `FileSource` under `#[cfg(debug_assertions)]` |
| 3 | `src-tauri/src/lib.rs` | add `pub mod update_service;`; `velopack::VelopackApp::build().run();` as **first line** of `run()` (before logger? — old code put logger first, then VelopackApp; keep that order so install/update hooks log); `.manage(Arc::new(UpdateService::new()))` |
| 4 | `src-tauri/src/commands/update.rs` | replace GH-API impl w/ velopack commands: `app_version`, `check_for_updates`, `download_update` (spawn_blocking + mpsc→`update-progress` pump + `update-downloaded`), `apply_pending_update` (no more AutoSync/Tunnel teardown — both deleted in pure-assistant rip; just call svc.apply(app)) |
| 5 | `src/lib/state/updates.svelte.ts` | re-point invoke contract: `download()` → `invoke("download_update")` listening `update-progress`; "Install" → `invoke("apply_pending_update")` (app exits, so no post-await state). Keep snooze/dismiss/checkOnLaunch/6h-timer as-is |

### Ship phase (NOT compile-verifiable — needs a real release)

| # | File | Change |
|---|------|--------|
| 6 | `scripts/release.ps1` | **revert** to the proven vpk recipe — fully recovered from git `3b70f66^`. Keeps the version-sync preflight + `Convert-ToAsciiSafe` notes scrub. Stage exe+icon → `vpk pack -u Rift -v $ver -p $staging -e rift-tauri.exe --packTitle Rift --packAuthors Blazzer --icon ... -o Releases [--releaseNotes f]` → `vpk upload github --repoUrl https://github.com/Blazzer10200/rift-releases --publish --channel win --tag $tag --token $(gh auth token) [--pre]` |
| 7 | `tauri.conf.json` | NSIS bundle still built by `tauri build`; vpk wraps the exe (not the NSIS installer — vpk produces its own Setup.exe). Verify no leftover updater plugin config |

### Ship-phase PREREQUISITE — vpk/crate version lockstep

**Installed `vpk` is `0.0.1298` (old).** Velopack requires the `vpk` CLI and the
`velopack` crate to be the **same version**. Before the first Velopack ship:

```
dotnet tool update -g vpk          # → 1.x (needs .NET SDK 8)
vpk --help                          # read the actual version from the banner
```

Then pin the crate to match exactly: `velopack = "=<vpk version>"` (e.g.
`"=1.2.0"`). For backend *compile-check* alone, `velopack = "1"` resolves fine;
tighten to the exact pin at ship time so packed-runtime == linked-runtime.

---

## 5. Migration bridge — the one-time wrinkle

Existing v0.4.x clients run the GH-API path with **no Velopack runtime**, installed
via NSIS perUser (`%LocalAppData%\Programs\Rift` or similar). Velopack installs to
its **own** location (`%LocalAppData%\Rift`). So the **first** Velopack release
cannot be a clean in-place upgrade:

- Current clients see it via the existing GH-API check, download the Setup.exe,
  and run it → they land on the **Velopack-managed** install (possibly alongside
  the old one until the old is uninstalled).
- From that build forward, Velopack owns updates and everything is automatic.

This is a known one-time transition (identical to the original Velopack→tauri
bridge). Plan: ship one final GH-API-path release whose installer is the Velopack
Setup.exe, with release notes telling users it's a one-time reinstall. **Test on
two machines** before broad release — auto-update only proves out across two
real Velopack releases.

---

## 6. Risks

- **R1 — apply still races WebView2.** Mitigation: `wait_exit_then_apply_updates`
  + `app.exit(0)` (§3). If still flaky, salvage the old stashed
  `kill_child_processes_on_exit` taskkill helper as a pre-exit hook.
- **R2 — v1.2.0 API drift from docs.** All signatures here are read from
  docs.rs/velopack/latest (1.2.0). `cargo check` is the real verifier — run it
  with dev **quit** (project rule: no `cargo check` during `tauri dev`).
- **R3 — parallel install confusion (§5).** Communicate in release notes; consider
  a one-time uninstaller for the old NSIS path.
- **R4 — no signing.** Velopack supports optional Authenticode signing of the
  installer; not required and not in scope. (This is distinct from the
  tauri-updater mandatory-key model — its absence does not brick clients.)
