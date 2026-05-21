# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 126 — 2026-05-21 — Split-pane v2.1: per-tab composer + concurrent send

### Completed
- **Per-tab composer state** — `TabState.draft` + `TabState.attachments` ($state). Deleted `tabDrafts`/`tabAttachments` Maps + stash/restore dance. `composerDraft`/`composerAttachments` are now getter/setter shims on store (back-compat for slash cmds + send()).
- **Composer.svelte rewired** — reads `tab.draft / tab.attachments / tab.queue / tab.streaming / tab.promptHistory` via `tabId` prop. Per-pane prompt-history recall. Textarea moved from `bind:value` → controlled `value + oninput`.
- **AssistantPane always renders Composer** — dropped `{#if focused}` gate; gated only on `tabId` presence. Submit auto-focuses pane before dispatch so `send()` targets correct tab.
- **EmptyState** accepts `tabId` prop — suggestion clicks write into correct pane's draft.
- **Auto-compact iterates panes** — AssistantPage effect walks `panes[]` + uses new `ctxPctFor(tab)` / `ctxWindowFor(tab)` helpers. Background tabs can no longer sail past threshold.
- **`stop(tabId?)` / `addAttachment(att, tabId?)` / `removeAttachment(id, tabId?)` / `compactConversation(focus?, tabId?)`** — all accept optional tabId.
- **Per-tab error banners** — `lastError` renders in owning pane regardless of focus.
- **Focus rail** — `inset 0 2px 0 var(--accent)` + accent-60% border on focused split-pane. Non-focused pane bg dimmed 94%.
- **Pane chrome always-on** — opacity 0.5 at rest (was 0), 0.95 on hover/focus.
- **`npm run check`** — 0 errors, 3 pre-existing CSS warnings. CDP-verified: 2 independent composers, drafts persist across focus swaps.

### Next Steps
1. **Smoke gate** (S124, still open) — live verify on real chat: auto-trigger @70%, pre-emption pill, header Compact button, agents-pill on Task spawn, E1 ctx-stats render.
2. **TasksDock per-pane** — dock still global/focused-tab; needs per-pane scope in split.
3. **Per-pane status chip** — Ctx%/model/cost in pane chrome (composer StatusHub is per-pane; header pill is not).
4. **Resizable divider** — drag-handle pattern exists in `browser/TwoPane.svelte`.
5. **Open-in-pane N right-click** — tabsbar + history drawer entry point.
6. *(Deferred)* Vertical split / 2×2 grid; keybinds skipped (user decision).

### Files Modified
- `src/lib/state/assistant.svelte.ts` — TabState draft/attachments, store shims, ctxPctFor, stop/addAttachment/removeAttachment/compactConversation tabId params
- `src/lib/components/assistant/Composer.svelte` — full per-tab rewrite
- `src/lib/components/assistant/AssistantPane.svelte` — always-on Composer, error visibility, focus rail CSS
- `src/lib/components/assistant/AssistantPage.svelte` — auto-compact pane-iteration effect
- `src/lib/components/assistant/EmptyState.svelte` — tabId prop

---

## Session 125 — 2026-05-21 — Phase E1/E4/E5 + agents-pill (shipped v0.4.21-alpha)

E1 ctx-stats pill, E4 retired-JSONL sweep, E5 history search across summaries. Agents-pill clickable → newTab() while old tab streams. Cleanup batch. Deferred: E2 collapse, E3 tab-close prompt, agents popover.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.21-alpha** (S125 shipped). S126 uncommitted (split-pane v2.1). Tauri 2 + Svelte 5 + Rust + russh.

**Commit pending:** S126 split-pane v2.1 (per-tab composer, auto-compact per pane, focus rail, pane chrome). Run `/git-ship` when ready.

**Smoke gate still open** from S124 — live verify on real chat:
- Auto-trigger @70% + 5min cooldown
- Pre-emption "Approaching auto-compact at 70%" pill
- Header Compact button (≥50% ctx)
- Agents-pill on real `Task` spawn
- E1 ctx-stats: `Ctx X% → est Y%`

**Next code lanes:**
1. `/git-ship` to land S126 → v0.4.22-alpha
2. Smoke gate (above)
3. Split-pane polish queue: TasksDock per-pane → per-pane status chip → resizable divider → open-in-pane menu
4. Refactor queue (each its own `/plan`): split `lib.rs` (2118L), `assistant.svelte.ts` (3109L), `assistant/mod.rs` (2244L)

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
