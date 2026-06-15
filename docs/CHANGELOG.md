# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.12.3 — 2026-06-15 — Self-update no longer bricked by hook-spawned daemons

> **Why.** A user's in-app update + reinstall failed with *"Failed to remove existing application directory."* Velopack couldn't rename `…\Rift\current\` because a live process held it as a working directory. Root cause: with **no workspace folder open**, Rift spawned the Claude CLI without a cwd, so the child inherited Rift's own install dir (`current\`). The CLI's SessionStart hook then launched a long-lived daemon (the user's Pulse telemetry daemon, `disown`ed) there — surviving app exit and locking `current\` forever. The apply-time reap only kills `rift-tauri.exe`, so it never caught the orphaned, out-of-tree daemon, and every update/reinstall died.

**Fixed:**
- **Claude CLI child cwd no longer defaults to the install dir** ([turn.rs](../src-tauri/src/assistant/turn.rs)). The spawn now sets cwd to the temp dir, overridden to the workspace root when a folder is open — so the child (and anything its hooks spawn) can never inherit `…\current\` again. Prevents the locker from ever being created; a disowned daemon can't be reaped at apply-time, so prevention is the only reliable cut.

**Verify.** version lockstep ×3 + `Cargo.lock` · `cargo check` clean · live repro fixed (orphan dir + lockers cleared, daemon re-anchored to a safe cwd).

## v0.12.2 — 2026-06-15 — Security: DOMPurify patch (CI `check` green)

> **Why.** The `check` workflow's `npm audit --omit=dev` gate flagged a moderate DOMPurify advisory (XSS vectors, `dompurify <=3.4.8`). DOMPurify backs Markdown's `{@html}` sanitization, so the bump is load-bearing.

**Changed:**
- **`dompurify` `3.4.3 → ^3.4.10`** (patched; same-minor, no API change). Prod audit now reports **0 vulnerabilities**; the `check` workflow's frontend job goes green.

**Verify.** version lockstep ×3 + `Cargo.lock` · `npm audit --omit=dev` 0 vulns · svelte-check 0/0 (4094) · vitest 162/162.

## Older versions

v0.12.1 split-pane send routed to wrong pane (#41 — `send(prompt, tabId?)` retargets the firing pane synchronously) + STT polish shimmer 6s cap & typing-cancel (#40) + single live timer in turn head (#39 P0-4). · v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.11.0 UI consistency pass: shared `PageHero` (Settings + Local LLM, width unified 880→820) · Home quick-actions balance + collapsed dup "new chat" · nav experimental-dot + Settings shortcut tooltip · live-status consolidated to the composer · drag-to-split routing fix + non-blocking STT · thinking-display diagnosis corrected in `turn.rs`. · v0.10.0 Home stats dashboard (`assistant_stats` + KPI tiles/heatmap, honest-data-only) + audit-hardening pass (strict image MIME allowlist · model-label dedupe · aria-labels) + Fable kill-switch · v0.9.4 self-hosted update feed (R2 bridge: updater → Cloudflare R2 `HttpSource` + `release.ps1` dual-publish + `web/` Pages site) · v0.9.3 release-readiness hardening (new-user auth dead-end RR-1 · field crash file RR-2 · open-path exec-deny RR-4 · steer/oneshot/zombie-download robustness · T4 swallow sweep) · v0.9.2 Concept-D tool-group cards + composer auto-correct · v0.9.1 UI polish arc (token counter climbs mid-turn · notifications→severity toasts · in-app image lightbox · drag-drop window guard · Activity declutter · streaming pacer tuning) · v0.9.0 minimal core (buddy release): −7,407-line strip (Harness/Swarm/cost-cockpit/compaction/custom-providers removed → 3 workspaces) + #33 closed by removal + #34 SessionDiff fix · v0.8.x composer slim + dictation/PTT fixes + loopback UI bridge + Fable 5 + backend split + tag-driven CI · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
