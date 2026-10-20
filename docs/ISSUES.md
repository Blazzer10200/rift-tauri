# Rift issue tracker

Open work only. Remove an issue when it ships; never renumber IDs. The next new ID is `#110`. Re-search named symbols before editing because line numbers drift.

## #101 — Update-feed trust root

**Status:** owner/operational decision · **Priority:** strategic security

Velopack packages are intentionally unsigned, so TLS, R2 access control, and the release runner protect the auto-executed update path. Scope and rotate the R2 credential, restrict it to the release bucket, and keep feed publication on the trusted runner. Do not reopen code-signing unless the owner changes that decision.

**Where:** `src-tauri/src/update_service.rs`, `.github/workflows/release.yml`

## #102 — MCP path-resolution TOCTOU

**Status:** open · **Priority:** low/theoretical

Workspace paths are canonicalized and checked before callers open them. A privileged local writer could swap a symlink between those operations and redirect the handle outside the root. If Rift begins operating on shared or untrusted workspaces, open first and verify file identity through the handle.

**Where:** `src-tauri/src/assistant/mcp_server.rs` (`resolve_under_roots`)

## #103 — Per-line CLI IPC traffic

**Status:** profile first · **Priority:** performance

Each Claude CLI stdout line crosses Tauri IPC separately. The frontend throttles rendering but not the crossing. Measure lines per second and CPU on a real long stream; if material, coalesce consecutive text deltas into roughly one frame while flushing immediately before permissions, results, and errors.

**Where:** `src-tauri/src/assistant/turn.rs`, `src/lib/state/assistant/streaming.ts`

## #104 — Repeated accumulated-text copies

**Status:** profile first · **Priority:** low performance

`appendText` immutably rebuilds blocks and concatenates the full accumulated message each frame. A rope or tail-chunk representation adds rendering complexity, so change it only after a long-response profile proves this is significant.

**Where:** `src/lib/state/assistant/streaming.ts` (`appendText`)

## #105 — Authenticated OpenAI API matrix

**Status:** live verification required · **Priority:** provider boundary

Automated contracts cover discovery, SSE decoding, tools, permissions, cancellation, persistence, and UI rendering. This machine has not completed a billed live request with a user-owned OpenAI API key.

Verify through Settings → Providers with a disposable key: model discovery, one text response, a read-only file tool, a denied write, an image turn, cancellation, restart/resume, and absence of credentials or prompt bodies in diagnostics. Never put the key in a shell or log.

**Where:** `src-tauri/src/assistant/openai.rs`

## #107 — Provider-neutral AI update feed

**Status:** blocked on a stable source · **Priority:** strategic

Workspace news currently and correctly identifies official Claude Code release coverage. Add source-tagged OpenAI/Codex changes only when an official machine-readable feed is available. Do not scrape challenge-protected HTML or automate a signed-in ChatGPT page in customer installs.

**Where:** `src/lib/components/workspace/NewsFeed.svelte`

Permanent regression rules were moved to `AGENTS.md` and `docs/ARCHITECTURE.md`; this file no longer carries closed issues, historical investigations, or release archaeology.
