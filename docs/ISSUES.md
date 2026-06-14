# Rift — Issue Tracker

> **Single source of truth for open work only.** When something ships, **delete its block** — `git log -- docs/ISSUES.md` preserves history. Issue IDs are **durable**: never re-number, only append. Each block carries `Where` (file:line — may have drifted, re-grep before acting), `Symptom`, and an optional `Fix sketch`.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables lived in `docs/archive/audit-history.md` — archive deleted 2026-06-09 (`1810c2e`), recover via `git log`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

### Conventions

- **Status** — `✅ resolved in-tree` (fixed, unshipped — block stays until `/git-ship` so `git log` keeps it) · `🚧 open` · `👤 needs your call` · `🧪 live-verify` (code-complete, needs runtime confirmation) · `🔒 blocked` (external dep) · `🗄 closed` (decided, kept for the record).
- **Tier** — `T1` ship-blocker / data-safety · `T2` code-complete needs live-verify · `T3` strategic / longer-term · `T4` LOW / cosmetic.

### Index

| ID | Title | Tier | Status |
|----|-------|------|--------|
| #33 | Compaction tool broken | T1 | 🗄 closed (feature removed in minimal-core strip) |
| #34 | Session Diff overlay bugs out when a long session's edits pile up | T2 | ✅ resolved in-tree + live-verified (synthetic 20-file CDP repro) |
| Auth-Rec | In-app sign-in recovery for 401 failures | T2 | 🧪 live-verify |
| Permission | Allow/Deny round-trip bar | T2 | 🔒 blocked (trust gate) |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| CR-UX | Trust segment binary-vs-ternary enum | T3 | ✅ resolved in-tree |
| #29 | CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime | T4 | ✅ resolved in-tree (🧪 prod verify) |
| #30 | Workspace chip vs CLI cwd possible drift | T3 | ✅ resolved in-tree |
| #31 | Deferred 2026-06-11 audit remainder (legacy provider cmds · 401-dup · Fable sunset sweep) | T3/T4 | 🚧 open |
| #32 | Ctx meter blank on restored conversations | T4 | ✅ resolved in-tree |
| #35 | Live shell + sub-agent streaming output panel (Claude-Code-desktop "Background tasks") | T3 | 🚧 open (idea) |
| #36 | Split-pane feature overhaul | T3 | 🚧 open (idea) — 2 concrete pains gathered + fixed (see #38) |
| #37 | Multi-window — separate OS windows (VSCode-style, multi-monitor) | T3 | 🚧 open (idea) |
| #38 | Per-pane STT routing + per-tab workspace root | T2 | ✅ resolved in-tree (🧪 live-verified via CDP; mic input untested) |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## 🚧 Open issues

### Tier 1 — broken feature

#### 34. Session Diff ("Changes") overlay bugs out on long-session pile-up (✅ resolved in-tree + LIVE-VERIFIED 2026-06-12)

- **Live CDP verify (synthetic 20-file/200-edit session, +7000/−6000):** overlay opened instantly, all 20 groups default-collapsed w/ crisp headers (no empty strips), 0 console errors. A 400-line `Write` group expanded to exactly 200 rows + "Show 201 more lines"; clicking revealed all 401. Group toggle + deep-link exemption behave.

- **Fix shipped in-tree (all four sketch items):** (1) header counts memoized per edit id (`countFor` cache — completed tool inputs are immutable; previously every stream tick re-diffed every edit while the overlay was open); (2) groups default-collapsed when >8 files OR a single group exceeds 400 changed lines (seeded once, user clicks sticky, deep-link target stays open); (3) `EditDiff` grew a `maxLines` prop — SessionDiff passes 200; rest hides behind a "Show N more lines" strip (+24 hysteresis); (4) `content-visibility: auto` + `contain-intrinsic-size` on `.dg` groups so off-screen groups skip layout/paint.

