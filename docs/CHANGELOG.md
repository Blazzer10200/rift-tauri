# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.23 — 2026-06-11 — Activity panel polish

> **Why.** The dock's Steps log echoed raw MCP tool ids twice per row (`mcp__rift__ask_user` as both title and sub-line), every MCP call got the generic wrench icon, multi-turn logs blurred into one flat list, and the promised Sources section never existed.

- **MCP steps humanized** — `classifyTool` parses `mcp__<server>__<tool>`: rift's `read_file`/`list_dir`/`grep`/`ask_user`/`notify`/`open_browser` get proper verbs, payload targets (question text / toast title / URL — ask_user input is nested `{questions:[…]}`), and per-tool icons (help/bell/globe); `git_*` maps to shell; unknown MCP tools stop echoing the raw id twice.
- **Turn separators** in the Steps log ("TURN N ─ ago") whenever it spans more than one turn.
- **New Sources section** — deduped URLs fetched/opened + web queries; click a link to open externally, a query to copy.
- **Last-turn recap upgraded** — two-line reply preview with accent quote bar (markdown stripped); stat grid aligned with section padding.
- **Opaque spine-node icons** — translucent `-soft` fills let the timeline spine bleed through the icon boxes (user-reported on hover); ask/write/pending/error tints now mix opaquely.
- Empty-state copy restructured with a title line.

**Verify.** svelte-check 0/0 (4093 files) · vitest 122/122 · live CDP pass with fresh error buffer: 0 console errors, all sections pixel-verified.

## Older versions

v0.8.22 multi-tab stream survival (live TabState authoritative over disk; pointer-switch tabs) + Harness mission control (active-sessions cell, turn drill-down, health alerts) + `/history` fix + dead-code sweep (−331L) + poison-safe CACHE locks · v0.8.21 self-aware Rift — loopback UI bridge resurrected (`bridge.rs`: ask_user card round-trip / open_browser dock / notify toast) + per-turn env snapshot + localhost links open in-app · v0.8.20 live plan limits — cost-cockpit "Plan limits" card + `/usage` popover via undocumented OAuth usage endpoint (CLI token read-only, 60s cache) · v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping, slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips + `turn.rs` overlapping-turn registry race fix · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8) · v0.8.15 hot-file splits + honest Settings update chip · v0.8.14 update-dialog render crash fix + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model · v0.8.12 pill `×` = 24h snooze · v0.8.11 Settings redesign + Harness one-viewport · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
