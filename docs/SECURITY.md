# Security — Known Accepted Advisories

> Per-advisory rationale for items `cargo audit` / `npm audit` flag but that we
> explicitly accept and ship. Re-evaluate on every dep bump.

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
