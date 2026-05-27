# Security — Known Accepted Advisories

> Per-advisory rationale for items `cargo audit` / `npm audit` flag but that we
> explicitly accept and ship. Re-evaluate on every dep bump.

## RUSTSEC-2023-0071 — `rsa 0.10.0-rc.16` Marvin Attack timing sidechannel

- **Severity:** MEDIUM (CVSS 5.9)
- **Status:** ACCEPTED. No upstream fix available.
- **Transitive chain:** `russh` → `russh-keys` → `ssh-key` → `rsa`.
- **Rift exposure:** RSA is only invoked at SSH host-key signature *verification*
  during a single, user-initiated SFTP connect. We do not verify untrusted-server
  signatures at scale and do not process attacker-chosen RSA inputs in a
  high-volume / chosen-ciphertext setting. The Marvin attack vector (timing
  oracle on PKCS#1 v1.5 decryption) does not match Rift's use.
- **Practical risk:** LOW. Documented to satisfy `cargo audit` + filtered via
  `--ignore RUSTSEC-2023-0071` in `scripts/audit.ps1`.
- **Revisit when:** `russh` bumps `ssh-key` past the patched `rsa` release
  (track upstream).

## Linux GTK chain — `atk`, `gtk`, `gdk` unmaintained warnings

- **Severity:** WARNING (informational, no CVE assigned).
- **Status:** N/A — Rift's only release target is Windows x64. The GTK chain is
  only pulled in for Linux feature-flag completeness via tauri and never
  compiled in shipped artifacts. Suppressed from triage.

## npm — vitest / esbuild / vite chain (dev-only)

- **Severity:** MODERATE (3 advisories: dev-server SSRF + esbuild dev request).
- **Status:** ACCEPTED for the alpha train.
- **Why:** Fix path is `npm audit fix --force` which bumps vitest to 4.x — a
  breaking change blocked behind the test rollout (#21 / #265). Dev server is
  only ever bound to `127.0.0.1` and only exposed during `npm run dev`.
- **Revisit when:** vitest test suite stabilizes; one batched 4.x bump.

## npm — `cookie <0.7.0`, `@sveltejs/kit` query.batch (dev/SSR-only)

- **Severity:** LOW + MODERATE.
- **Status:** ACCEPTED. Rift ships as a Tauri desktop app; the SvelteKit SSR
  surface and cookie parsing path are not used in the production webview.
- **Revisit when:** dep bump arrives that doesn't force a SvelteKit major.
