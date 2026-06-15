# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.11.0 — 2026-06-15 — UI consistency pass: shared PageHero + Home/nav polish

> **Why.** A full-app UI run-through (`docs/design/ui-review-2026-06-15.md`) found the same page-hero chrome copy-pasted across Settings and Local LLM (drifted to different widths), a lopsided Home page, and unmarked experimental nav. This release unifies the shared chrome and tidies Home + nav, and folds in two prior in-flight fixes.

**New:**
- **Shared `PageHero` component** — one source for the eyebrow / icon / title / desc / status-chip / body hero chrome. Settings and Local LLM now consume it instead of two drifted copies; `.sb-wrap` width unified 880px → 820px.

**Changed / Fixed:**
- **Home balance.** Filled the dead right-column with a Quick-actions card; collapsed the two near-identical "start a chat" affordances — the hero button is now "＋ New tab", the full-width launcher stays the primary action.
- **Nav clarity.** Experimental amber dot on the Local LLM nav item; the Settings gear tooltip now surfaces its Ctrl+3 / Ctrl+, shortcuts.
- **Live-status consolidation.** The streaming readout (label · elapsed · tokens) now lives once at the composer; the in-bubble stage-strip is dots-only and trailing-activity is removed (was: tokens in 3 places, elapsed in 2).
- **Drag-to-split** pane-routing fix (a right-half drop into a single pane now splits instead of mis-routing) + STT Haiku polish made non-blocking (composer editable the instant the raw transcript lands).
- **Docs.** Corrected the thinking-display diagnosis in `turn.rs` — Opus 4.8/4.7 default `thinking.display` to `"omitted"` (not "-p mode encryption"; Sonnet streams it because its default is `"summarized"`). CLI 2.1.177 exposes no override flag, so Opus thinking text can't be surfaced today — gated upstream.

**Deferred → `docs/ISSUES.md #39`:** P0-3 unify the CLI-update notice (3 surfaces, touches the Velopack update path); the P2 shared size-token + color-token + a11y sweep.

**Verify.** version lockstep ×3 + `Cargo.lock` at 0.11.0 · svelte-check 0/0 (4094 files) · Settings + Local LLM heroes live-verified via CDP.

## Older versions

v0.10.0 Home stats dashboard (`assistant_stats` + KPI tiles/heatmap, honest-data-only) + audit-hardening pass (strict image MIME allowlist · model-label dedupe · aria-labels) + Fable kill-switch · v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
