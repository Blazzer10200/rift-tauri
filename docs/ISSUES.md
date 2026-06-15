# Rift — Issue Tracker

> **Single source of truth for open work only.** When something ships, **delete its block** — `git log -- docs/ISSUES.md` preserves history. Issue IDs are **durable**: never re-number, only append. Each block carries `Where` (file:line — may have drifted, re-grep before acting), `Symptom`, and an optional `Fix sketch`.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables lived in `docs/archive/audit-history.md` — archive deleted 2026-06-09 (`1810c2e`), recover via `git log`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here. **Pruned 2026-06-15 (cont.139):** six "✅ resolved in-tree" blocks (#34, CR-UX, #29, #30, #32, #38) all shipped at/under `v0.11.0` (HEAD) — verified in-tree, blocks deleted per the ship-delete rule; recover via `git log`.

### Conventions

- **Status** — `✅ resolved in-tree` (fixed, unshipped — block stays until `/git-ship` so `git log` keeps it) · `🚧 open` · `👤 needs your call` · `🧪 live-verify` (code-complete, needs runtime confirmation) · `🔒 blocked` (external dep) · `🗄 closed` (decided, kept for the record).
- **Tier** — `T1` ship-blocker / data-safety · `T2` code-complete needs live-verify · `T3` strategic / longer-term · `T4` LOW / cosmetic.

### Index

| ID | Title | Tier | Status |
|----|-------|------|--------|
| Fable-Off | Fable 5 manually pulled (US-gov disablement) — flip both kill-switches to re-enable | T2 | 🔒 disabled in-tree (revert-when-re-enabled) |
| Auth-Rec | In-app sign-in recovery for 401 failures | T2 | 🧪 live-verify (real logged-out sign-in untested) |
| Permission | Allow/Deny round-trip bar | T2 | 🔒 blocked (trust gate) |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| #31 | Deferred remainder from the 2026-06-11 dead-code/debug audit | T4 | 🚧 open (trimmed — most sub-items shipped) |
| #35 | Live shell + sub-agent streaming output panel | T3 | 🚧 open (idea) |
| #36 | Split-pane feature overhaul | T3 | 🚧 open (idea) — dividers + per-pane head done; vertical/grid splits remain |
| #37 | Multi-window — separate OS windows (VSCode-style, multi-monitor) | T3 | 🚧 open (idea) |
| #39 | UI-consistency review + full-app audit 2026-06-15 — deferred findings | T3/T4 | 🚧 catalogued (P0-4 shipped; rest deferred) |
| #40 | STT end-of-dictation polish UX — long shimmer + flash | T3/T4 | ✅ resolved in-tree (v0.12.1) |
| #41 | Split-pane send routed to wrong pane | T1 | ✅ resolved in-tree (v0.12.1) |
| #33 | Compaction tool broken | T1 | 🗄 closed (feature removed in minimal-core strip) |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## 🚧 Open issues

### Tier 2 — operational / revert-when-unblocked

#### Fable-Off. Fable 5 manually disabled (US-gov disablement 2026-06-14 — temporary)

- **What:** Fable 5 was pulled by US-gov action. Rather than rip the code out, it's gated off behind a manual kill-switch that reuses the existing sunset machinery (hides the picker row, coerces any stored/pinned Fable pref → default, backend swaps a pinned Fable session → opus before it can hit the API).
- **To re-enable when it comes back:** flip BOTH flags to `false` — `FABLE_DISABLED` in [state/assistant/helpers.ts](../src/lib/state/assistant/helpers.ts) (frontend) and `FABLE_DISABLED` in [src-tauri/src/assistant/config.rs](../src-tauri/src/assistant/config.rs) (backend). The date-based `FABLE_SUNSET_MS`/`fable_sunset_passed()` still applies underneath, so re-enabling only restores Fable through its original Jun 22 sunset.
- **Why both sides:** frontend hide alone wasn't enough — a persisted *session pin* (`load_session_model`) can re-inject `claude-fable-5` on resume, bypassing the picker; the backend guard (`fable_unavailable()` in turn.rs) catches that.
- **Sweep on hold:** keep every Fable branch (`fableAvailable()`/`fable_unavailable()` gates, `fableSunsetNoticed` toast, `limited` rows, the type-union member, price entry) so re-enabling stays a two-flag flip. Only sweep if Fable is confirmed *permanently* gone.
- **Verified:** svelte-check 0/0 · vitest (fable test split into disabled/enabled cases, self-heals on re-enable) · cargo check clean. Shipped `13b7c80` (v0.10.0).

#### Auth-Rec — in-app sign-in recovery for 401 failures (🧪 live-verify)

- **Status:** shipped in v0.8.9+ (`9c468a4`+`2d72af8`) — `assistant_open_login(console)` spawn + actionable 401 banner ([Sign in]/[Open Settings]/[Re-check]). CDP-verified all banner states; the live login spawn itself is compile/registration-verified only. **v0.9.3 (RR-1):** the same Sign-in/Re-check now also lives on the `needsAuth` welcome card (`AssistantWelcome.svelte`) — closes the new-user dead-end where a red-pill user could never reach the post-turn banner (send is disabled, so no turn fires).
- **Remaining:** confirm an end-to-end real sign-in on a genuinely-logged-out machine (dev box stays authed). **Strategic follow-ups** (not built): proactive re-probe before first send; auto-prefer an authed install when multiple exist; collapse scattered 401 string-matching into one `AuthError` enum + DiagBus telemetry.

#### Permission — Allow/Deny round-trip bar (🔒 blocked on a trust-standard workspace)

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Live-verify attempt 2026-06-10 (cont.103):** switched to "Ask before edits" + asked for a `git_commit` in a derived-trust workspace — the MCP server correctly **doesn't expose git-write tools at derived trust**, so the prompt can never fire there. That confirms the trust gate works, but the bar itself remains unexercised. Verifying requires pinning `trust_level=standard` on a throwaway repo — deliberately not done unattended because the trust segment **pins one-way**. Verify the bar whenever a trust-standard throwaway repo is next exercised.

### Tier 3 — strategic / longer-term

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Progress (cont.105, 2026-06-10):** audit findings **#1-#6 + #8-#10 SHIPPED** and live-verified — Steps-rail `cd`-strip, slash menu → palette design language, empty-dock auto-collapse, scroll bottom padding, **per-chat model scoping** (`TabState.modelOverride` + `effectiveModel`), jump-back-in snippets + model chips, KPI zero-state unify, user-turn inset card, insight severity stripes. **cont.138 (v0.11.0):** shared `PageHero`, Home quick-actions card, nav experimental-dot + Settings tooltip.
- **Remaining from the audit:** #7 cost-chart sparse-data polish · #11 rich inline diff (design pass) · #13 tab strip into titlebar (lowest priority) · message hover actions discoverability. Then the per-page Settings checklist.
- **Input:** [ui-audit-2026-06-09.md](design/ui-audit-2026-06-09.md) · [ui-review-2026-06-15.md](design/ui-review-2026-06-15.md).

#### 31. Deferred remainder from the 2026-06-11 dead-code/debug audit (trimmed — most shipped)

- ✅ **Blocking fs reads in async commands:** `read_oauth_token()` in `usage_rate_limits` now uses `spawn_blocking` (this session). `load_config()` in `assistant_send` reads before the first yield — acceptable for a tiny local file.
- **Optional split follow-ups (quality, not threshold):** [messagebubble-split.md](design/messagebubble-split.md) B1-B6 + [chattabsbar-split.md](design/chattabsbar-split.md) T1-T6 stay mapped; `assistant_send` (large fn inside turn.rs) can split internally later. Both target files already under threshold.
- *(Shipped/superseded sub-items removed: legacy provider commands superseded by the minimal-core strip; 401-detection dedupe shipped `is_auth_rejection()` 2026-06-11; Fable sweep folded into Fable-Off above.)*

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### 35. Live shell + sub-agent streaming output panel (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** a Chat-page feature that lets users *watch what's happening under the hood* the way Claude Code desktop's pane menu does. Two concrete asks: **(1) sub-agent activity** — what each spawned sub-agent is actually doing as it runs, not just "N agents"; **(2) terminal I/O, both directions** — the live back-and-forth with a shell, whether *the user* drove it or a *sub-agent* did. Today Rift only shows *counts* (composer LivePills render `▸ N shells` / `◍ N agents`).
- **Note:** a **live sub-agent activity dock** shipped `a3ab764` (v0.10.0) — `parent_tool_use_id` routing in `streaming.ts`/`SubAgentDock.svelte`/`activityDock.svelte.ts`. That partially addresses ask (1). The terminal-I/O half (2) is still open.
- **Why the rest is non-trivial — the data may not exist yet:** the CLI streams tool_use/tool_result envelopes; a `Bash` tool result arrives as one final block, not incremental stdout. Real *live* shell output would need either (a) the CLI to stream partial Bash output, or (b) routing long-lived/background Bash through Rift's own MCP/PTY so we own the stream. Per the `agentSpawns` comment, "CLI does NOT stream intermediate sub-agent activity — we only know spawn + final result."
- **Scope decision needed:** confirm what the CLI actually streams before designing more UI — a panel that can only show final output isn't the ask.

#### 36. Split-pane feature overhaul (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** "the split pane feature could be way better improved." General dissatisfaction.
- **What exists today:** N-pane split inside the single window — `panes: PaneState[]`, drag-a-tab-onto-a-half to split, `MAX_PANES` cap, `focusedPaneIdx`, per-pane composer-draft stash/restore. State on the one `AssistantStore`, persisted to `localStorage` `rift.ui.tabs.v1`.
- **Where:** [state/assistant/tabs.ts](../src/lib/state/assistant/tabs.ts) · `PaneState`/`MAX_PANES` in `state/assistant/types.ts` · `AssistantPane.svelte`.
- **Already DONE:** per-pane STT routing + per-tab workspace root (shipped `ca5db9d`, v0.10.0) · legible in-flow `.pane-head` strip with conversation title (shipped `3b28567`) · resizable dividers (drag, double-click reset, arrow-key adjust — `AssistantPage.svelte`). Don't re-scope these as new.
- **Still un-prioritized:** vertical/grid splits (2×2, not just a row) · drag-pane-to-reorder · drag a pane out → its own window (overlaps #37). **Keybinds are OUT** (user doesn't use them). **`MAX_PANES` stays 4** (DECIDED NO 2026-06-14 — 4×320px = 1280px; a 5th makes panes unusably narrow). Ask for specifics before the bigger overhaul.

#### 37. Multi-window — separate OS windows (idea — user-requested 2026-06-14)

- **Want (user, 2026-06-14):** VSCode-style — open Rift sessions as **separate native windows** so each can live on a different monitor. Distinct from #36 (splits *within* one window).
- **Feasibility: YES.** Tauri 2 supports multiple `WebviewWindow`s natively. Tractable because the **Rust backend already partitions by session** — turn registry keyed by cliSessionId; stream/permission/error events are **broadcast app-wide** (`app.emit`) carrying the session id, and the frontend filters by id.
- **Route A — "New Window" MVP (a session per monitor):** spawn a second `WebviewWindow` (same app URL, unique label); it boots its own store. Must-fix gotchas: **(1)** shared origin → both windows stomp `rift.ui.tabs.v1` → namespace the persistence key per window label; **(2)** same convo open in 2 windows = disk save race → per-convo owning-window guard or last-write-wins; **(3)** UI-bridge + permission prompts are broadcast → carry window label through the turn + `emit_to` instead of global `emit`; **(4)** custom titlebar → new windows need the `Titlebar` component + `core:window:allow-start-dragging` granted for non-`main` labels.
- **Route B — tear-off tab (true VSCode drag-out):** Route A + drag a tab out of the tabsbar → spawns a window pre-loaded w/ that convo + ownership transfer + per-window tab arrays + cross-window drag. Bigger lift, natural follow-up.
- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `app.windows` · [lib.rs](../src-tauri/src/lib.rs) · `assistant/turn.rs` + `assistant/bridge.rs` (global `emit` sites) · `state/assistant/persistence.ts` + `tabs.ts` · `shell/Titlebar.svelte` + `capabilities/`.
- **Recommendation:** ship Route A first; Route B layers on.

### Tier 3/4 — catalogued audit findings

#### 39. UI-consistency review + full-app audit 2026-06-15 — deferred findings (catalogued, not blind-fixed)

High-confidence, safe items were fixed in `086e403` (MIME allowlist · MODEL_LABELS dedupe · 2 aria-labels) and the v0.11.0 UI batch (shared `PageHero` [P0-2], Home + nav [P1]). **cont.139 (PR #5 + this session):** helper dedup, security hardening, token cleanup. Remaining items catalogued below.

**Top of the backlog (deferred — needs desktop CDP eyeball):**
- ✅ **P0-4 — double live timer in the assistant turn head (resolved v0.12.1).** While streaming, the role-row **heartbeat** (`9s`) ticked one line above the per-thinking-block timer (`Thinking for 6s`) → two stacked live counters. **Fixed:** `MessageBubble.svelte:334` now shows the `live-dot` (not the heartbeat) while `hasActiveThinking` — the thinking pill carries the only ticking number during the thinking phase; the heartbeat returns for tool/text phases. One live timer at all times.
- **P0-3 — unify the CLI-update notice.** Surfaced 3 ways (Home `.dash-cli` banner, ChatTabsBar `.cli-badge` pill+popover, Settings inline button) with independent dismiss. Extract one `UpdateNotice` + a single shared dismiss state. ⚠️ touches the **Velopack** update-notification path, so verify carefully.
- **P2 — shared size tokens.** Tab 36px / tab-item 26px / settings sub-tab 42px / Home buttons 38px each hardcoded → introduce `--control-h` / `--tab-h`. Rename the dual-meaning `.sb-bento` (column in Settings vs 2-col grid in Local LLM) so a third page can't inherit the wrong layout.

**Security (remaining — low real-risk; mostly same-user or by-design):**
- ✅ `local_llm_base_url` → `ANTHROPIC_BASE_URL` — http(s) + host validation added at setter + turn.rs sink (PR #5).
- ✅ `convo_store.rs assistant_export_save` — extension allowlist (.md/.json/.txt) added (this session).
- ✅ `git_local.rs` GIT_CONFIG_GLOBAL/SYSTEM env vars now stripped (this session).
- ✅ `capabilities/default.json` `opener:allow-open-url` — dropped `http://**`, https-only (this session).
- `mcp_server.rs` read_file/list_dir TOCTOU symlink race (acknowledged in-code). (low, same-user)
- `bridge.rs` ephemeral port is 192-bit-token-gated but has no connection-rate-limit. (low)
- **NOT a bug:** CSP `connect-src https://registry.npmjs.org` is the legit CLI-update npm version check; `open_in_vscode` Unix arg is shell-free (OS-quoted). Verified — leave.

**UI/UX token consistency (remaining — cosmetic; batch with live CDP eyeball):**
- ✅ Dead color-var fallbacks stripped (`var(--danger,#e66)`, `var(--warn,#e2b340)`) — PR #5.
- ✅ `EnhanceBar`/`AssistantWelcome` text-on-accent → `var(--accent-fg)` (this session).
- ✅ `ToolChip` `var(--ok, ...)` fallback dropped (this session).
- `Markdown.svelte:842` `#22272e` — intentional Shiki github-dark-dimmed match; `--bg-inset` resolves lighter so left as-is.
- `ToolChip.svelte` terminal bg/text oklch literals — intentional terminal-look design; left as-is.
- Off-token radius (7px/11px/16px vs 6/10/12 scale) + font-size literals (9–10.5px below `--fs-xs`).
- Scrollbar `scrollbar-width:thin` overrides are **no-ops** in WebView2 — remove or commit to one style (your call).

#### 40. STT end-of-dictation polish UX — long shimmer + end-of-phrase flash (✅ resolved v0.12.1)

User report (Web Speech engine, default): "at the end of speech-to-text it shows my entire phrase, then keeps flashing for so long." All four sub-causes fixed in [stt.svelte.ts](../src/lib/state/stt.svelte.ts) + [Composer.svelte](../src/lib/components/assistant/Composer.svelte):

- ✅ **40a — shimmer could run up to 15s.** `polishWebSpeechFinal()` pulsed `.textarea-wrap.polishing` for the entire `stt_clean_transcript` (Haiku) call, capped only by the backend's 15s `CLEANUP_TIMEOUT` ([cleanup.rs:28](../src-tauri/src/stt/cleanup.rs)). **Fixed:** a 6s frontend `SHIMMER_CAP_MS` timer drops the visual early; the cleaned-text swap still lands if the call beats the cap (the raw transcript is already committed + editable).
- ✅ **40b — typing didn't stop the shimmer.** Added `stt.cancelPolish()` (bumps a `polishGuard` token + clears `polishing`); the composer `oninput` now calls it, so typing kills the pulse instantly and invalidates the late swap.
- ✅ **40c — full-transcript pulse read as "loading."** Mitigated by 40a/40b — the pulse is now bounded and dismissible. (A scoped "polishing…" chip instead of animating the text is a possible future polish, not done.)
- ✅ **40d — multiple full-textarea rewrites at stop.** `onEnd()` now skips its `composeDraft` rewrite when the draft already equals the streamed final (no delta), killing the end-of-phrase flash-in.
- **Scope note:** Whisper engine polishes backend-side (`stt://final` arrives pre-cleaned) and never sets `polishing`, so the shimmer was Web-Speech-only.

#### 41. Split-pane send routed to the wrong pane (✅ resolved v0.12.1)

User report: "in split-pane, the message I send in pane 1 lands in the other pane." **Root cause:** `send()` ([send.ts:26](../src/lib/state/assistant/send.ts)) and every activeTab-scoped getter it reads (`streaming`/`queue`/`composerAttachments`/`effectiveModel`) key off the global `currentConvoId`, but the per-pane composer fired `assistant.send(text)` with **no** tabId — so the turn targeted whichever pane was focused/active, not the firing pane. The old `onsubmit` did `if (!focused) setFocusedPane(paneIdx)` first, but that left a desync window (and `setFocusedPane`'s async `loadConversation` branch doesn't set `currentConvoId` synchronously). Drafts/attachments were already pane-correct (keyed to `tabId`); only the send entry was wrong. **Fixed:** `assistant.send(prompt, tabId?)` ([assistant.svelte.ts:1299](../src/lib/state/assistant.svelte.ts)) now retargets `currentConvoId` to the firing pane's tab synchronously (via `setFocusedPane` on the matching pane index — the composer only renders for a loaded tab, so the async load path is never hit) before `sendImpl`. `AssistantPane` `onsubmit` passes its `tabId`. Verified: svelte-check 0/0, 162/162 vitest, 0 console errors live.

---

## 🗄 Closed — kept for the record

### 33. Compaction tool broken (closed 2026-06-12 — feature removed)

- User report (v0.8.25/26): "the compaction tool does not work." Resolved by **removal, not repair** — the minimal-core strip (S2, buddy-release campaign) deleted the whole compaction subsystem (pipeline, auto-fire, UI, backend summarize/remint). Long chats → Ctrl+T fresh tab; the ≥70% ctx nudge survives with new copy. Legacy boundary pills in old saved conversations still render.

### 14. No CI — release path local-only (closed by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI **is** in place now (tag-driven `release.yml` → rift-releases). Code-signing was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Reopen only if signing is reconsidered.

---

## Investigated 2026-06-05 — NOT bugs (don't re-chase)

- **Model-menu rows "don't switch on click"** — NOT a bug. Rows use `onmousedown` (fires before blur so the menu doesn't close first); a synthetic `click` simply doesn't trigger them. Real pointer + keyboard both work.
- **Jump back in doesn't navigate** (audit suspicion) — NOT a bug, verified live.

---

## Active design briefs

- `docs/design/assistant-mod-split.md` (#20 backend — COMPLETE R1-R8, shipped v0.8.16; kept as the split pattern reference)
- `docs/design/composer-split.md` (#20 frontend — COMPLETE C1-C7; kept as the component-split pattern reference)
- `docs/design/messagebubble-split.md` + `docs/design/chattabsbar-split.md` (optional quality follow-ups — both files already under threshold)
- `docs/design/assistant-svelte-split.md` (#20 frontend — COMPLETE, M0-M9 all shipped; KEPT permanently — the `src/lib/state/assistant/*` module headers reference it)
- `docs/design/ui-review-2026-06-15.md` (#39 backlog source)

---

## Last full-app verification

- **🔍 Post-strip CDP sweep 2026-06-12 (cont.119, v0.9.0 dev) — all green.** All 3 workspaces (Home 3-tile bento + live gauges · Chat · Settings w/ cut sections confirmed gone), 0 console errors. #34 live-verified via synthetic 20-file/200-edit injection; new ctx≥70% nudge copy + cwd-mismatch badge confirmed rendering.
- **🔍 UI consistency batch 2026-06-15 (cont.138, v0.11.0) — green.** svelte-check 0/0 (4094 files); Settings + Local LLM heroes live-CDP-verified on shared `PageHero`.
