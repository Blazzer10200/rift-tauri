# Cross — Self-update

_2 confirmed findings._ [← back to index](README.md)

## Cross — Self-update

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Medium | Unvalidated `releaseUrl` passed to `openUrl()` | `src/lib/components/dialogs/UpdateDialog.svelte:64` | Assert `url.startsWith('https://github.com/')` before calling `openUrl`, or reconstruct from the validated version tag |
| Low | Host-prefix check allows any GitHub repo; no binary hash/sig | `src-tauri/src/commands/update.rs:167` | Tighten prefix to include `/Blazzer10200/rift-releases/`; publish + verify a SHA-256 sidecar post-download |

---

**[Medium] Unvalidated `releaseUrl` opened via `openUrl()`** — `UpdateDialog.svelte:64`

`openReleasePage()` reads `updates.info?.releaseUrl` (sourced from the GitHub API's `html_url` field via `UpdateInfoDto.release_url` in `update.rs:140`) and forwards it to `openUrl()` with zero scheme or host checks. A spoofed `html_url` — via MitM of the unauthenticated HTTPS call to `api.github.com` (no cert pinning; a locally-trusted CA suffices) or direct repo compromise — could open an arbitrary URI (`file://`, `javascript:`, phishing `https://`) in the user's default browser. The `download_update` command already carries an explicit host allowlist (`update.rs:167–169`); this companion path has no equivalent guard. Severity is medium rather than high because exploitation requires compromising a TLS connection or the GitHub account — not trivially achievable — and the impact is limited to browser URL dispatch, not direct RCE.

**[Low] Host-prefix check does not constrain repo path; no binary verification** — `update.rs:167`

The `download_update` host allowlist (`starts_with("https://github.com/")` or `starts_with("https://objects.githubusercontent.com/")`) permits any GitHub-hosted asset, not just the known releases repo. Combined with the intentional absence of signature or hash verification (documented design choice following the v0.4.33 key-loss incident), a spoofed `browser_download_url` pointing to any attacker-controlled GitHub release would pass validation and cause the downloaded NSIS binary to be executed via `openPath()` without any integrity check. Severity is low: both prerequisites (MitM on `api.github.com` TLS or GitHub account compromise) already fully circumvent the host check regardless, making the missing repo-path constraint a defense-in-depth gap rather than a standalone exploitable vulnerability under normal threat conditions.
