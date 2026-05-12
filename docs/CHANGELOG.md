# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.28-alpha-test — 2026-05-12 — Server-side `find`-exec listing (30-60s → sub-second scans on WAN)

The slow part of every scan was `list_recursive_batch` walking the remote tree over SFTP — open-dir/read-dir/close-dir is O(N) round-trips per directory. On Blazzer's LAN (<1ms RTT) that's invisible; on Trey's Tailscale-tunneled WAN (30-80ms RTT, confirmed direct path via `tailscale status`/`netcheck`) it's 30-60s per scan. Diagnostic ruled out DERP relay + symmetric NAT — pure SFTP protocol overhead.

### Landed
- **`list_via_exec` helper** in [sftp/mod.rs](src-tauri/src/sftp/mod.rs) — runs `find <root> -maxdepth N <prune> -type f -printf '%p\t%s\t%T@\n'` over a single SSH exec channel. One round-trip per root vs N per directory. Prune list mirrors `sync::ignore::ignored_directory_names()` so we don't waste server-side traversal on `node_modules`/`target`/`.git`/etc.
- **`list_recursive_batch` tries exec first.** One exec channel per root in parallel (channels are cheap vs full SFTP sessions). Roots that exec succeeds on short-circuit before any SFTP worker spin-up. Roots that fail (no `find` on PATH, non-POSIX shell, perms) fall through to the existing SFTP worker path → existing belt-and-braces retry. Behavior degrades safely; no profile flag needed.
- **Exec exit-1 tolerated.** `find` returns 1 for "some files failed" (e.g. transient ENOENT mid-walk on a churning dir) but still streams the rest. We accept 0 and 1, fail other codes so fallback kicks in.

### Expected impact
- Reconcile/auto-pull scan time on Trey's link: ~30-60s → ~1-3s.
- Blazzer's LAN: unchanged (already <1s).
- Pull throughput is a separate axis (his uplink caps it); won't improve from this work alone.

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors, 5 pre-existing a11y warnings.

v0.2.27 archived to git log.
