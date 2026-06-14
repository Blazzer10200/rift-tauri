# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.5 — 2026-06-14 — R2 publish actually fires (feed populated)

> **Why.** v0.9.4 pointed the updater at the self-hosted Cloudflare R2 feed, but the CI `env:` block that passes the R2 secrets into `release.ps1` landed *after* the v0.9.4 tag was cut — so v0.9.4's own release run never reached the conditional `vpk upload s3`. The bucket stayed empty: the site download CTA 404'd and R2-reading clients had no feed. This is the first tag to run the corrected workflow.

**Fixed:**
- **R2 dual-publish now executes.** `release.yml`'s Release step exports the four `R2_*` secrets, so `release.ps1`'s conditional `vpk upload s3` runs and lands `Setup.exe` + `.nupkg` + `releases.win.json` in the `rift-releases` R2 bucket. Site CTA + R2 auto-update feed go live with this tag.

**Verify.** version lockstep ×3 + `Cargo.lock` at 0.9.5.

## Older versions

v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
