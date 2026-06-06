# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-06 (cont. 63) — SHIPPED v0.6.1: CLI multi-install + unified update UI

Shipped the cont.62 CLI multi-install backend plus this session's update-UI redesign as **v0.6.1**.
- **CLI multi-install (cont.62):** `enumerate_claude_installs()` ([assistant/mod.rs](../src-tauri/src/assistant/mod.rs)) finds EVERY claude (PATH + native sites + npm bundled exe + `.cmd` shims, deduped shim→exe), newest wins, `assistant_update_cli` updates ALL (npm once, native per-exe). `ClaudeInstall` DTO on `AuthStatus.installs`. Settings lists installs w/ active/behind tags + per-row method-aware copy; "still behind" hint on a native no-op.
- **Update UI redesign (this session):** one `cliUpdate.summary(installs)` source ([cliUpdate.svelte.ts](../src/lib/state/cliUpdate.svelte.ts)) feeds the contextual line (npm/native/multi/stuck/error) across Home banner + tab-bar popover + Settings — was hand-authored 3× and drifted. Home + popover now share a tone-aware treatment (`data-tone` = accent/warn/danger). `UpdateDialog.svelte` status tints fixed `oklch`→`oklab` (warm-hue purple-wrap bug, per CRITICAL rule). A temp `UpdatePreview.svelte` dev panel drove all states/tones for live CDP verification, then was removed (file + AppShell mount).
- **Verified:** `cargo check` 0/0 · `npm run check` 0/0 (4063 files) · every surface (banner, popover, Settings row, Velopack dialog across all states+tones) CDP-verified live.

### RESUME HERE (cont.63)
- **v0.6.1 SHIPPED** (feat commit + published to `rift-releases` via `release.ps1`). PENDING live-verify on a real dual-install box: banner clears after Update-all; if a native copy truly won't bump, the "still behind" hint + DiagBus logs name the culprit.
- **v0.6.0 carry-over live-verify still owed:** v0.5→0.6 auto-update on a real machine · browser render-flash · mid-turn steer · permission bar · fresh-install onboarding.
- **Open:** #21 test harness (T1) · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum sign-off. (#30 update-UI redesign shipped + block deleted from ISSUES.)

---

## Shipped + prior arcs — detail in `git log`
- **v0.6.1** (cont.63) CLI multi-install + unified update UI · **v0.6.0** (cont.61, `316dc5e`) browser dock + polish (includes the cont.57 model-picker capability matrix — that work shipped here, NOT pending) · **v0.5.0** (cont.51, `62dae27`) Velopack stable.
- **release.ps1 gotchas:** bump THREE files + `Cargo.lock` (run `cargo check` so the lock updates) BEFORE; commit for a clean tree or pass `-Force`; quit `rift-tauri.exe` (dev) before build — Win file-lock; drop portable AFTER `vpk upload`; never wrap `release.ps1`/`tauri build` in `*>&1` from the PS tool (PS5.1 → terminating `NativeCommandError`). Setup.exe-only. vpk CLI version == velopack crate version.
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
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.6.1 stands** (shipped 2026-06-06, cont.63).
