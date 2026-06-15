# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.141) — merged PR #5, added CLI-update test net (PR #6)

Buddy session while user was away. **Merged PR #5** (`claude/project-status-update-g5z7cl`, rebase → `300b6e0`) after independent re-verify on a local checkout: `cargo check` clean · `cargo test config::` 2✓ · svelte-check 0/0 (4094) · vitest 162✓ — every claim in the PR matched. Branch deleted.

Then **opened PR #6** (`chore/cliupdate-test-coverage`): +23 vitest cases for `cliUpdate.svelte.ts` (semver compare, multi-install drift, dismissal gating, 5-way `summary` precedence) — the load-bearing update-detection path had zero coverage. Exported `CliUpdate` for isolated test instances (singleton unchanged). vitest 185✓ · svelte-check 0/0 (4095). **No runtime behavior change.**

Investigated #39 P0-3/P2 to scope safe headless work — found the safe items are exhausted: store-level dedup (`summary`/`commandFor`/`isAnyStale`) already done so P0-3 is genuinely the per-surface visual extraction (needs CDP); `.sb-bento` rename moot (now Settings-only); scrollbar no-op premise likely stale (Chromium ≥121). Recorded in ISSUES.

**RESUME:** Review/merge PR #6. Then the CDP-gated backlog: #39 P0-3 (unify CLI-update notice) + P2 (size tokens); Local LLM backend pass; #37 multi-window Route A MVP.

## Session 2026-06-15 (cont.140) — #39 security hardening + dead-code cleanup (PR #5, MERGED → `300b6e0`)

Three batches, merged cont.141 (detail in `git log 300b6e0`):
- **Helper dedup:** `shortPath` → `tabsbar/helpers.ts`; `basename` ← `toolCaption.ts`; `HistoryDrawer` → `statsHelpers.modelLabel`.
- **Security:** `local_llm_base_url` http(s)+host validation (setter + `turn.rs` sink); `assistant_export_save` ext allowlist; `git_local.rs` strips `GIT_CONFIG_GLOBAL/SYSTEM`; `opener:allow-open-url` https-only; `read_oauth_token()` → `spawn_blocking`.
- **Cleanup:** dead `var(--danger/--warn,#hex)` fallbacks stripped; text-on-accent → `var(--accent-fg)`.

## Session 2026-06-15 (cont.139) — Local LLM cockpit → SHIPPED v0.12.0

Rebuilt Local LLM page into a status-driven cockpit. Pruned stale ISSUES. Shipped `v0.12.0`.

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.141 → PR #6** CLI-update test net (open). **cont.140 → `300b6e0`** Security + dead-code cleanup (PR #5 merged).
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
Read this + `docs/ISSUES.md` before assuming state. v0.12.0 shipped; PR #5 merged (`300b6e0`). PR #6 open (CLI-update test net). Next: #39 P0-3 + P2 (needs desktop CDP); Local LLM backend pass; #37 multi-window Route A.
