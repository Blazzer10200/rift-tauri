# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.106) — app-wide right-click menus + Fable ctx fix + model-menu reorg

svelte-check 0/0 (4092 files) · vitest assistant 23/23 (1 new) · live CDP E2E (all menu surfaces, real paste, pixel shots) · 0 console errors.

- **Custom context menus everywhere; stock WebView2 menu suppressed.** New `state/contextMenu.svelte.ts` (rune store + global fallback builder) + `shell/ContextMenuHost.svelte` (portal, viewport clamp, mousedown/Esc/blur dismiss), wired in `+layout.svelte` via `<svelte:document oncontextmenu={handleGlobalContextMenu}>`. Fallback surfaces: edit fields Cut/Copy/Paste/Select-all (live disabled states) · text selection Copy · `<pre>` Copy code · links Open-in-browser/Copy-address · empty background = no menu. `MessageBubble` adds Copy message / Copy selection. **Convention: a component owns a right-click by calling `e.preventDefault()`** — global handler skips `defaultPrevented` events (existing OpenInPaneMenu on tabs/history untouched, coexists). Shift+right-click in dev = native menu (Inspect element).
- **`tauri-plugin-clipboard-manager` added** (Cargo dep + `lib.rs` init + npm pkg + `clipboard-manager:allow-read-text` capability) — Paste needs clipboard *read*, permission-gated in WebView2 via navigator API. E2E proven: `Set-Clipboard` → menu Paste click → text landed in composer w/ `bind:value` intact (setRangeText + synthetic `input`).
- **Fable ctx-window fix:** `ctxWindowFor` (assistant.svelte.ts) had no `fable-5` pattern → 200K denominator on a 1M model (header meter wrong AND auto-compact would fire ~5× early). Added to the 1M branch + regression test.
- **Model menu reorg (user picked layout via preview):** two-line rows — name+badge top, new `blurb` field (modelMatrix.ts) + right-aligned ctx column below; full taglines remain hover tooltips. SettingsMenu row restructured on the existing `.rift-menu-row-body` pattern.

### RESUME HERE

- **cont.106 work committed, NOT shipped** — next `/git-ship` bundles it (version bump ×3 + CHANGELOG happen there, not before). v0.8.18 stands as last tag.
- v0.8.18 release CI verified green (cont.106). User prod = 0.8.12 → still needs ONE manual Setup.exe onto the Velopack train.
- Next bites: audit remainder (#7 charts · #12 chip affordance · #11/#13 design passes · `/history` + hover-actions checks), then Settings per-page checklist.
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum · consider rotating VM Administrator password (runner-scratch cleanup, cont.105).

## Prior arcs — detail in `git log` + CHANGELOG

cont.105 #4 UI sweep, 9/13 audit findings (shellLabel cd-strip · SlashMenu palette grammar · per-chat model scoping `TabState.modelOverride`+`effectiveModel` · Home/Welcome snippets via `ConversationMeta.last_snippet`) + **v0.8.18 shipped** (annotated tags enforced). cont.104 Rail-v2 (per-chip steer/queue, next-turn inject) + turn.rs registry race fix (`clear_session_pid_if`/`clear_steer_tx_if`). cont.103 effort ladder CLI 1:1 (smart=`--effort high` default) + composer split C1-C7 (no file >2000L). cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Effort mapping lockstep:** `effortToFlag` (helpers.ts) ↔ `turn.rs` match arm ↔ `modelMatrix.ts` tables — change all three together; the vitest mirror test guards it.
- **Right-click ownership:** component context handlers MUST `preventDefault()` or the global fallback double-fires on top of them.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.18 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
