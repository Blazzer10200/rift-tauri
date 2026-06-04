# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 44–45) — Velopack auto-update restored + update-UI visual QA

`cargo check` 0/0 (velopack 1.2.0); `npm run check` 4067/0/0; `release.ps1` PS-parses clean. Update notification UI (toast + dialog) CDP-verified + polished (cont.45). UNSHIPPED on v0.4.46 → ships v0.4.47. Full design + rationale: [docs/design/velopack-auto-update.md](design/velopack-auto-update.md).

- **Auto-update is back, one-click-then-unattended.** New [update_service.rs](../src-tauri/src/update_service.rs) wraps `velopack::UpdateManager` over the **native `GithubSource`** (v1.2.0 — old ~200-line custom REST source deleted). [commands/update.rs](../src-tauri/src/commands/update.rs): `check_for_updates`/`download_update` (streams `update-progress`)/`apply_pending_update`. `lib.rs` runs `VelopackApp::build().run()` after the `RIFT_MCP_SERVER` guard + manages `UpdateService`.
- **Apply-exit bug fixed:** `wait_exit_then_apply_updates(silent,restart)` + `app.exit(0)` (NOT the old `apply_updates_and_restart` that raced WebView2). [updates.svelte.ts](../src/lib/state/updates.svelte.ts) + `UpdateDialog.svelte` repointed (states: available→downloading→installing); `releaseUrl` synthesized from the tag.
- **Ship:** `release.ps1` reverted to `vpk pack`/`vpk upload github`; **vpk 0.0.1298→1.2.0**, crate pinned `=1.2.0`, script preflights they match. `--pre` valid again for alpha (GithubSource prerelease:true).

### RESUME HERE
- cont.44 + cont.40/41/42/43 ALL UNSHIPPED on v0.4.46 — ONE ship commit, next ver **v0.4.47**. Backend changed → `cargo check` before ship (cont.44 verified clean). CHANGELOG has a v0.4.47 entry on top + the pure-assistant Unreleased below.
- **TASK 1 — UPDATE-UI VISUAL QA: ✅ DONE (cont.45).** Toast + dialog CDP-verified in all states (available/downloading/installing/download-error) — all clean. Polished `UpdateDialog.svelte`: `installing` is now its own compact branch (no redundant notes/link → no tall→short→tall jump) + card icon Download→spinning `RefreshCw`. Drove states via a new **dev-only `window.__updates` hook** (AppShell, `import.meta.env.DEV`-guarded, prod-stripped) — the only way to capture `installing` (real flow exits the app). **Reuse for future update-UI QA:** set `window.__updates.{state,info,progress,dialogOpen}` → `shot-sel ".upd-shell"`. `npm run check` 0/0.
- **NOT ship-tested — REQUIRED before broad release:** Velopack auto-apply only proves out across **two real releases on a real machine** (design doc §6 R1). Also the **migration bridge** (§5): the first Velopack release reinstalls per-user as `rift-tauri.exe` at `%LocalAppData%\Rift`, SEPARATE from the current NSIS `rift.exe` — one-time reinstall, communicate in release notes.
- **`rift-tauri.exe` name collision (post-Velopack):** after migration the *prod* app is `rift-tauri.exe` too — the "never blanket-kill rift, dev=rift-tauri/prod=rift.exe" rule in CLAUDE.md will need revisiting once clients are on Velopack.
- **NOT a silent-install regression:** cont.42 VOIDED *fully-silent no-click* NSIS `/S`. This flow is **consent-first** (user clicks Download once) THEN unattended — that's what the user explicitly asked for this session. Distinct from the rejected pattern.
- **Grounding caveat (cont.43):** grounded enhance is agentic (~10-20s) → opt-in, NEVER default. `--max-turns 6` valid on bundled CLI.
- **Dead-dep flag (confirmed this session):** `russh`/`russh-sftp`/`notify` = **0 usages** in `src-tauri/src/` (orphaned by pure-assistant rip; russh also RUSTSEC-0153/0154). Removal touches rustls feature wiring → do as its own verified pass. Also: **DEFER STT** (whisper-rs 0.13→0.16 blocked); dead-code DiagStage / secrets fns / SAFE_MCP entries.
- **History v3 backlog** (designed, NOT built): content-aware search, pinning, in-list kbd nav, bulk delete.

## Prior arcs — detail in `git log`
- **43 (Composer + Improve-prompt):** resting bar 5→2 icons; (?) popover removed; Improve-prompt → Sonnet + meta-prompt v2 + refine/diff/ground loop (`assistant_enhance_prompt` gained `model`/`directive`/`cwd`).
- **42 (Claude-update automation):** in-app CLI update (`assistant_update_cli`, method-aware npm/native), install-method detection (`AuthStatus.install_method`), "Update now" on 3 surfaces, 6h Rift self-update re-check. User-initiated / notify-only.
- **41 (onboarding rebuild):** 3-step welcome (`OnboardingFlow.svelte`: stepper + Welcome/Personalize/Connect-Claude); `onboarding.css` pruned ~130 dead lines. Frontend-only.
- **40 (History v2):** `HistoryDrawer` cost (`ConversationMeta.cost_usd` + Rust sum), real MD/JSON export (`assistant_export_save`); 13 audit/swarm scripts archived.
- **39 / 32–38 (audit arc):** 244+247 findings cleared; chat-rail retired; Activity split + `SessionDiff.svelte`; titlebar nav.
- **20/21 PURE-ASSISTANT:** SFTP/sync/server/RCON ripped; MCP→read/list/grep+`git_*`; IA=3 workspaces.

## CRITICAL DON'T-TOUCH
- **Onboarding gate:** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && !assistant.hasApiKey && !assistant.auth?.loggedIn`. `configLoaded` gates timing so it never flashes pre-probe.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 3 workspaces** — home·1 chat·2 settings·3. Nav lives in the **titlebar** (Home/Chat `.navitem`s + Settings gear); switch via `workspace.setActive`/Ctrl+1-3. Settings = one scroll-doc, 5 sections. **Left chat rail retired** — history lives ONLY in the History drawer (View-menu → `HistoryDrawer`).
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → opens Session Diff. `SessionDiff.svelte` reads `tab.messages` via `EditDiff` `hideHead`; open via `assistant.ui.diffOpen/diffTarget`.
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. v0.4.46 stands.
