# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.44-alpha-test — 2026-05-12 — Scan-result per-resource breakdown for diagnosis

Adds a `by_resource` field to the `drift_scan_result` event payload and `eprintln!` line: every drift scan now reports ToPush / ToPull / ToDelete / Conflicts counts broken down by resource. Without this, a scan output like "430 to_delete" gave no hint which resources were affected — diagnostic export showed 39 to_delete on `[depend]` and 108 on `[ox]` (both correctly blocked by the mass-delete circuit breaker) with the remaining ~283 unaccounted for. Next diagnostic will show the exact distribution.

### Open investigation
Live testing on v0.2.43 surfaced a likely partial-SFTP-listing issue: after a fresh pull, the drift scanner classified ~430 local files as `ToDelete` — meaning the recursive remote listing returned fewer entries than the snapshot expected. Hypothesis: deep paths under `ox_lib` (4-5 levels nested) or `[bracket]`-encoded resource dirs may be silently dropping during russh-sftp recursive enumeration. Mass-delete circuit breaker prevented actual data loss on the high-count resources. Per-resource breakdown in v0.2.44 will localize the issue for the root-cause fix in the next session.

### Verify
`svelte-check` 0/0/3994 · `cargo check` clean · `vitest` 6/6.
