# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.3 — 2026-06-13 — Release-readiness hardening

> **Why.** A pre-wider-distribution audit (`docs/release-readiness-2026-06-13.md`) surfaced two ship-blockers — at the new-user edge and in field crash observability — plus a small robustness backlog. This closes both T1 blockers, the T2 should-fixes, and every actionable T4 swallow. No feature changes.

**Fixed — ship-blockers:**
- **New-user auth dead-end (RR-1).** The `needsAuth` welcome card was static — a user who skipped onboarding with no CLI had no in-app way to authenticate (the interactive Sign-in lived only in a post-turn banner a red-pill user can never reach). Added live **Sign in** (`startLogin`) + **Re-check** buttons on the card; fixed the dead "hit refresh" text.
- **No field crash file (RR-2).** Panics now write a dedicated, non-rotating `crash-<ts>.txt` (version + location + scrubbed backtrace) alongside `rift.log` — survives a second crash and captures startup panics that never reach the UI.

**Fixed — hardening:**
- **open-path capability (RR-4)** now denies OS-execute-by-default extensions (exe/msi/bat/cmd/…) so "Open" can't shell-launch an executable.
- **Steer loss (RR-6)** during the init handshake surfaces write/build errors instead of dropping silently.
- **Oneshot errors (RR-7)** surface a panicked stderr-drain JoinError instead of an empty body.
- **Zombie download (RR-9)** after a stall can no longer flip `downloaded=true` — an epoch guard invalidates the abandoned attempt so a retry can't `apply` a stale package.

**Polish + robustness (T4):** dropped the redundant wrench glyph from tool-group heads; mixed tool-groups now lead with the dominant kind ("Reading 3 files +1 more"). Surfaced previously-swallowed errors — stop-on-spawn kill (B3), unreachable-UI permission emit now denies fast (B4), removed an `.expect()` panic path (A1), bridge missing-`session_id` warning (B7), STT capture-thread panic (B10), log-rotation-failure size cap (B11), and corrected a stale Velopack version comment (RR-14).

**Verify.** svelte-check 0/0 · vitest 12/12 (toolCaption) · `cargo check` clean at 0.9.3 · CDP-verified live: app boots with 0 console errors, wrench-free group heads + correct captions render.

## Older versions

v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
