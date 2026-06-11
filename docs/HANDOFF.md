# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.113) — Activity panel polish → SHIPPED v0.8.23

**Shipped v0.8.23** (`0cbcfa9` → tag → CI release green, assets published to rift-releases). Feature commit `b6dfabe`, verified (svelte-check 0/0 · vitest 122/122 · live CDP 0 console errors). All in `ActivityPanel.svelte`:

- **MCP steps humanized** — `classifyTool` now parses `mcp__<server>__<tool>`: rift `read_file/list_dir/grep/ask_user/notify/open_browser` get verbs + payload targets + icons (HelpCircle/Bell/Globe); `git_*`→shell; generic fallback stops echoing raw id twice. **ask_user input is nested: `{questions:[{question,…}]}`** (mirrors AskUserQuestion) — flat `s("question")` reads empty.
- **Turn separators** in Steps ("TURN N ─ ago") when log spans >1 turn; rows carry `turn` (assistant-msg counter; agentSpawns slotted by startedAt).
- **Sources section** — deduped url/query tool inputs, click → open/copy; wires up previously-dead `openSource`.
- **Last-turn recap** — 2-line reply preview w/ accent quote bar (md stripped); stat grid aligned to 14px section padding (was flush-left).
- **Opaque icon tints** — `--accent-soft`/`--danger-soft` are 12%-alpha; spine bled through ask/write/pending/error icon boxes → swapped to `color-mix(in oklab, … 14%, var(--bg-elev-2))`. User-reported on hover.
- Empty-state copy restructured w/ title line.

**Finding (not fixed):** ctx meter blank on restored convos — `lastTurnUsage` is live-only, never persisted; filed as **ISSUES #32**.

### RESUME HERE

- **Chat-page "clean + user-friendly" arc continues.** Activity panel now done-ish (recap+preview, humanized steps, turn seps, Sources). Open candidates: collapsible sections w/ counts, Tool-mix/Insights sections (header comment's old promises — comment now updated to reality), tooltip `.tip` glass transparency (user noticed; app-wide call).
- User prod app still needs one manual Setup.exe (pre-Velopack) — unchanged.
- **ISSUES #31** (legacy `base_url`/`provider_model` cmds · turn.rs 401-dup helper · blocking-fs-in-async · **Fable dead-branch sweep after Jun 22**) + **#30** (reload-while-on-chat init race, semi-repro) + **#32** (ctx meter on restore).
- CDP warts: synthetic `.click()` doesn't fire row handlers; wrapper accumulates console errors across reloads — restart `cdp:serve` before trusting `[errors]` (did this session; procedure works).
- Carried: `browser_screenshot` MCP design arc · #7 charts · #12 chip affordance · #11/#13 · Settings checklist · POLISH tier · SEC-1 · #29 CSP-nonce · CR-UX.

## Prior arcs — detail in `git log` + CHANGELOG

cont.112 UI/UX arc (Home Mission-Control bento + `targetHarnessSubtab` deep-link · chat revamp w/ right-aligned user bubbles + merged thought rows + 1100px col · Activity idle recap, DOCK_DEFAULT 340). cont.111 full-codebase audit → shipped v0.8.22. cont.110 multi-tab stream-kill fix + Harness mission control. cont.109 bridge.rs loopback v0.8.21. cont.108 live plan limits v0.8.20. cont.106 custom context menus + Fable 1M fix. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175. **Spine-node icons stay opaque** (no `-soft` fills in ActivityPanel rows).
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.23 stands.**
- **`turn.rs::kill_all_session_children` re-export** + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
