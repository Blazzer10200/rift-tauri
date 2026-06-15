# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.139) — Local LLM cockpit redesign → SHIPPED v0.12.0

Rebuilt the **Local LLM page** (`local-llm/LocalLlmPage.svelte`) from a flat 3-card form into a status-driven cockpit; pruned stale ISSUES; shipped **`v0.12.0`** (tag pushed → CI → rift-releases).
- **Cockpit layout:** Mode master strip on top, then status/readiness **rail** (left) + **config** (right). Content vertically centers (`justify-content: safe center`) — killed the dead lower-half void.
- **Readiness state machine** (`Off → Incomplete → Ready → Verified`) derived from live config — drives hero chip tint, status dot, and a 2px state-colored rail hairline. Setup checklist (endpoint/model/key/verified) with live values.
- **Verify card:** client-side round-trip latency (`performance.now()`) + "checked HH:MM:SS" stamp + reply in an inset card; any endpoint/model edit invalidates the prior pass. **Quick-start presets** fill base URL; **detected models** are selectable chips (was a hidden datalist). Mode strip washes accent when on; off-state shows a Rift→Endpoint→Model flow explainer.
- **Frontend-only** — no backend/config touched, fully reversible. svelte-check 0/0 (4094 files). User live-eyeballed on/off states.
- **ISSUES prune:** deleted six shipped "resolved in-tree" blocks (#34, CR-UX, #29, #30, #32, #38 — all ≤ v0.11.0).
- **RESUME:** #39 P0-3 (unify CLI-update notice — Velopack path) + P2 (shared size/color tokens). Local LLM **backend pass** (live health ping on load · model metadata · real tool-calling probe) deferred — frontend cockpit done, backend richness not started.
- **Flagged for your call:** possibly-stale design docs `self-hosted-distribution*.md` / `ui-polish-arc.md` — not deleted (couldn't confirm completed). Catalogued #39 dead-code dupes deliberately kept out of this release (multi-file refactor, not night-ship material).

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.139 → v0.12.0** Local LLM cockpit (this session).
- **cont.138 → v0.11.0** shared `PageHero` (Settings + Local LLM), Home quick-actions, nav experimental-dot, live-status→composer, drag-split fix, thinking-comment fix.
- **cont.136** `a3ab764` live sub-agent activity dock (`parent_tool_use_id` routing).
- **cont.134 → v0.10.0** Home stats dashboard, Fable disabled behind `FABLE_DISABLED` kill-switch, audit-hardening.
- **cont.131–133** per-pane STT routing + per-tab workspace root, Home `assistant_stats`.

## Prior arcs — detail in git log + CHANGELOG
cont.130 v0.9.5 R2 ship. cont.127–129 Local-LLM (shim+probe+picker) gated (`docs/design/local-llm.md`). cont.119 minimal-core strip (3 workspaces) → v0.9.0. cont.94 Fable 5. PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Local LLM page is a cockpit** (cont.139) — status rail + config grid; readiness state machine drives the tinting. Frontend-only; backend pass not started.
- **PageHero is the shared hero** (cont.138) — Settings + Local LLM consume `shared/PageHero.svelte`; page-local `.sb-chip` variants stay per-page. Don't re-fork the hero chrome.
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills.
- **Tool-group grouping (cont.121):** `coalesceToolGroups`; open = `expandedGroups.has(key) !== defaultOpen` (XOR).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163). **Versions lockstep ×3 + Cargo.lock** — only at ship.
- **IA: 3 core workspaces** (Home·Chat·Settings) + **experimental Local LLM** (kbd 4, gated).

## Live state pointer
Read this + `docs/ISSUES.md` before assuming state. v0.12.0 shipped. Next: #39 P0-3 + P2; Local LLM backend pass.
