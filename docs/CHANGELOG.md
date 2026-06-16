# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.15.0 — 2026-06-16 — Multi-window (Route A) + sub-agent dock cleanup + env-pill overlap fix

> **Why.** A session was trapped in one native window — no way to put a second chat on a second monitor. This adds a real second window with isolated per-window state, plus two UI-polish fixes that landed in the same arc.

**Added:**
- **Multi-window (#37, Route A)** — a "New window" button (`AppWindow` icon) in the titlebar spawns a second native window (`invoke("open_new_window")` → `commands::open_new_window`). Each window is a `window-N`-labelled `WebviewWindowBuilder` (decorations off, centered via the extracted module-level `center_in_work_area`), gated by the new [`secondary-window.json`](../src-tauri/capabilities/secondary-window.json) capability (`window-*` glob + drag perm). **Per-window event routing** — the label flows `window → assistant_send → write_mcp_config → RIFT_BRIDGE_WINDOW → bridge`, and the bridge + `turn.rs` emit via `emit_to(window_label, …)` instead of a global `emit`, so events never bleed across windows. **Per-window tab state** — `tabsStorageKey()` namespaces persisted tabs by label (`rift.ui.tabs.v1` for main, `rift.ui.tabs.<label>.v1` otherwise).

**Fixed:**
- **Multi-window opened to `about:blank` on Windows** — `open_new_window` was a *synchronous* command, which deadlocks WebView2 mid-build (tauri-apps/tauri#13963); the window appeared but never navigated. Making the command `async` dispatches the build off the main thread and the window now loads the app cleanly.
- **Env-pill overlapped open docks** — the collapsed Environment pill (`position:absolute right:8px`) floated over the browser / sub-agent dock headers on the same right edge. `pillRight` is now `$derived(8 + open-dock widths)` with a `transition: right 200ms`, so the pill glides clear of any open dock (verified 8px → 571px when the browser dock opens).

**Changed:**
- **Sub-agent dock cleanup** (CSS-only) — the `/plan` prompt no longer renders as a wall of text (`.desc` clamped to 2 lines; full prompt still in the main transcript's Task call); tool rows lightened from bordered cards to a borderless checklist (pending = soft-accent tint + pulse, error = faint danger), results get a left-accent border, tighter body gap.

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · cargo build green (dev) · CDP-verified live: two app windows render full UI w/ isolated tab namespaces, env-pill glides 8→571px clear of the browser dock.

## Older versions

v0.14.0 Chat-page top-right redesign — Environment became a floating pill widget (auto-shows on first message, expands to an in-flow panel that never overlaps the composer), header de-duped (branch + ctx% each shown once), View menu regrouped into History · Panels · Layout. · v0.13.0 Environment panel (source-control dock) + tooltips pulled app-wide (no-op a11y shim, −77 lines dead `.tip` CSS) + design-token consistency (`--radius-2xl`, 36 off-scale literals replaced) + `git_local.rs` UNC symlink-guard fix + new `ARCHITECTURE.md`/`SECURITY.md` docs. · v0.12.3 self-update brick fix — Claude CLI child cwd no longer defaults to the install dir (a hook-spawned daemon could lock Velopack's `current\`). · v0.12.2 security: DOMPurify `3.4.3→^3.4.10` (prod audit 0 vulns; `check` CI green). · v0.12.1 split-pane send routed to wrong pane (#41 — `send(prompt, tabId?)` retargets the firing pane synchronously) + STT polish shimmer 6s cap & typing-cancel (#40) + single live timer in turn head (#39 P0-4). · v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.11.0 UI consistency pass: shared `PageHero` (Settings + Local LLM, width unified 880→820) · Home quick-actions balance + collapsed dup "new chat" · nav experimental-dot + Settings shortcut tooltip · live-status consolidated to the composer · drag-to-split routing fix + non-blocking STT · thinking-display diagnosis corrected in `turn.rs`. · v0.10.0 Home stats dashboard + audit-hardening + Fable kill-switch · v0.9.x self-hosted R2 update feed, release-readiness hardening, Concept-D tool cards, UI-polish arc, and the minimal-core strip (−7,407 lines → 3 workspaces) · v0.8.x composer slim + dictation/PTT + loopback UI bridge + Fable 5 + backend split + tag-driven CI · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