- **Where:** [SessionDiff.svelte](../src/lib/components/assistant/SessionDiff.svelte) (314L) + `EditDiff.svelte` it instantiates per edit.
- **Symptom (user screenshot, 19 files +187/−72):** rows render as empty/clipped strips — file headers cut off mid-paint, content rows collapse to thin bars, some groups show only 1-2 diff lines then a void. Gets worse the more edits accumulate.
- **Likely shape:** zero virtualization/cap — every group renders every `EditDiff` eagerly (each runs `diffArrays` per edit; `countDiff` already re-diffs everything for the header counts, so each edit is diffed TWICE). A long session w/ big `Write` payloads (whole-file `new_string` as all-additions) → main-thread starvation mid-layout. Also suspect: `target` scroll `requestAnimationFrame` racing the staggered render.
- **Fix sketch:** default groups COLLAPSED above N files (header counts only — cheap) · memoize/share the per-edit diff between header count + body render · cap rendered lines per edit w/ "show more" · consider `content-visibility: auto` or simple windowing on `.diff-sheet`. Reproduce w/ a synthetic 20-file/200-edit session before picking.

### Tier 2 — code-complete, needs live-verify

#### Auth-Rec — in-app sign-in recovery for 401 failures (🧪 live-verify)

- **Status:** shipped in v0.8.9+ (`9c468a4`+`2d72af8`) — `assistant_open_login(console)` spawn + actionable 401 banner ([Sign in]/[Open Settings]/[Re-check]). CDP-verified all banner states; the live login spawn itself is compile/registration-verified only. **v0.9.3 (RR-1):** the same Sign-in/Re-check now also lives on the `needsAuth` welcome card (`AssistantWelcome.svelte`) — closes the new-user dead-end where a red-pill user could never reach the post-turn banner (send is disabled, so no turn fires).
- **Remaining:** confirm an end-to-end real sign-in on a genuinely-logged-out machine (dev box stays authed). **Strategic follow-ups** (not built): proactive re-probe before first send; auto-prefer an authed install when multiple exist; collapse scattered 401 string-matching into one `AuthError` enum + DiagBus telemetry.

#### Permission — Allow/Deny round-trip bar (🔒 blocked on a trust-standard workspace)

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Live-verify attempt 2026-06-10 (cont.103):** switched to "Ask before edits" + asked for a `git_commit` in a derived-trust workspace — the MCP server correctly **doesn't expose git-write tools at derived trust**, so the prompt can never fire there. That confirms the trust gate works, but the bar itself remains unexercised. Verifying requires pinning `trust_level=standard` on a throwaway repo — deliberately not done unattended because the trust segment **pins one-way** (see CR-UX). Fold into the CR-UX decision: when the trust enum is reworked, verify the bar in the same pass.

### Tier 3 — strategic / longer-term

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Progress (cont.105, 2026-06-10):** audit findings **#1-#6 + #8-#10 SHIPPED** and live-verified — Steps-rail `cd`-strip (`shellLabel` + vitest), slash menu → palette design language, empty-dock auto-collapse, scroll bottom padding, **per-chat model scoping** (`TabState.modelOverride` + `effectiveModel`; opening an old chat no longer rewrites the new-chat default or toasts), jump-back-in snippets + model chips (backend `last_snippet` on `ConversationMeta`), KPI zero-state unify + `opus· high` space fix, user-turn inset card, insight severity stripes. Audit's "Jump back in doesn't navigate" suspicion: **not a bug** (verified live).
- **Remaining from the audit:** #7 cost-chart sparse-data polish · #11 rich inline diff (design pass) · #13 tab strip into titlebar (lowest priority) · message hover actions discoverability. (#12 tool-chip expand affordance ✅ 2026-06-11 — chevron brightened `fg-muted`→accent-on-hover + nudge + Expand/Collapse tooltip on chip-head.) (`/history` fixed + live-verified 2026-06-11 — drawer opens on single Enter via store request flag.) Then the per-page Settings checklist.
- **Input:** [ui-audit-2026-06-09.md](design/ui-audit-2026-06-09.md) — live CDP audit of v0.8.14, 13 ranked findings (refinement tier, not redesign).

