# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 125 — 2026-05-21 — Phase E1/E4/E5 + agents-pill UX + workspace cleanup

Uncommitted. Will ship as v0.4.21-alpha on next `/git-ship`.

**Phase E polish (3 of 5):**

- **E1 — Boundary pill ctx stats.** `BoundaryBlock` + `ctxPctBefore?/ctxPctEstAfter?`; `compactConversation()` snaps `ctxPct` at stage + `outputTokens/ctxWindow*100` at finalize; MessageBubble pill renders `Ctx X% → est Y%`. ([assistant.svelte.ts:69-91, 2787-2870](src/lib/state/assistant.svelte.ts), [MessageBubble.svelte:362-366](src/lib/components/assistant/MessageBubble.svelte#L362))
- **E5 — HistoryDrawer search across summaries.** Rust `ConversationMeta + compaction_summaries`, extracted via `Value::get("compactionHistory")`. HistoryDrawer filter falls through to summary text. ([assistant/mod.rs:336-348,509-547](src-tauri/src/assistant/mod.rs), [HistoryDrawer.svelte:11-21](src/lib/components/assistant/HistoryDrawer.svelte))
- **E4 — Retired-JSONL sweep.** `pub fn cleanup_retired_jsonls()` walks convos for retired sessionIds, deletes matching CLI JSONLs older than 30d. Wired via `spawn_blocking` in setup(). ([assistant/mod.rs:808-893](src-tauri/src/assistant/mod.rs#L808))

**Background-agents UX.** Tabs already fully independent (recon: zero backend serialization; `TabState.streaming` per-tab). Was signage, not architecture. Shipped: agents-pill clickable → `newTab()` while old tab streams in BG; background-streaming tabs get accent-tinted bg + pulsing 2px underline (reduced-motion honored). ([ChatTabsBar.svelte:298-309,463-490](src/lib/components/shell/ChatTabsBar.svelte))

**Cleanup batch:** Deleted dead `AssistantHeader.svelte` (448L, 0 refs); removed `scratch/`; pruned `Releases/` (370M→72M); moved `state/audit-2026-05-20/` to `state/_archive/`; trimmed CHANGELOG + HANDOFF under cap.

**Deferred:** E2 (default-collapse archived — behavioral regression w/o buy-in); E3 (tab-close prompt — possibly annoying); background-agents popover panel (pill functional).

**Verify:** `npm run check` 0/0 (3 pre-existing CSS warnings unrelated). `cargo check` skipped (dev alive).

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.20-alpha** (S123+S124 shipped). S125 uncommitted. Tauri 2 + Svelte 5 + Rust + russh.

**Smoke gate still open** from S124 — needs live verify on a real chat (not test data):
- Auto-trigger crossing at threshold=70% + 5min cooldown
- Pre-emption "Approaching auto-compact at 70%" pill
- Header Compact button (≥50% ctx)
- Background-agents pill on real `Task` spawn
- E1 ctx-stats render: `Ctx X% → est Y%` next to cost/model

**Next code lanes:**

1. Run `/git-ship` to land S125 (Phase E1/E4/E5 + agents-pill + cleanup). Bumps to v0.4.21-alpha.
2. Live-smoke gate (above) — 15-min CDP-driven pass.
3. Known refactors from punch list: split `lib.rs` (2118L) → `commands/*.rs`; split `assistant.svelte.ts` (3109L); split `assistant/mod.rs` (2244L). Each deserves its own `/plan` session.

**Phase 6 keychain runtime:** old plaintext `bridgeToken`/`apiKey` get auto-lifted to Windows Credential Manager on first config load. `cmdkey /list:rift` for `rift/bridge.<server_key>` + `rift/assistant.api_key`.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` + full `BUILTINS` in `--allowed-tools`.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` when attachments present. 20MiB cap.
- Settings is workspace (kbd 9), `Ctrl+,` flips; do NOT reintroduce slideover scrim.
- Assistant scrollbar: `.scroll` + `.strip` BOTH `scrollbar-width: none` — don't reintroduce `scrollbar-gutter: stable`.
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0`. Don't reintroduce empty-tabs CTA.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- **`tauri.conf.json` `dragDropEnabled: false`** — removing breaks cross-region HTML5 DnD. Rift has no file-drop Tauri events, cost = zero.
- **AssistantPane drop handlers on `.pane` outer div only** — never move to inner `.drop-zone` overlays; loses the continuous-preventDefault chain.
- **`composerDraft` stays store-level** — moving to per-pane requires Composer rewire across ~30 refs; not worth it.
- **`compactionHistory[]` field name is camelCase** in persisted JSON (`compactionHistory`, not `compaction_history`) — Rust extracts via `Value::get("compactionHistory")` in `assistant_list_conversations`. Don't rename.
