# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 128 — 2026-05-21 — UI/UX overhaul (S127 + S128 shipped as v0.4.22-alpha)

Shell polish (left-rail flip, titlebar divider, statusbar bump w/ breathing pulse), workspace cross-fade transitions + slide-fade tabs rail, History date grouping + Hide-tests filter, Diagnostics tile-grouping (3 sections), Settings Reset 2-click confirm, WatchedFolders compact display, Composer placeholder trim, dropped `agents`+`attachments` stubs, New-chat button moved flush-after-tabs + restyled, EmptyState refresh (rotating halo + 2×2 resume tiles + dropped synced-card), History moved to ChatTabsBar popover (portal to body, position:fixed anchored). Full notes: `docs/CHANGELOG.md`.

## Session 127 — 2026-05-21 — Split-pane polish + UI audit (shipped v0.4.22-alpha alongside S128)

TasksDock per-pane, per-pane status chip, resizable divider w/ kbd+dblclick reset + persisted fracs, OpenInPaneMenu right-click on tabs+history rows. UI audit doc `docs/design/ui-audit-2026-05-21.md` (5 P0 / 7 P1 / 4 P2 from 9 workspace shots).

## Session 126 — Split-pane v2.1: per-tab composer + concurrent send

`TabState.draft`/`attachments` per-tab ($state); `composerDraft`/`composerAttachments` are back-compat getter/setter shims on store. Composer + EmptyState rewired around `tabId` prop, AssistantPane always renders Composer (gated on tabId, not focus). Auto-compact walks `panes[]` w/ `ctxPctFor`/`ctxWindowFor`. `stop/addAttachment/removeAttachment/compactConversation` accept optional tabId. Per-tab error banners. Focus rail + always-on pane chrome.

---

## Session 126 — Split-pane v2.1: per-tab composer + concurrent send

`TabState.draft`/`attachments` per-tab ($state); `composerDraft`/`composerAttachments` are back-compat getter/setter shims on store. Composer + EmptyState rewired around `tabId` prop, AssistantPane always renders Composer (gated on tabId, not focus). Auto-compact walks `panes[]` w/ `ctxPctFor`/`ctxWindowFor`. `stop/addAttachment/removeAttachment/compactConversation` accept optional tabId. Per-tab error banners. Focus rail + always-on pane chrome.

## Session 125 — Phase E1/E4/E5 + agents-pill (shipped v0.4.21-alpha)

E1 ctx-stats pill, E4 retired-JSONL sweep, E5 history search across summaries.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.22-alpha** (S126+S127+S128 shipped). Tauri 2 + Svelte 5 + Rust + russh.

**Smoke gate still open** from S124 — live verify on real chat:
- Auto-trigger @70% + 5min cooldown
- Pre-emption "Approaching auto-compact at 70%" pill
- Header Compact button (≥50% ctx)
- Agents-pill on real `Task` spawn
- E1 ctx-stats: `Ctx X% → est Y%`

**Next code lanes:**
1. Smoke gate (above)
2. Files diff-dot per row (needs new backend `drift_scanner` per-row verdict command) — deferred from S128
3. Files breadcrumb `\` → `›` chevrons — deferred from S128
4. UI audit gaps: settings sub-tabs, chat-with-content, split-pane visual, backend organization (lib.rs/assistant.svelte.ts/assistant/mod.rs)
5. Refactor queue (each its own `/plan`): split `lib.rs` (2118L), `assistant.svelte.ts` (3109L), `assistant/mod.rs` (2244L)

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
- Settings is workspace (kbd **8** post-S128), `Ctrl+,` flips; do NOT reintroduce slideover scrim.
- **History popover** — lives in ChatTabsBar via `<HistoryDrawer compact onSelected>`. Popover uses `use:portal` to `<body>` + `position: fixed` anchored from button rect — putting it inside the normal tree hits `.tabs-rail` overflow:hidden clip.
- **WorkspaceShell cross-fade** — all once-opened panes mounted absolute-layered; only active is opacity:1 + `inert={false}`. Don't switch back to `[hidden]`/`display:none`.
- Assistant scrollbar: `.scroll` + `.strip` BOTH `scrollbar-width: none` — don't reintroduce `scrollbar-gutter: stable`.
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0`. Don't reintroduce empty-tabs CTA.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- **`tauri.conf.json` `dragDropEnabled: false`** — removing breaks cross-region HTML5 DnD. Rift has no file-drop Tauri events, cost = zero.
- **AssistantPane drop handlers on `.pane` outer div only** — never move to inner `.drop-zone` overlays; loses the continuous-preventDefault chain.
- **`compactionHistory[]` field name is camelCase** in persisted JSON (`compactionHistory`, not `compaction_history`) — Rust extracts via `Value::get("compactionHistory")` in `assistant_list_conversations`. Don't rename.