#### 31. Deferred remainder from the 2026-06-11 dead-code/debug audit

Three audits (backend, frontend, orphan files) shipped a sweep this session; these were found but deliberately deferred:

- **Legacy provider commands (🗄 superseded 2026-06-12):** the entire custom-provider + compression surface was removed in the minimal-core strip (S6) — nothing left to migrate or sweep.
- **401-detection duplicated in turn.rs (✅ resolved in-tree 2026-06-11):** extracted `is_auth_rejection()` + `auth_rejection_message()`; both the stdout result-frame and stderr-exit sites now share one detector + one remap (unified on the richer CLI-path message).
- **Blocking fs reads in async commands (T4):** `load_config()` in `assistant_send` / `read_oauth_token()` in `usage_rate_limits` do sync disk I/O on the tokio executor. Tiny local files — wrap in `spawn_blocking` only if it ever shows up in traces.
- **Fable sunset sweep (dated):** after Jun 22 (`FABLE_SUNSET_MS = Date.UTC(2026, 5, 23)` in `state/assistant/helpers.ts`), all Fable branches (`fableAvailable()` gates, `fableSunsetNoticed` toast in send.ts, `limited` rows in modelMatrix.ts) become permanently dead — sweep them out.
- **Optional split follow-ups (quality, not threshold — #20 closed at ship):** [messagebubble-split.md](design/messagebubble-split.md) B1-B6 + [chattabsbar-split.md](design/chattabsbar-split.md) T1-T6 stay mapped; `assistant_send` (917L fn inside turn.rs) can split internally later.

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### CR-UX. Trust segment binary-vs-ternary enum (✅ resolved in-tree 2026-06-11 — user signed off in-session)

- **Fix shipped in-tree:** dead `full` dropped. `is_valid_trust_level` now `readonly|standard` (new writes of "full" rejected); `effective_trust_level` migrates persisted ternary-era `full` → `standard` read-side (no disk rewrite); `mcp_server::trust_level()` maps legacy `RIFT_TRUST_LEVEL=full` env → `standard`; `trust_rank` 2-level; turn.rs git-write allowlist gate `== "standard"`; frontend `TrustLevel` type narrowed. Tests updated both sides of the gate.
- **Permission-bar live-verify still rides on a trust-standard throwaway repo** (unchanged — see Permission block).

#### 30. Workspace chip shows current workspace, not the resumed tab's cwd (✅ resolved in-tree 2026-06-11)

- **Fix shipped in-tree:** new `assistant_session_cwd` command (convo_store.rs) exposes the cwd sidecar; `loadConversation` hydrates `TabState.sessionCwd`; ChatTabsBar shows a warn-tinted `cwd-badge` (folder leaf + full-path tooltip) next to the project chip when the active tab's pinned cwd ≠ `workspace.current` (separator/case-insensitive compare). Block stays until `/git-ship`.

- **Seen 2026-06-11 (cont.110, live CDP):** chips said `resume-project` while real turns read remotion-playground files. **Explained:** the turns resumed an old tab whose session was started under remotion-playground — `load_session_cwd` (convo_store) correctly pins a resumed session to its original cwd; the title-bar chip reflects the *currently selected* workspace only. Per-session cwd persistence working as designed.
- **The gap:** nothing in the UI tells you a tab is operating in a different folder than the chip shows. Fix sketch: per-tab cwd badge (tabsbar tooltip or composer notice) when `session_cwd != workspace.current`.

#### 35. Live shell + sub-agent streaming output panel (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** a Chat-page feature that lets users *watch what's happening under the hood* the way Claude Code desktop's pane menu does (its "Background tasks" entry, alongside Preview / Diff / Terminal / Files / Plan). Two concrete asks: **(1) sub-agent activity** — what each spawned sub-agent is actually doing as it runs, not just "N agents"; **(2) terminal I/O, both directions** — the live back-and-forth with a shell, whether *the user* drove it or a *sub-agent* did (command in → streaming stdout/stderr out). Today Rift only shows *counts*: composer LivePills render `▸ N shells` / `◍ N agents`, no rolling output, no per-agent detail.
- **Why it's non-trivial — the data may not exist yet:** the CLI streams tool_use/tool_result envelopes; a `Bash` tool result arrives as one final block, not incremental stdout. Real *live* shell output would need either (a) the CLI to stream partial Bash output (check current envelope shape), or (b) routing long-lived/background Bash through Rift's own MCP/PTY so we own the stream. Sub-agents are worse: per the `agentSpawns` comment in `assistant.svelte.ts`, "CLI does NOT stream intermediate sub-agent activity — we only know spawn + final result." So an agent live-feed is blocked on the same upstream gap.
- **Where to start:** `liveActivity` derivation + `LivePills.svelte` (current counts-only readout) · `TabState.agentSpawns` / shell tracking in `state/assistant/streaming.ts` · the dock/panel was removed cont.122 (don't reintroduce `assistant.ui.dockOpen`) so this is a *new* surface, not a revival.
- **Scope decision needed:** confirm what the CLI actually streams before designing UI — a panel that can only show final output isn't the ask. If upstream won't stream it, the honest version is a per-shell/-agent expandable card showing final output + timing, framed as "tasks" not "live tail."

#### 36. Split-pane feature overhaul (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** "the split pane feature could be way better improved." General dissatisfaction — **specific pain points not yet gathered.** Capture now; scope before building.
- **What exists today:** N-pane split lives entirely *inside the single window* — `panes: PaneState[]` (`{tabId}` array), drag-a-tab-onto-a-half to split, `MAX_PANES` cap, `focusedPaneIdx`, per-pane composer-draft stash/restore. Drop-into-pane handles split-on/swap/focus. All state on the one `AssistantStore`, persisted to `localStorage` `rift.ui.tabs.v1`.
- **Where:** [state/assistant/tabs.ts](../src/lib/state/assistant/tabs.ts) (pane lifecycle: addPane/closePane/setFocusedPane/dropTabIntoPane/scrubTabFromPanes) · `PaneState`/`MAX_PANES` in `state/assistant/types.ts` · `AssistantPane.svelte` renders the pane grid.
- **Candidate improvements (to confirm w/ user):** resizable pane splits (drag the divider — current looks fixed-ratio) · vertical *and* horizontal splits (grid, not just a row) · keyboard pane nav/move · per-pane independent scroll already?/focus ring clarity · raise/remove `MAX_PANES` · drag a pane out → its own window (overlaps #37).
- **Concrete pains gathered (user, 2026-06-14 cont.131) → FIXED in #38:** (1) STT dictation landed in the *other* pane; (2) switching a pane's project folder leaked to other panes (esp. visible after `/clear`). Both resolved — see #38.
- **Pane identity (✅ in-tree 2026-06-14 cont.132, `3b28567`):** panes were identifiable only by a tiny 50%-opacity floating number. Replaced the floating `.pane-chrome` with an always-legible in-flow `.pane-head` strip (split mode only) — pane index + **conversation title** + ctx chip + close; focused pane = accent wash + brighter title. `AssistantPane.svelte`. svelte-check/vitest/CDP green.
- **Resizable dividers already DONE** (`AssistantPage.svelte:29-132` — drag, double-click reset, arrow-key adjust). Don't re-scope as new.
- **Still un-prioritized:** vertical/grid splits (2×2, not just a row) · drag-pane-to-reorder · raise `MAX_PANES`. **Keybinds are OUT** (user doesn't use them — don't add Ctrl/Alt pane shortcuts). Ask for specifics before the bigger overhaul.

#### 37. Multi-window — separate OS windows (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** VSCode-style — open Rift sessions as **separate native windows** so each can live on a different monitor (session on monitor 1, another on monitor 2). Distinct from #36 (splits *within* one window).
- **Feasibility: YES.** Tauri 2 supports multiple `WebviewWindow`s natively. Tractable because the **Rust backend already partitions by session** — turn registry keyed by cliSessionId; stream/permission/error events are **broadcast app-wide** (`app.emit`) carrying the session id, and the frontend filters by id. So multiple windows can drive different sessions against one backend w/o a backend rewrite.
- **Today:** single OS window (`tauri.conf.json` declares one), custom titlebar (`decorations:false`). All tab/pane/convo state in one webview's `AssistantStore` → `localStorage` `rift.ui.tabs.v1` + convos on disk.
- **Route A — "New Window" MVP (a session per monitor):** spawn a second `WebviewWindow` (same app URL, unique label); it boots its own store. Must-fix gotchas: **(1)** shared origin → both windows stomp `rift.ui.tabs.v1` → namespace the persistence key per window label; **(2)** same convo open in 2 windows = disk save race → per-convo owning-window guard or last-write-wins for v1; **(3)** UI-bridge + permission prompts are broadcast (`ask_user`/`open_browser`/`notify`/`PERMISSION_EVENT`) → carry window label through the turn + `emit_to` instead of global `emit`; **(4)** custom titlebar → new windows need the `Titlebar` component + `core:window:allow-start-dragging` granted for non-`main` labels (see drag-region gotcha).
- **Route B — tear-off tab (true VSCode drag-out):** Route A + drag a tab out of the tabsbar → spawns a window pre-loaded w/ that convo + ownership transfer + per-window tab arrays + cross-window drag. Bigger lift, natural follow-up.
- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `app.windows` · [lib.rs](../src-tauri/src/lib.rs) (only `get_webview_window("main")` today; emits are global) · `assistant/turn.rs` + `assistant/bridge.rs` (global `emit` sites to scope per-window) · `state/assistant/persistence.ts` + `tabs.ts` (localStorage key namespacing) · `shell/Titlebar.svelte` + `capabilities/`.
- **Recommendation:** ship Route A first (complements, doesn't replace, in-window splits); Route B layers on. Load-bearing risks: shared-origin localStorage + global event/bridge broadcast — both solvable b/c backend keys by session.

#### 38. Per-pane STT routing + per-tab workspace root (✅ resolved in-tree 2026-06-14 cont.131)

- **Two split-pane bugs the user hit (the concrete #36 pains):**
  - **(a) STT landed in the wrong pane.** Dictation wrote to `assistant.composerDraft` — the *focused-pane* shim. A mic-button `onclick` fires before the pane-focus `onclick` bubbles, so `stt.start()` captured the OLD focused pane and text landed there. Same single-global flaw made voice "send it" fire every mounted composer.
  - **(b) Project folder was one global.** `cfg.current_root` is a single backend value; switching it in one pane changed it for all, so a freshly `/clear`ed or new pane inherited the just-switched folder. (cwd was *already* pinned per-session after the first turn via `save_session_cwd`; the leak was the global default + the UI display + the @-mention/branch probe.)
- **Fix (a) — STT target binding:** `stt.targetTabId` bound at `start(tabId)`; all draft reads/writes route through `readDraft/writeDraft` to that tab; composer passes its `tabId` (mic toggle + push-to-talk); the `sendRequested` effect gates on `stt.targetTabId === tabId`. [state/stt.svelte.ts](../src/lib/state/stt.svelte.ts) · [Composer.svelte](../src/lib/components/assistant/Composer.svelte).
- **Fix (b) — per-tab root:** `TabState.workspaceRoot` (canonical per-tab folder) + `assistant.effectiveRoot(tab)`/`activeRoot` (`tab.workspaceRoot ?? global`). Per-pane picker writes the tab via new backend cmd `assistant_set_tab_root` (canonicalize + record recent MRU, **no `current_root` mutation** → zero leak). `assistant_send` takes optional `root`, used on the first turn (then per-session pinned as before). `newTab` snapshots the focused tab's root; `clearConversation` preserves the pane's own root; disk-load hydrates `workspaceRoot` from `sessionCwd`. `@`-mention walk + branch probe scope to `activeRoot` (commands take optional `root`); caches invalidated on focus change. [turn.rs](../src-tauri/src/assistant/turn.rs) · [workspace.rs](../src-tauri/src/assistant/workspace.rs) · [state/assistant/{tabs,workspace,persistence}.ts] · [ChatTabsBar.svelte](../src/lib/components/shell/ChatTabsBar.svelte) · [AssistantWelcome.svelte](../src/lib/components/assistant/AssistantWelcome.svelte).
- **Verified:** `svelte-check` 0/0, vitest 132/132, Rust recompiled (CDP-invoked `assistant_set_tab_root` → canonical path), live CDP: set focused tab → exfil-v1, new tab inherited exfil-v1 (concrete snapshot, no live-global ref), restore → default; 0 console errors. **Untested:** real mic dictation (no CDP audio); true cross-pane non-leak (live state was single-pane — guaranteed by the snapshot design + inherit test). No new persistence needed — saved convos re-hydrate `workspaceRoot` from `sessionCwd` on load; unsaved tabs (never sent) already don't survive reload at all (`restoreTabs` keeps only convos in the meta list), so there's nothing to persist. Tabsbar `cwdMismatch` badge now compares pinned cwd vs the tab's `activeRoot` (was the global default → would've badged spuriously).

### Tier 4 — LOW / cosmetic

#### 29. CSP nonce nullifies `'unsafe-inline'` — inline styles blocked at runtime (✅ resolved in-tree 2026-06-11 — 🧪 needs prod-build verify)

- **Fix shipped in-tree:** the nonce came from **Tauri's asset-CSP rewriter**, not SvelteKit (no `kit.csp` configured). Added `"dangerousDisableAssetCspModification": ["style-src"]` to tauri.conf.json security — style-src keeps the static `'self' 'unsafe-inline'` (now actually effective), script-src nonce hardening untouched. Dev builds never exercised the rewrite, so verify on the next prod build: transitions animate + update progress-bar fills + zero CSP console violations.

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp` (`style-src 'self' 'unsafe-inline'`). At runtime SvelteKit injects a `nonce-…` into the served CSP.
- **Symptom (observed v0.8.14, prod CDP):** per CSP spec, **a nonce makes `'unsafe-inline'` be ignored** — so Svelte's dynamically-applied inline styles get blocked. Console spams `Applying inline style violates ... style-src 'self' 'unsafe-inline' 'nonce-…'`. Real impact: Svelte transition styles (fly/fade) and `style="width:{progress}%"` on the update download progress-bar don't apply. **Cosmetic** — download/apply and all clicks still work; animations snap and the progress fill stays empty.
- **Fix sketch:** make the static CSP and SvelteKit's nonce agree. Either (a) configure SvelteKit `kit.csp` so the nonce also covers the styles Svelte injects, or (b) drop the nonce path so `'unsafe-inline'` actually takes effect, or (c) move the affected inline styles to classes. **App-wide blast radius** — verify every transition + `style:` binding across the app before shipping; deliberately kept out of the v0.8.14 update-fix release to avoid re-breaking the updater.

#### 32. Ctx meter blank on restored conversations (✅ resolved in-tree 2026-06-11)

- **Fix shipped in-tree:** `buildSaveRecord` persists `lastTurnUsage` on the record (backend `Conversation.extra` flatten round-trips it, zero Rust changes); `loadConversation` hydrates it after `resetUsage()`. Block stays until `/git-ship`.

- **Where:** `ActivityPanel.svelte` ctx meter + composer gauge read `assistant.ctxTokensFor(tab)` → `tab.lastTurnUsage`, which is set only by live `recordTurnUsage` and never persisted by the conversation store.
- **Symptom:** a history-restored conversation shows no Context gauge (and 0% in the composer) until the first new turn completes — exactly when "how full is this old session?" matters.
- **Fix sketch:** persist the final turn's usage in conversation meta on save; hydrate `lastTurnUsage` in `loadConversation`. Found cont.113 (2026-06-11).

> Also parked: **Wave-1 LOWs #91–#134** — clippy/doc/perf nits, in the deleted `docs/archive/audit-history.md` (recover via `git log` if ever needed; not tracked live here).

---

## 🗄 Closed — kept for the record

### 33. Compaction tool broken (closed 2026-06-12 — feature removed)

- User report (v0.8.25/26): "the compaction tool does not work." Resolved by **removal, not repair** — the minimal-core strip (S2, buddy-release campaign) deleted the whole compaction subsystem (pipeline, auto-fire, UI, backend summarize/remint). Long chats → Ctrl+T fresh tab; the ≥70% ctx nudge survives with new copy. Legacy boundary pills in old saved conversations still render.

### 14. No CI — release path local-only (closed by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI is **not being pursued** — it only made sense bundled with code-signing, which was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Releases stay local via `scripts/release.ps1`. Reopen only if signing is reconsidered.

---

## Investigated 2026-06-05 — NOT bugs (don't re-chase)

- **Model-menu rows "don't switch on click"** — NOT a bug. Rows use `onmousedown` ([Composer.svelte:1209](../src/lib/components/assistant/Composer.svelte)) (fires before blur so the menu doesn't close first); a synthetic `click` simply doesn't trigger them. Real pointer + keyboard both work.

---

## Active design briefs

- `docs/design/assistant-mod-split.md` (#20 backend — COMPLETE R1-R8, shipped v0.8.16; kept as the split pattern reference)
- `docs/design/composer-split.md` (#20 frontend — **COMPLETE C1-C7 2026-06-10**; kept as the component-split pattern reference — the `composer/` child headers cite it)
- `docs/design/messagebubble-split.md` + `docs/design/chattabsbar-split.md` (optional quality follow-ups — both files already under threshold)
- `docs/design/assistant-svelte-split.md` (#20 frontend — COMPLETE, M0-M9 all shipped; KEPT permanently — the `src/lib/state/assistant/*` module headers reference it)

---

## Last full-app verification

- **🔍 Composer + effort live pass 2026-06-10 (cont.103) — all green.** Effort ladder retuned to mirror the CLI 1:1 (5 stops; Smart=`--effort high` default; Deep=`xhigh`; Ultracode=`xhigh`+workflows; Sonnet caps at Smart; Haiku hidden) and proven end-to-end: spawn log showed `model=sonnet effort=high` on a real turn. C3-C7 extractions each CDP pixel-verified live: queue chip mid-stream + ✓Steered flash + a visible mid-turn redirect (REDIRECTED in transcript), live pills `0:07 · 382 tok/s`, enhance panel (real Haiku rewrite, Ground/Diff toggles, Discard w/ draft intact), slash menu (15 cmds), @mention fuzzy pick, settings menu digit-pick / drag-to-Ultracode / Opus→Sonnet clamp, perm menu portal pick + outside-close. Known CDP wart: `c.sh look`'s console-error list accumulates since cdp:serve boot (mid-HMR ReferenceErrors linger) — trust the screenshot/state, not the stale error tail, or restart cdp:serve.
- **🔍 Post-strip CDP sweep 2026-06-12 (cont.119, v0.9.0 dev) — all green.** All 3 workspaces (Home 3-tile bento + live gauges · Chat · Settings w/ cut sections confirmed gone), 0 console errors. #34 live-verified via synthetic 20-file/200-edit injection; new ctx≥70% nudge copy + cwd-mismatch badge confirmed rendering.
