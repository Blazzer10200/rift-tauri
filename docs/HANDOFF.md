# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 118 — 2026-05-20 — N-pane split (cap 4) + v0.4.15-alpha ship

### Completed
- Generalized `panes` from `[PaneState, PaneState] | null` → `PaneState[]` (always length≥1, 1..MAX_PANES=4)
- Store: new `addPane()` / `closePane(idx)` / `canAddPane` getter; `dropTabIntoPane(tabId, paneIdx: number)` w/ sentinel `paneIdx === panes.length` = auto-add new pane; `scrubTabFromPanes` / `setFocusedPane` / `assignFocusedPane` / `restoreTabs` array-driven; restore clamps focused idx + prunes stale tab refs
- `AssistantPage` renders `{#each panes as p, i}` w/ 1px dividers
- `AssistantPane`: `paneIdx: number`, `min-width: 320px`, new pane-chrome (numbered badge + ✕ close)
- `ChatTabsBar`: single `.in-pane` underline + numeric `.pane-badge` (was `in-p0`/`in-p1`); split-toggle calls `addPane()`, disabled at cap, shows count
- `AppShell`: `Ctrl+\` = addPane, `Ctrl+Shift+\` = closePane(focused)
- `StatusHub` pane lookup via `findIndex`
- CHANGELOG v0.4.15-alpha entry covering S116 + S117 + this session + sync nits
- 3-file bump 0.4.14 → 0.4.15-alpha via `scripts/bump.ps1`

### Verify
- `npm run check` 0 errors / 3 pre-existing CSS warns / 4053 files

### Next session
- v0.4.15 ship via `release.ps1` (push complete, release pending dev-kill)
- Phase 3 split: resize handle on dividers, `paneWidthPct` persistence, drag-FROM-pane
- After ship: #37 + #38 Phase 6 OS-keychain; remaining sync MEDs #47 #58 #59 #60 #62; Wave-2 FE MEDs

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.15-alpha** (released). Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:**
1. #37 + #38 — Phase 6 OS-keychain
2. Remaining sync MEDs: #47 #58 #59 #60 #62
3. Wave-2 FE MEDs: #146 #147 #148 #149 #151-#177
4. Split Phase 3: resize handle on divider, `paneWidthPct` persistence, drag-FROM-pane (currently only drag-INTO-pane works)

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
