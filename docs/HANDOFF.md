# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.140) — #39 security hardening + dead-code cleanup (PR #5, unmerged)

All work lives on `claude/project-status-update-g5z7cl` — **not yet merged to main**. Three batches:

**Batch 1 — helper dedup (svelte-check 0/0 · vitest 162✓):**
- `shortPath` → `tabsbar/helpers.ts` (+test); `HomePage` + `AssistantWelcome` drop local copies.
- `basename` exported from `toolCaption.ts`; `ToolChip` drops duplicate `basenameOf`.
- `HistoryDrawer` uses `statsHelpers.modelLabel` (richer id→label map).

**Batch 2 — security hardening (cargo check clean · 2 Rust tests✓):**
- `local_llm_base_url` → http(s)+host validation at setter + `turn.rs` sink (`is_valid_local_base_url`).
- `convo_store.rs assistant_export_save` — extension allowlist (`.md/.json/.txt`).
- `git_local.rs` — `GIT_CONFIG_GLOBAL` + `GIT_CONFIG_SYSTEM` added to env_remove block.
- `capabilities/default.json` — `http://**` dropped from `opener:allow-open-url`, https-only.
- `usage/limits.rs` — `read_oauth_token()` wrapped in `spawn_blocking` (#31).

**Batch 3 — token/color cleanup (svelte-check 0/0):**
- Dead `var(--danger,#e66)` / `var(--warn,#e2b340)` fallbacks stripped.
- `AssistantWelcome` + `EnhanceBar` text-on-accent → `var(--accent-fg)`.
- `ToolChip` `var(--ok, ...)` fallback dropped.

**RESUME:** Merge PR #5. Then: #39 P0-3 (unify CLI-update notice) + P2 (size tokens); Local LLM backend pass; #37 multi-window Route A MVP.

## Session 2026-06-15 (cont.139) — Local LLM cockpit → SHIPPED v0.12.0

Rebuilt Local LLM page into a status-driven cockpit. Pruned stale ISSUES. Shipped `v0.12.0`.

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.140 → PR #5** Security + dead-code cleanup (unmerged — on branch `claude/project-status-update-g5z7cl`).
- **cont.139 → v0.12.0** Local LLM cockpit.
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
Read this + `docs/ISSUES.md` before assuming state. v0.12.0 shipped. PR #5 open (security/cleanup). Next after merge: #39 P0-3 + P2 (needs desktop); Local LLM backend pass; #37 multi-window Route A.
