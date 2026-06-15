# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.12.2 — 2026-06-15 — Security: DOMPurify patch (CI `check` green)

> **Why.** The `check` workflow's `npm audit --omit=dev` gate flagged a moderate DOMPurify advisory (multiple XSS vectors in `IN_PLACE`/template/shadow-root sanitization, `dompurify <=3.4.8`). Pre-existing — it also failed the prior commit. DOMPurify backs Markdown's `{@html}` sanitization, so the bump is load-bearing.

**Changed:**
- **`dompurify` `3.4.3 → ^3.4.10`** (patched; same-minor, no API change). Prod audit now reports **0 vulnerabilities**; the `check` workflow's frontend job goes green.

**Verify.** version lockstep ×3 + `Cargo.lock` · `npm audit --omit=dev` 0 vulns · svelte-check 0/0 (4094) · vitest 162/162.

## v0.12.1 — 2026-06-15 — Split-pane send fix + STT/timer polish

> **Why.** Three user-spotted quirks during real use. All frontend-only, no backend or config touched.

**Fixed:**
- **Split-pane send routed to the wrong pane (#41, T1).** A message composed in one pane could land in the other. `send()` keyed off the global `currentConvoId`, but the per-pane composer fired without a tabId — so the turn targeted whichever pane was focused, not the one that fired. `assistant.send(prompt, tabId?)` now retargets the active conversation to the firing pane's tab synchronously before dispatch; `AssistantPane` passes its `tabId`. (Drafts/attachments were already pane-correct.)
- **STT polish shimmer flashed too long (#40).** End-of-dictation, the textarea pulsed for the entire Haiku cleanup call (backend cap 15s). Now: a **6s frontend cap** drops the visual early (the raw transcript is already committed + editable), **typing cancels** the shimmer instantly (`cancelPolish()` + guard token, invalidating the late swap), and a redundant rewrite at stop is skipped so the phrase no longer flashes back in.
- **Double live timer in the turn head (#39 P0-4).** While "Thinking" the role-row `9s` heartbeat ticked one line above the thinking pill's own timer. The heartbeat now yields to a quiet dot during the thinking phase and returns for tool/text phases — one live counter at a time.

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 (4094 files) · vitest 162/162 · live app 0 console errors after HMR.

## Older versions

v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.11.0 UI consistency pass: shared `PageHero` (Settings + Local LLM, width unified 880→820) · Home quick-actions balance + collapsed dup "new chat" · nav experimental-dot + Settings shortcut tooltip · live-status consolidated to the composer · drag-to-split routing fix + non-blocking STT · thinking-display diagnosis corrected in `turn.rs`. · v0.10.0 Home stats dashboard (`assistant_stats` + KPI tiles/heatmap, honest-data-only) + audit-hardening pass (strict image MIME allowlist · model-label dedupe · aria-labels) + Fable kill-switch · v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
