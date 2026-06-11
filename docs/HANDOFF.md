# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.112) — UI/UX arc: Home bento · chat revamp · Activity recap

**v0.8.22 CI release confirmed green** (closes cont.111's resume item). Three commits, each verified (svelte-check 0/0 · vitest 122/122 · live CDP 0 console errors):

- **`f877927` Home Mission-Control bento** — full-width tile grid (1560px): workspace tile · today/month/burn KPI minis w/ inline budget bar · 14-day sparkline · 2-col Jump-back-in · right rail = plan-limit gauges + top-3 insights. New one-shot `workspace.targetHarnessSubtab` + `openHarness()` deep-links sparkline → Harness>Cost (consumed by HarnessPage `$effect`, same pattern as `/history`). ≤1240px collapses to scrolling stack (visually untested — CDP can't resize OS window). `home/UsageCard.svelte` was created then folded into tiles + deleted same session.
- **`28218c3` Chat thread revamp** — user turns = right-aligned accent-tinted bubbles (tail 14/14/4/14, 62ch cap, no avatar/"You"/rail; copy via context menu). **Supersedes ui-audit #9 left-aligned-user decision — user-approved.** Consecutive done-thinking blocks fold into one "Thought for Ns" row (summed duration, joined text — in MessageBubble `grouped`). `--chat-col-max` un-drifted 900→1100px (comment always said 1100); thread/composer/alerts widen in lockstep. Kept: Rail-v2 assistant rail + step dots (cont.104).
- **`84d895f` Activity panel idle recap** — "Last turn" stat card (duration/tools/files/cost from final assistant msg) leads idle panel after Done strip clears; `DOCK_DEFAULT` 300→340 (now exported; AssistantPane reset + CSS lockstep).

**#30 semi-repro:** workspace chip → "Open project" + empty tab strip after a frontend `location.reload()`; app state stays healthy; NEXT reload restored both. Transient init race, not persistent — repro path is reload-while-on-chat.

### RESUME HERE

- **Next arc (user's stated goal): chat page "clean + user-friendly as possible"** — continue Activity panel polish (idle empty-state copy is still weak; consider collapsed Outputs/Sources w/ counts). User signed off on everything shipped this session.
- User prod app still needs one manual Setup.exe (pre-Velopack) — unchanged.
- **ISSUES #31** (audit deferred: legacy `base_url`/`provider_model` cmds · turn.rs 401-dup helper · blocking-fs-in-async · **Fable dead-branch sweep after Jun 22**) + **#30** (now w/ repro note above).
- CDP wart: synthetic `.click()` doesn't fire row handlers (welcome recents, tabsbar rows) — top-level nav buttons OK. CDP wrapper accumulates console errors across reloads — restart `cdp:serve` for clean buffer before trusting `[errors]`.
- Carried: `browser_screenshot` MCP design arc · #7 charts · #12 chip affordance · #11/#13 · Settings checklist · POLISH tier · SEC-1 · #29 CSP-nonce · CR-UX.

## Prior arcs — detail in `git log` + CHANGELOG

cont.111 full-codebase audit + cleanup → shipped v0.8.22 (diagnostics slim, `/history` fix, poison-safe CACHE locks, dedups; 3-agent audit). cont.110 multi-tab stream-kill fix + Harness mission control. cont.109 bridge.rs loopback (ask_user/open_browser/notify) v0.8.21. cont.108 live plan limits v0.8.20. cont.106 custom context menus + Fable 1M fix. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.22 stands.**
- **`turn.rs::kill_all_session_children` re-export** + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
