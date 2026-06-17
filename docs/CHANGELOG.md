# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.16.2 — 2026-06-17 — Release repo rename + docs cleanup

> **Why.** The public release/feed repo was renamed `rift-releases` → **`rift`** for a cleaner public front door; this propagates the new identity everywhere and tidies the docs tree.

**Changed:**
- **Release repo renamed to `rift`.** All references repointed — the in-app "view releases" link (`updates.svelte.ts`), the Settings about-row link, `web/index.html`, `README.md`, and `release.ps1`'s publish target (`$releaseRepo`). GitHub auto-redirects old URLs, so existing installs are unaffected. The production updater is independent regardless (reads the Cloudflare R2 `HttpSource`, not GitHub).
- **Settings license label corrected** `MIT` → `Proprietary`.

**Docs:**
- Pruned 3 superseded/zero-reference dated docs (`ui-polish-arc`, `overnight-2026-06-15`, `release-readiness-2026-06-13`) — history in `git log`.
- Swept canonical `Blazzer10200/rift-releases` → `/rift` across live docs (R2-bucket name `rift-releases` deliberately unchanged — different resource).
- Added `docs/design/website-design-brief.md` (download-site rebuild handoff).

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · string-only app edits (no behavior change).

## Older versions

v0.16.1 Portability + licensing cleanup — STT models resolve through `dirs_home()` (Velopack no longer wipes them); actionable CLI-not-found error; license declared `UNLICENSED`/private. · v0.16.0 CLI transparency — surfaced three previously-silent Claude CLI stream events inline: a "Conversation compacted" pill (`compact_boundary` + `Ctx X%→Y%` meta), `max_tokens`/`refusal` stop-reason notices (+ one-click Continue), and plain-English run errors; View-menu tidy + split-empty-card polish. Frontend-only. · v0.15.0 Multi-window (Route A) — titlebar "New window" spawns a second native window w/ isolated per-window tab state + `emit_to(label)` event routing, gated by `secondary-window.json`; async-command fix for the WebView2 `about:blank` deadlock; env-pill glides clear of open docks; sub-agent dock CSS cleanup. · v0.14.0 Chat-page top-right redesign — Environment became a floating pill widget (auto-shows on first message, expands to an in-flow panel that never overlaps the composer), header de-duped (branch + ctx% each shown once), View menu regrouped into History · Panels · Layout. · v0.13.0 Environment panel (source-control dock) + tooltips pulled app-wide (no-op a11y shim, −77 lines dead `.tip` CSS) + design-token consistency (`--radius-2xl`, 36 off-scale literals replaced) + `git_local.rs` UNC symlink-guard fix + new `ARCHITECTURE.md`/`SECURITY.md` docs. · v0.12.3 self-update brick fix — Claude CLI child cwd no longer defaults to the install dir (a hook-spawned daemon could lock Velopack's `current\`). · v0.12.2 security: DOMPurify `3.4.3→^3.4.10` (prod audit 0 vulns; `check` CI green). · v0.12.1 split-pane send routed to wrong pane (#41 — `send(prompt, tabId?)` retargets the firing pane synchronously) + STT polish shimmer 6s cap & typing-cancel (#40) + single live timer in turn head (#39 P0-4). · v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.11.0 UI consistency pass (shared `PageHero`, Home quick-actions, drag-to-split fix) · v0.10.0 Home stats + Fable kill-switch · v0.9.x R2 update feed + minimal-core strip (−7,407 lines → 3 workspaces) · v0.8.x composer slim + dictation/PTT + loopback bridge + Fable 5 + backend split + tag-driven CI · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
