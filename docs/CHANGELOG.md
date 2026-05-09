# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.4-alpha — 2026-05-09 — Audit fix-pass round 3: 4 more findings + verified no-ops

Continuation autopilot of the 2026-05-09 audit sweep while user AFK. Round 1+2 closed 42 findings. Round 3 closes 4 more and verifies 3 already-correct items, leaving 6 genuinely deferred (POSIX-port / upstream / scoped-session work).

### Closed
- **M4** `atomic_write_json` hardened on Windows — `sync_all()` before rename for crash-durability + 5-attempt retry on `MoveFileExW` sharing violations (AV / Search indexer / Explorer thumbnailer transient holds). Cleans up tmp file on terminal failure.
- **M16** drag-drop payload `catch {}` upgraded to `console.warn` in `LocalPane.svelte` and `RemotePane.svelte` — silently swallowed JSON parse failures now surface for debugging.
- **L11** `connection.disconnect()` doc comment — explains tauri event listeners are deliberately preserved across disconnect/reconnect (re-wiring would race with in-flight emits).
- **L2** Dead utility types removed from `$lib/utils.ts` — `WithoutChild`, `WithoutChildren`, `WithoutChildrenOrChild` were shadcn-svelte boilerplate, zero imports across `src/`.

### Verified no-op (audit findings already correct)
- **M10** `selectedConflict` prune effect — already wired at `AppShell.svelte:46-51` via prior round.
- **L7** Tauri capabilities — `core:default` + explicit window perms + `opener:default` is already minimal for current feature set.
- **L10** Ctrl+P shortcut — actually wired at `AppShell.svelte:106` → `gotoSettings("servers")`. Command-palette claim is accurate.

### Deferred (need scoped sessions, not autopilot)
- **M6** POSIX file perms — latent, no Windows impact.
- **M13** Bootstrap mid-chunk cancel — needs Rust-side cancellation token threaded through `download_paths`. Chunk-level cancel (50/chunk) already works.
- **L4** russh future-incompat — upstream crate.
- **L8** hostname binary on POSIX — latent.
- **L9** dual-crypto stack — needs `rustls-platform-verifier` feature-flag investigation to drop `aws-lc-rs`.
- **L14** ConflictResolver semantics — code-correct, audit asked for behavioral verification only.

### Verify
- `cargo check --lib` — clean, only known russh future-incompat note.
- `npm run check` — 3933 / 0 errors / 1 advisory (intentional Settings:18).

### Note
Source bumped to 0.2.4-alpha but **no installer built this session** — user explicitly steered to dev-server-default workflow. Run full build pipeline (`npm run tauri build` + silent install + iconcache bust) when batch is ready to ship.

v0.2.3-alpha archived.
