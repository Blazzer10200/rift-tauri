# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.138) — UI consistency batch → SHIPPED v0.11.0

Shipped the UI-review §3 P0/P1 backlog and folded in cont.135/137's in-flight work. All committed; tagged **`v0.11.0`** → CI building (`release.yml` → rift-releases).
- **Shared `PageHero`** (`src/lib/components/shared/PageHero.svelte`) — Settings + Local LLM now consume one hero source; killed the copy-pasted chrome + the 880→820px `.sb-wrap` drift. [P0-2]
- **P0-1** already in the recommended state — Local LLM header pill = status `<span>`, Mode-card switch = the control. No functional change needed.
- **Home** (`HomePage.svelte`): Quick-actions card fills the dead right column; hero "New chat" → "＋ New tab" (launcher stays primary). **Nav** (`Titlebar.svelte`): experimental amber `.exp-dot` on the Local LLM item; Settings gear tooltip shows Ctrl+3 / Ctrl+,. [P1-4/5/6/7]
- **Verified:** svelte-check 0/0 (4094 files) · Settings + Local LLM heroes live-CDP-verified · `Cargo.lock` auto-bumped to 0.11.0 by the running dev.
- **Thinking-display finding (closed):** Opus 4.8 thinking text can't stream — model default `thinking.display:"omitted"` + CLI 2.1.177 exposes no override flag. Sonnet streams it because its default is `"summarized"`. `turn.rs:791` comment corrected (was wrongly "-p mode encryption"). Rift's pipeline already renders `thinking_delta` — works for free if the CLI ever adds a display flag.
- **DEFERRED → `ISSUES.md #39`:** P0-3 unify the CLI-update notice (3 surfaces — touches the **Velopack** update path, verify carefully); P2 shared size/color tokens + `.sb-bento` rename.
- **NOTE:** two operator subagents bailed mid-task (returned stale "result" text, skipped the required `npm run check`); finished + verified inline. Prefer inline + CDP-verify for UI work.
- **RESUME:** P0-3 + P2 from ISSUES #39 when ready.

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.138 → v0.11.0 SHIPPED** (this session). Commits: `ab1eeb9` drag-split + non-blocking STT (was cont.135), `22272cb` live-status→composer (was cont.137), `fd10609` thinking-comment fix, `08acc86` PageHero/Home/nav feature.
- **cont.136** `a3ab764` live sub-agent activity dock (Chat right-edge; `parent_tool_use_id` routing in `streaming.ts`/`SubAgentDock.svelte`/`activityDock.svelte.ts`).
- **cont.134 → v0.10.0** Home stats dashboard, Fable disabled behind `FABLE_DISABLED` kill-switch, audit-hardening (`086e403`).
- **cont.131–133** per-pane STT routing + per-tab workspace root, `.pane-head`, Home `assistant_stats`.

## Prior arcs — detail in git log + CHANGELOG
cont.130 v0.9.5 R2 ship. cont.127–129 Local-LLM (shim+probe+picker) gated/unshipped (`docs/design/local-llm.md`). cont.123 → v0.9.3. cont.121 → v0.9.2. cont.119 minimal-core strip (3 workspaces) → v0.9.0. cont.94 Fable 5. PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **PageHero is the shared hero** (cont.138) — Settings + Local LLM consume `shared/PageHero.svelte`; page-local `.sb-chip` variants stay in each page (snippet CSS scopes to the defining component). Don't re-fork the hero chrome.
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups`; open = `expandedGroups.has(key) !== defaultOpen` (XOR). Card + left status-rail, NOT a spine bullet.
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 core workspaces** (Home·Chat·Settings) + **experimental Local LLM** (kbd 4, gated, not shipped). **Versions lockstep ×3 + Cargo.lock** — only at ship.

## Live state pointer
Before assuming current state, read this file + `docs/ISSUES.md`. v0.11.0 shipped; P0-3 + P2 remain in ISSUES #39.
