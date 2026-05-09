# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.2-alpha — 2026-05-09 — Debug-sweep fix pass: 28 audit findings landed

Audit-driven fix sweep across the codebase. 28 findings from the 2026-05-09 debug audit (60 total reported, 28 actioned this pass) span functional bugs, UX polish, and pre-public-ship gates.

### Critical fixes
- **C1 CSP** — replaced `csp: null` w/ a sane `default-src 'self' ipc: …` policy. Closes the largest webview attack surface.
- **C2 TOFU prompt** — `probe_server_fingerprint` + `set_server_fingerprint` Tauri commands. First-connect now probes the server, hands the fingerprint to a Confirm dialog, and only persists after explicit user acceptance. Replaces the prior silent blind-TOFU.
- **C3** `bootstrap_list_files` no longer panics on non-ASCII remote paths — switched from lowercased-byte-slice to `eq_ignore_ascii_case` + `split_at` on the original.
- **C4** `wireEvents()` race — synchronous `wiring` boolean gate + try/catch rollback. No more double-registered listeners on HMR or accidental double-mount.
- **C5** `EditChangedEvent` now carries `serverKey` (Rust + TS); listener drops events whose key doesn't match the currently-selected server. No more wrong-server reupload prompts.

### High-impact fixes
- **H1** `openBootstrap` in-flight guard — no double-detection on rapid clicks.
- **H3 + H12** `editor_for` lock narrowed: check map → drop guard → SFTP open → reacquire → `or_insert`. Slow-server connect no longer freezes other editor calls.
- **H4** documented Velopack public-key pinning TODO at the actual call site.
- **H5** AddServer `__HOME__` literal-tilde gone — new `default_ssh_key_path` Tauri command returns the resolved path.
- **H6** TwoPane Edit/Diff/Delete toasts no longer reference shipped phases — copy now describes actual behavior.
- **H7** `LocalPane` lock badge matching switched to full remote path via new `remoteForLocalPath` helper. Eliminates false-positive locks across same-named files.
- **H11** `reject_path_traversal` guard added to `generate_ssh_key` + `probe_server_fingerprint`.
- **H13** Toast `setTimeout` handles tracked + cleared on rapid flashes (TwoPane, DriftReview).
- **H15** AddServer `gotoStep` validates intermediate steps before forward jumps.
- **H16** AddServer ssh-preview shows `(invalid port)` instead of silently dropping `-p`.

### Medium / cosmetic
- **M3** `urlencoding_minimal` doc updated to clarify it escapes `&=+` (already correct, doc was misleading).
- **M11** DriftReview resets `subpathsText` on server switch when no saved value exists.
- **M14** Bootstrap "close Rift to cancel" copy → "click Cancel".
- **M19** Bootstrap close result includes `cancelled` flag.
- **M7** `pill` and `heroVariant` rewritten as `$derived.by(() => …)` for explicit dep capture.
- **M8** Activity-feed truncation uses `.slice(0, 200)` instead of `.length =` mutation.
- **L1** Removed unused `FolderOpen` import in LocalPane.
- **L15** ActivityFeed pause-counter clamped via `Math.max(0, …)`.
- **L16** ActivityFeed virtual-list `{#each}` key dropped `startIdx` — rows reuse on scroll.
- **L5** Verified `GITHUB_REPO_URL` points at `Blazzer10200/rift-tauri` ✅.

### Verify
- `cargo check --lib` — clean, only known russh future-incompat note.
- `cargo clippy --lib --tests` — clean.
- `npm run check` — 3933 / 0 errors / 1 advisory (intentional Settings:18).

v0.2.1-alpha archived.
