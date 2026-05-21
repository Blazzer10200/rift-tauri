# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.4.21-alpha — 2026-05-21 — Phase E polish + background-agents UX + doc cleanup

**Phase E polish (3 of 5):**

- **E1 — Boundary pill ctx stats.** `BoundaryBlock` gains `ctxPctBefore?: number + ctxPctEstAfter?: number`. `compactConversation()` snaps `ctxPct` at stage time + computes `outputTokens / ctxWindow * 100` at finalize. MessageBubble pill renders `Ctx X% → est Y%` next to cost/model so headroom won per compact is visible at a glance. ([assistant.svelte.ts:69-91, 2787-2870](src/lib/state/assistant.svelte.ts), [MessageBubble.svelte:362-366](src/lib/components/assistant/MessageBubble.svelte#L362))
- **E5 — HistoryDrawer search across summaries.** Rust `ConversationMeta` gains `compaction_summaries: Vec<String>` extracted via `Value::get("compactionHistory")` walk in `assistant_list_conversations`. TS mirror + HistoryDrawer filter falls through to summary text — long-running compacted convos stay searchable by topic. Placeholder updated to "Filter by title or summary…". ([assistant/mod.rs:336-348,509-547](src-tauri/src/assistant/mod.rs))
- **E4 — Retired-JSONL startup sweep.** New `pub fn cleanup_retired_jsonls() -> usize` walks `~/.rift/assistant/conversations/*.json`, collects `compactionHistory[*].priorSessionId` (UUID-shape-validated), deletes matching `~/.claude/projects/*/<uuid>.jsonl` files older than 30 days. Wired in `lib.rs` setup() via `spawn_blocking` so slow disks don't block window-show. Best-effort; errors logged + swallowed. ([assistant/mod.rs:808-893](src-tauri/src/assistant/mod.rs#L808))

**Background-agents UX.** Recon confirmed tabs are already fully independent — backend zero serialization across `session_id`s, `TabState.streaming` per-tab. Was signage, not architecture. Shipped: (1) agents-pill became clickable `<button>` — click spawns fresh tab via `assistant.newTab()` while the old tab keeps streaming in background. (2) Background-streaming tabs (streaming + NOT active) get accent-tinted background + pulsing 2px accent underline; reduced-motion honored. (3) Queue surfacing in Composer already existed for the same-tab case. ([ChatTabsBar.svelte:298-309, 463-490](src/lib/components/shell/ChatTabsBar.svelte))

**Workspace + doc cleanup (massive consolidation).** Deleted dead `AssistantHeader.svelte` (448L, 0 refs — absorbed into ChatTabsBar in S122). Removed `scratch/` + audit shards (`state/_archive/audit-2026-05-20/`, 32 files). Pruned `Releases/` from 46 nupkgs → keep v0.4.18–20 only (370M → 72M, 298MB reclaimed). Merged 4 reference docs (`CONTRIBUTING` + `ONBOARDING` + `TREY-SETUP` + `AUTHORIZED_KEYS`) into one `docs/DEVELOPING.md`. Nuked `docs/archive/` entirely (17 retired files; `git log` preserves) + `docs/plans/` (1 stale, 1 moved to `design/`). Pruned `docs/ISSUES.md` 1638 → 979 lines (265 → 170 numbered issues — dropped all SHIPPED blocks; `git log -- docs/ISSUES.md` preserves the shipped history). New rule on the file: *"When something ships, delete the block — git log is the archive."* Trimmed `CHANGELOG` + `HANDOFF` under the 600-word cap. **Total: 63 → 9 tracked .md files.**

**Verify.** `npm run check` 0 errors (3 pre-existing CSS warnings unrelated). `cargo check` clean post dev-kill. Velopack release shipped via `scripts/release.ps1`.
