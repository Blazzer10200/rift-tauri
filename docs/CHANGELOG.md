# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — pure-assistant conversion (uncommitted on v0.4.46)

> **Why.** Rift is becoming a pure local-workspace coding assistant. The entire SFTP/sync/server/RCON half — the original reason Rift existed — has been stripped from both UI and backend. What remains is the Claude assistant working against a locally-picked folder.

**Frontend (Phase A–C).** IA collapsed to 3 tabs (Home · Chat · Settings). Home repurposed as an assistant dashboard (folder-picker + recents + ask bar). Onboarding reduced to a single Claude-auth step. Removed all sync/conflict/browser/activity/diagnostics surfaces, server/bootstrap/keygen dialogs, and the connection/sync stores. Settings gutted of Server/RCON/SSH/fingerprint/remote-shell sections.

**Backend (Phase D).** Deleted `sftp/ sync/ bootstrap/ edit/ tunnel/ transport/ bridge/ rcon/ profile/`, `local_fs.rs`, `path_guard.rs`, `state/{remote_state,sync_snapshot}.rs`, `assistant/remote_bridge.rs`, and `commands/{sftp,sync,rcon,profile}.rs`. Trimmed `lib.rs` managed state + command registry to the assistant/browser/stt/update surface. The MCP server now exposes only `read_file / list_dir / grep` + the local-git tools (`git_status/diff/log/pull/commit/push`); the loopback bridge and all sync/remote-shell/`ask_user` tools are gone (`ask_user` dropped with the bridge — the assistant falls back to plain-text questions). Diagnostics keeps the `DiagBus` + panic hook + frontend pump; the sync-only `diag_state` DTO/pump was removed and `diag_log_frontend_error` moved to `commands/mod.rs`.

**Verify.** `cargo check` 0 errors / 0 warnings · `npm run check` 0 / 0 (4077 files). Stale `allow_remote_shell` keys in `~/.rift` are ignored (serde, no `deny_unknown_fields`). Version intentionally NOT bumped — ships as one arc.

**UI mock-fidelity pass (same arc).** Aligned the live UI to the Claude design handoff. The accent is now emerald everywhere — the composer ring, send button, context-gauge divider, and chat-tabs aurora no longer tint by model family (model identity stays on the picker swatches + history dots); the Bypass-permissions pill keeps its amber warning color. The chat sidebar gained drag-to-resize (persisted, 180–420px, double-click to reset), matching the Activity dock. The Activity dock's Steps section became a persistent chronological log — every tool call (done + pending), newest-first, as two-line rows (target / "verb · time ago") with a +adds/−removed stat on writes — and its quick-actions split into three separate pills with a centered empty state. Code blocks gained a taller header with a language pill and airier line-height, and file-edit diffs are now syntax-highlighted per line (reusing the shared shiki core). Verified `npm run check` + `cargo check` 0/0; Activity dock CDP-verified live. Grouped tools, step numbers, step dividers, and the turn-summary bar were already present in the thread and left intact.

**Theme-consistency pass + Settings layout polish (same arc).** Killed every leftover hardcoded identity hue so the whole UI tracks the chosen accent: tool-chip categories (read/shell/agent/meta), model badges (sonnet/opus/haiku), the "ultra" effort-tier ramp, and the markdown note/tip callouts all now resolve to `var(--accent)` — only true status colors (warn/danger/ok, diff add-del, error/ask) stay fixed. Scrollbars made theme-independent neutral grey (`--border-strong → --fg-faint → --fg-muted`) everywhere instead of accent-tinted. Files: `app.css`, `ToolChip.svelte`, `HistoryDrawer.svelte`, `Composer.svelte`, `Markdown.svelte`. Settings page: left-aligned the doc column (was auto-centered → marooned in a ~460px dead band beside the nav); rewrote stale copy still referencing the ripped server/SSH features. **Fixed a real `.st-row` layout bug** — wide controls (a long Claude-Max subscription pill) crushed the label/description body to 45px (one word per line); rows now `flex-wrap` so an oversized control drops below the description, right-aligned (`.st-row-body flex: 1 1 300px`, `.st-row-ctl margin-left:auto`, with `.st-row-stack > .st-row-body flex:0 0 auto` so the basis doesn't leak into column-stacked rows). All section layouts CDP-verified live; `npm run check` 0/0 (4079 files).

## v0.4.46 — 2026-05-31 — feat: permanent activity dock with live "reacts as it works" feedback

> **Why this release exists.** The assistant activity dock graduated from a peek-on-demand panel into a permanent, live workspace. Work no longer silently appears and vanishes — each tool call visibly lands with its result, the dock gained quick actions, and file outputs now show how much changed.

**The activity dock is always open.** It used to default closed and snap shut on every new or cleared conversation; now it's a permanent surface that stays put across new / clear / tab-switch (you can still hide it from the composer when you want the room). [assistant.svelte.ts](src/lib/state/assistant.svelte.ts)

**Quick actions, surfaced.** A compact toolbar at the top of the dock: copy the whole transcript as Markdown, compact the conversation, and jump to the latest message — plus a one-click Stop while a turn is streaming. [ActivityPanel.svelte](src/lib/components/assistant/ActivityPanel.svelte)

**File outputs show their churn.** Every file Claude writes or edits now carries a `+added / −removed` line count, accumulated across repeat edits, so a glance tells you what changed where.

**Live work reacts as it happens.** The headline change: a finished tool no longer just disappears. Each call now lands with a green check and its duration (a red ✗ on error), holds for a moment, then eases out — and rows slide and reorder instead of popping in and out. The redundant idle pulses were cut down to a single live indicator, and a completed turn ends on a calm "Done · {time}" confirmation. All motion respects your reduced-motion setting.

**Verify.** `npm run check` 0 / 0 (4110 files); live-verified across real streaming turns via CDP (live row → completion tick → turn-end confirmation). NSIS bundle + SHA256 round-trip via `release.ps1` at ship.
