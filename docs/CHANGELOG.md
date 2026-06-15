# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.10.0 — 2026-06-15 — Home stats dashboard + audit-hardening pass

> **Why.** The Home page had a large empty lower-center region. This release fills it with a Rift-native stats dashboard (inspired by Claude Code desktop, built on honest data only), plus a full-app audit pass (security · dead-code · UI/UX) and a couple of composer/UX fixes.

**New:**
- **Home stats dashboard.** New `assistant_stats` backend command scans every saved transcript once → per-conversation summaries; the frontend aggregates in local time. Overview = 8 KPI tiles (Sessions · Messages · Tool calls · Spend / Active days · Streak · Peak hour · Top model) + an 18-week GitHub-style activity heatmap + a Moby-Dick word-count fun-fact. Models tab = daily message bars + colored per-model breakdown. All/30d/7d range chips; empty + skeleton states. No fabricated token totals — per-message tokens aren't persisted, so only honest metrics are shown.

**Changed / Fixed:**
- **Fable 5 disabled** (US-gov disablement, temporary) behind a revertible two-flag kill-switch — the picker hides it and any pinned/stored Fable session falls back to Opus before it can hit the API. Re-enable = flip `FABLE_DISABLED` on both sides.
- **Composer attachments** now render inside the input card instead of floating between the queue rail and the composer (fixed the lifted-rail / detached-image layout when pasting mid-turn).
- **Security:** attachment MIME is now a strict allowlist (png/jpeg/gif/webp) instead of an `image/` prefix.
- **Tidy:** deduped a drift-prone model-label map; added a couple of icon-button aria-labels. Deferred audit findings catalogued in `docs/ISSUES.md` #39.

**Verify.** version lockstep ×3 + `Cargo.lock` at 0.10.0 · svelte-check 0/0 · vitest 154/154 · cargo check clean · stats dashboard live-verified via CDP.

## Older versions

v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
