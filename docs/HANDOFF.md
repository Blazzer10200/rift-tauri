# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.142) — user-spotted fixes → SHIPPED v0.12.1

Issue-logging session that turned into a ship. Three user-reported quirks fixed, all frontend:
- **#41 (T1) split-pane send routing** — a pane's message landed in the wrong pane. `send()` keyed off global `currentConvoId`; the composer fired with no tabId. Fix: `assistant.send(prompt, tabId?)` retargets `currentConvoId` to the firing pane's tab synchronously before `sendImpl`; `AssistantPane` passes `tabId`. (Drafts/attachments were already pane-correct.)
- **#40 STT polish UX** — dictation shimmer ran up to the backend's 15s cap. Fix: 6s frontend `SHIMMER_CAP_MS`, `cancelPolish()` on typing (guard token kills the late swap), skip redundant `onEnd` rewrite.
- **#39 P0-4 double timer** — role-row `9s` heartbeat now yields to a quiet dot while a thinking block is active (thinking pill owns the only ticking number in that phase).

Verify: svelte-check 0/0 (4094) · vitest 162✓ · live app 0 console errors. Shipped **v0.12.1** (release CI ✅).

Then **v0.12.2** — the `check` workflow's `npm audit --omit=dev` gate failed on a pre-existing moderate DOMPurify advisory (`dompurify <=3.4.8`, XSS vectors; backs Markdown `{@html}`). Bumped `dompurify 3.4.3 → ^3.4.10` (same-minor, no API change). Prod audit → 0 vulns. Shipped **v0.12.2** — release CI ✅ **and** check CI ✅ (both green). Note: `check.yml` runs `npm audit --omit=dev` as a hard gate — a new prod advisory will fail every push until the dep is bumped.

**RESUME:** clean slate. PR #6 (CLI-update test net) review/merge, then the CDP-gated backlog below.

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.142 → v0.12.1 + v0.12.2** split-pane send fix (#41) + STT polish UX (#40) + double-timer (#39 P0-4); then DOMPurify `3.4.3→^3.4.10` security bump (check CI gate). Both releases + check CI green.
- **cont.141 → PR #6** (open) CLI-update test net: +23 vitest cases for `cliUpdate.svelte.ts`, exported `CliUpdate` for isolated tests (singleton unchanged), no runtime change. PR #5 merged (`300b6e0`).
- **cont.140 → `300b6e0`** #39 security hardening (local_llm base_url validation · export ext allowlist · git_local env strip · opener https-only · oauth read `spawn_blocking`) + helper dedup + dead color-var cleanup.
- **cont.139 → v0.12.0** Local LLM cockpit redesign.
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
Read this + `docs/ISSUES.md` before assuming state. **v0.12.2 shipped** (latest; release + check CI both green). PR #6 open (CLI-update test net). Next: #39 P0-3 + P2 (needs desktop CDP); Local LLM backend pass; #37 multi-window Route A.
