# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-02 (cont. 2) — Phases 4 & 5 DONE + CDP-VERIFIED. **Full redesign arc P1–P5 complete on `main`, NOT pushed/shipped.** Only P6 (ship) remains.
Plan + baseline: `docs/design/redesign-gap-audit.md`. `npm run check` 0/0/4109 after each phase. P1–3 detail in `git log` (`fff86a2`/`d34fe7f`/`cb0384c`/`d5e3eef`/`77c8aab`). This session:
- **`bb3fb10` P4 Files** — kept local|remote two-pane, ADDED a right `FbDetail.svelte` (340px) showing status pill + size/modified/location meta for the last single-selected file (either pane; clearing one side never clobbers the other). Text-`▲` → status-LED pips (synced/modified/conflict) per file row. All/Lua/Conflicts/Modified filter chips drive both panes (dirs always shown for nav). Statuses real-data only: conflict=`connection.conflicts`, modified=`connection.dirtyEdits`. **Lua code-preview deferred** (no file-read path). CDP-verified chips+detail+empty-state; populated rows/LEDs need a connected server (untested-by-design — no live FiveM box touched).
- **`c5963f5` P5 Onboarding** — full `ob-*` restyle: `OnboardingFlow` = left-rail vertical stepper + right pane + pinned footer (dots · Back · one solid CTA; shell owns Back/Next, steps report readiness via `onReady`/`onSaved`). New `ObStage.svelte` per-step anims. Welcome 8-swatch accent picker → `uiPrefs.setAccentHue`; stale comment removed. **ProfileSetup folded into `ServerAdd` (deleted) → 5 steps.** FirstSync simulated animated scan (bar+log+pills; pills `—`, no fabricated counts). New global `src/lib/styles/onboarding.css` (mockup port; `--gi-*`→accent, srgb/rgba→oklch). **All 5 steps CDP-verified end-to-end** via temp-forced gate (restored byte-exact; AppShell shows clean diff).

### RESUME HERE — Phase 6: SHIP the whole P1–P5 arc (deliberate `/git-ship` session — do NOT mid-session)
- Ship pipeline: bump THREE version files (`package.json`+`Cargo.toml`+`tauri.conf.json`, currently 0.4.46) + `Cargo.lock` → CHANGELOG → `/check` → `npm run tauri build` → self-replace dance → install → shortcut/iconcache → commit. `scripts/release.ps1` preflights version lockstep.
- Nothing else outstanding on the redesign. If verifying live first: `scripts/run-dev.bat` → `npm run cdp:serve` → `bash scripts/cdp/c.sh`.

## CRITICAL DON'T-TOUCH (frontend/redesign — full backend list in `git log`/prior HANDOFF)
- **Onboarding gate (#7):** `showOnboarding` MUST keep all 4 conditions (`!dismissed && serversLoaded && servers.length===0 && defaultSshKeyExists===false`). Loosening flashes onboarding for existing users; `defaultSshKeyExists` null=unknown→don't show. P5 must temp-force then RESTORE.
- **Accent themeable via `--accent-h`** (app.css `:root` only): write `oklch(L C var(--accent-h))`; never hard-code an accent hue. Components use `var(--accent)`/`--accent-soft` (already retheme) — do NOT add `--accent-h` to them. Status LEDs (`--ok/warn/danger/info`) stay fixed.
- **IA:** `home` top-level kbd 1; Activity is a Sync tab (`syncPage.tab`), not a workspace. Kbds: home·1 chat·2 sync·3 files·4 settings·5. Home wiring real-data only (no fabricated counts).
- **AssistantPane drop handlers on `.pane` outer only** — inner overlays break the preventDefault chain (ChatRail is a flex sibling, never an ancestor of `.pane`). `tauri.conf.json dragDropEnabled:false` required for HTML5 DnD. `.shell` MUST be `position:fixed; inset:0`.
- **RCON:** pw plaintext over UDP is how FXServer RCON works — keep it in the keychain, never on disk/IPC. Targets `bridge_port` (game port), not the SSH port.
- **Versions lockstep** across `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`, not mid-session.
