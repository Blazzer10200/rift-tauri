# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-06 (cont. 60) — Browser arc finished + render polish (unshipped)

Completed the in-app browser dock. CDP-live-verified; `svelte-check` 0/0; backend verified via live event pipeline (dev console = Rust verifier, dev was running — no `cargo check`). Files: [browser/mod.rs](../src-tauri/src/browser/mod.rs), [commands/browser.rs](../src-tauri/src/commands/browser.rs), [lib.rs](../src-tauri/src/lib.rs), [WebBrowserPage.svelte](../src/lib/components/webview/WebBrowserPage.svelte), [browserDock.svelte.ts](../src/lib/state/browserDock.svelte.ts), [AppShell.svelte](../src/lib/components/AppShell.svelte).
- **Functions:** Enter-to-go (was wired) · **true reload** (`browser_reload`→`location.reload()`, replaces re-`navigate()` that polluted Back) · **Ctrl+L** focus+select (`browserDock.focusAddress()`+`focusToken`, gated chat) · select-all on focus · Copy URL + Open-external in a **`⋯` menu** (`.rift-menu`+portal, scrim+Esc).
- **Bar 8→6 buttons** (dropped Go arrow): `[←][→][⟳] [address] [Add to chat] [⋯] [✕]`.
- **Loading spinner on ALL navs:** native `on_page_load`→emits `browser://load {phase,url}`; frontend `listen()` drives `loading`+address; 20s watchdog.
- **Render polish:** native webview `.background_color(Color(9,10,11,255))` = `--bg` #090a0b (kills pre-paint flash); stage `#0b0b0d`→`var(--bg)`. Page renders fine (read_page=202 chars); CDP can't capture native child webviews — black in shots ≠ bug.
- **Decisions:** Back/Fwd grey-out SKIPPED (Tauri doesn't expose WebView2 `CanGoBack`). MCP `read_page`/`open_url` bridge DEFERRED — MCP server is a SEPARATE PROCESS ([lib.rs:102](../src-tauri/src/lib.rs#L102), no `AppHandle`) → cross-process IPC, own session. Repaint nudge HELD (flicker risk).

cont.59 (browser model's-eyes: `read_page`→Add-to-chat, omnibox) folded in — arc complete. cont.58: #36 scroll-spy + ISSUES reorg.

### RESUME HERE (cont.60)
- All cont.52–60 UNSHIPPED → next `/git-ship` (3-file lockstep + `Cargo.lock`; CHANGELOG/bump deferred). Browser arc is shippable.
- **PENDING USER EYEBALL:** browser render-flash fix + does page show content cleanly (native webview CDP-invisible — only user can confirm).
- ISSUES **#31–36** fixed in-tree (delete on ship).
- **Open work all blocked / needs-you / live-verify:** #21 test harness (T1) · #30 Update-UI redesign (taste — CDP-drivable via `window.__updates`) · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum (sign-off).
- Pending USER live-verify: steer mid-turn on a tool turn · permission Allow/Deny bar · v0.5.0 auto-update on a real machine · beta onboarding on a fresh tester install.
- CDP wrapper (`npm run cdp:serve`) left running.

---

## Prior unshipped (detail in git log, all ride next ship)
- **cont.57:** Model-picker capability matrix (`Composer.svelte` `ModelOpt`) — per-model `effortStops`+`$effect` auto-clamp, amber Ultracode caption, fast-mode behind `FAST_MODE_WIRED=false` (→#31). **cont.55–56:** Harness motion polish + beta-notice onboarding step (see CRITICAL gate). **cont.52–54:** `Markdown.svelte` streaming reveal · Harness dead-wait `.tl-dead` · ISSUES #32–35 + KPI no-scroll redesign.

---

## Shipped + prior arcs — detail in `git log`
- **v0.5.0** (2026-06-04, cont.51, `62dae27`): Velopack stable to `rift-releases`. **Pending live-confirm:** 2nd auto-update proof point on a real machine.
- **Carry-over:** `check.yml` per-push email spam; prod app now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = sidebar + 5 sections; Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.5.0 stands** (shipped 2026-06-04, cont.51).
