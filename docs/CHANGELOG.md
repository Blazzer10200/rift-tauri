# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.4 — 2026-06-14 — Self-hosted update feed (R2 bridge release)

> **Why.** Cuts the auto-updater over from the `rift-releases` GitHub repo to a self-hosted Cloudflare R2 feed. This is the **bridge** release: shipped via the GitHub path so existing clients (still on `GithubSource`) receive it, while the binary itself is the first to read R2 — so every client from here on updates from R2.

**Changed:**
- **Updater feed → Cloudflare R2 (`HttpSource`).** `update_service.rs` now resolves updates from `https://pub-4fb26c0fc8df484488e4415f112f2d28.r2.dev` instead of the GitHub releases repo (`HttpSource::new` arity confirmed vs velopack 1.2.0). The `RIFT_UPDATE_FEED` local FileSource dev hatch is unchanged.
- **`release.ps1` dual-publish to R2.** After the GitHub `vpk upload`, a conditional `vpk upload s3` pushes the same artifacts to R2 when R2 CI secrets are present (no-op otherwise); GitHub path untouched. This is the run that auto-lands `Setup.exe` in the bucket.
- **`web/` static download site** (Cloudflare Pages, Rift design tokens) live at **rift-5hr.pages.dev**.

**Infra (provisioned cont.125):** R2 bucket `rift-releases` (public URL, CORS-enabled, smoke-tested 200) + 4 CI secrets set.

**Verify.** `cargo check` clean at 0.9.4 · version lockstep ×3 + `Cargo.lock` at 0.9.4.

## Older versions

v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
