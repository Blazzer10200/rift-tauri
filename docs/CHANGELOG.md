# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.12.0 — 2026-06-15 — Local LLM page: cockpit redesign

> **Why.** The Local LLM workspace was a flat config form — sterile rows, a one-shot Test button, and a big dead void below the cards. This release rebuilds it as a status-driven "cockpit" so setup is guided and the connection state is legible at a glance. Frontend-only; no backend or config touched, fully reversible.

**Changed:**
- **Cockpit layout.** A Mode master strip on top, then a status/readiness **rail** (left) + **config** (right). Content vertically centers, killing the empty lower half of the page.
- **Readiness state machine.** `Off → Setup incomplete → Ready → Verified & live`, derived from live config — drives the hero chip tint, a glowing status dot, and a 2px state-colored rail hairline. A setup checklist (Endpoint / Model / API key / Verified) shows each step's current value.
- **Verify card.** Test now reports client-side round-trip **latency** + a "checked HH:MM:SS" stamp and renders the reply in a proper inset card. Editing the endpoint or model invalidates a prior pass, so the rail never claims "live" against untested settings.
- **Quick-start presets** (LiteLLM `:4000` / `127.0.0.1`) fill the base URL with one click; **detected models** now surface as selectable chips instead of a hidden `<datalist>`.
- **Active-mode tint.** The Mode strip washes accent when local mode is on; the off-state shows a `Rift → Endpoint → Local model` flow explainer instead of dead dimmed rows.

**Docs.** Pruned six shipped "resolved in-tree" blocks from `docs/ISSUES.md` (#34, CR-UX, #29, #30, #32, #38 — all landed at or under v0.11.0; history via `git log`).

**Verify.** version lockstep ×3 + `Cargo.lock` at 0.12.0 · svelte-check 0/0 (4094 files) · Local LLM page live-eyeballed (on/off states) by user.

## Older versions

v0.11.0 UI consistency pass: shared `PageHero` (Settings + Local LLM, width unified 880→820) · Home quick-actions balance + collapsed dup "new chat" · nav experimental-dot + Settings shortcut tooltip · live-status consolidated to the composer · drag-to-split routing fix + non-blocking STT · thinking-display diagnosis corrected in `turn.rs`. · v0.10.0 Home stats dashboard (`assistant_stats` + KPI tiles/heatmap, honest-data-only) + audit-hardening pass (strict image MIME allowlist · model-label dedupe · aria-labels) + Fable kill-switch · v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
