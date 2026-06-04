# Cross — Dependencies

_3 confirmed findings._ [← back to index](README.md)

## Cross — Dependencies

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Low | russh 0.60.2 + russh-cryptovec 0.59.0 — unbounded allocation / OOM (RUSTSEC-2026-0154, RUSTSEC-2026-0153) | `src-tauri/Cargo.toml` lines 51–52 | Remove stale `russh` + `russh-sftp` entries; both advisories resolved in one deletion |
| Low | vitest ^2.1.9 — arbitrary file read + RCE via UI server (GHSA-5xrq-8626-4rwp) | `package.json` line 51 | Bump to `vitest@^4.1.8`; review 4.x migration notes |

---

**RUSTSEC-2026-0154 + RUSTSEC-2026-0153 — russh / russh-cryptovec (dead weight)**

`russh = { version = "~0.60", ... }` at Cargo.toml line 51 resolves to 0.60.2, which transitively pulls russh-cryptovec 0.59.0. Both advisories are real: russh 0.60.2 copies a 32-bit SSH packet field directly as an allocation size (OOM/crash); russh-cryptovec 0.59.0 has unchecked buffer-growth that can corrupt memory. However, the entire SSH/SFTP layer was stripped in the 2026-06-03 pure-assistant conversion — grep across all of `src-tauri/src/` finds zero `use russh` or `russh::` imports (only a stale comment at `assistant/mod.rs` line 718). No live code path reaches either crate; the vulnerability requires an active SSH connection to trigger. Both advisories share one root dep, so a single Cargo.toml edit — removing the `russh` and `russh-sftp` lines — eliminates both with zero functional impact.

**GHSA-5xrq-8626-4rwp — vitest UI server RCE**

`"vitest": "^2.1.9"` at package.json line 51 resolves entirely within the vulnerable range (< 4.1.0). The advisory's CVSS 9.8 (AV:N/AC:L/PR:N/UI:N) describes a network-accessible UI server endpoint with no authorization check, allowing any reachable origin to read arbitrary files and execute code. The rated severity is overstated for this project: the `test` npm script is `"vitest run"` — headless, no HTTP server — and `--ui` does not appear in any defined script. The attack surface the score describes is never instantiated unless a developer manually runs `vitest --ui` outside of npm scripts. The dep should be updated to `^4.1.8` (note the semver-major jump; review vitest 4.x migration notes for API breakage), which also resolves transitive esbuild and vite-node advisories that chain through vitest.
