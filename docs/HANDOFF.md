# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.117) — Composer slim-down + open-issue sweep → SHIPPED v0.8.26

**Shipped v0.8.26** (feature `336a422`, release `d61dc62`, tag pushed → CI run 27379596782 queued at handoff — **verify green**). v0.8.25 release CI confirmed green earlier this session. Verified: cargo check clean · trust tests 2/2 · svelte-check 0/0 (4093 files) · vitest 122/122.

- **Composer slim-down (user: "why is it so damn big"):** pending rail no longer renders for bare streaming — only w/ queue chips OR (streaming && steerable draft/steerFlash); Beta disclaimer line deleted; wrap capped `min(--chat-col-max, 880px)`, padding 10/14→6/10, radius 18→14 (incl. streaming ::before).
- **Activity recap quote removed** (user disliked it): `lastTurn.preview` computation + markup + `.recap-preview` CSS gone; stat grid (duration/tools/files/cost) kept.
- **#30 resolved:** `assistant_session_cwd` cmd (convo_store.rs) + lib.rs registry · `TabState.sessionCwd` hydrated in `loadConversation` · warn `cwd-badge` in ChatTabsBar when pinned cwd ≠ workspace (normalized compare).
- **#29 resolved:** root cause = **Tauri asset-CSP rewriter** injects style-src nonce (NOT SvelteKit — no `kit.csp`); fix = `"dangerousDisableAssetCspModification": ["style-src"]` in tauri.conf.json. Prod-only behavior — **verify on v0.8.26 install**: transitions animate, update progress fills, no CSP console spam.
- **#12 resolved:** ToolChip chevron `fg-muted` + accent/nudge hover + Expand-details tooltip.
- **CR-UX resolved (user signed off via ask_user card):** trust enum collapsed `readonly|standard`. `is_valid_trust_level` rejects `full`; `effective_trust_level` + `mcp_server::trust_level()` migrate legacy `full`→`standard` read-side; turn.rs git-write gate `== "standard"`; `TrustLevel` type narrowed; tests updated (mod.rs + mcp_server.rs).

### RESUME HERE

- **LEAD ITEM (user-set): #33 compaction tool broken** — reproduce first (what exactly fails: summarize? remint? post-compact resume?), then `/diagnose` → fix + regression test → evaluate improvements. Entry points: `compaction.ts` + `oneshot.rs` summarize/remint. CI 27379596782 already verified green (v0.8.26 published).
- ~~Verify CI release 27379596782 green~~ ✅ done in-session → user installs v0.8.26 → live-test: composer slim look (no Working-rail idle-stream, no disclaimer, 880px) · cwd badge (open a chat resumed from another folder) · #29 transitions/progress-fill + zero CSP violations · tool-chip hover affordance · git tools still gated correctly at both trust levels. Plus carried v0.8.25 dictation live-tests (question stays question, masked cussing, "send it", PTT alt-tab, Ctrl+E, ctx meter on restore).
- **Permission-bar live-verify** now unblocked by CR-UX ship: pin trust=standard on a throwaway repo, fire a git_commit under "Ask before edits".
- **ISSUES remaining:** Auth-Rec (needs logged-out machine) · #31 blocking-fs (deferred-by-design) · **Fable dead-branch sweep after Jun 22** · #4 remainder (#7 charts · #11 inline diff · #13 tab strip · hover actions) · #17 (blocked) · Settings checklist · POLISH tier · SEC-1 · `browser_screenshot` MCP arc.
- Chat-page arc candidates: collapsible Activity sections, tooltip `.tip` glass transparency (app-wide).

## Prior arcs — detail in `git log` + CHANGELOG

cont.116 dictation data-fence + tracker cleanup → v0.8.25. cont.115 enhance wand v2 + dictation uncensored + PTT → v0.8.24. cont.113 Activity panel polish → v0.8.23. cont.112 UI/UX arc (Home bento, chat revamp, 1100px col). cont.111 full-codebase audit → v0.8.22. cont.109 bridge.rs loopback v0.8.21. cont.108 live plan limits v0.8.20. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Trust enum is now 2-level** (cont.117) — `full` must stay rejected for new writes but MIGRATE read-side (config + `RIFT_TRUST_LEVEL` env); don't "clean up" the migration arms.
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175. **Spine-node icons stay opaque**.
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.26 stands.**
- **`turn.rs::kill_all_session_children` re-export** (sweeps `oneshot::ENHANCE_PIDS`) + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
