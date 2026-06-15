# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-15 (cont.143) — self-update brick fixed → SHIPPED v0.12.3

User's in-app update + reinstall failed: *"Failed to remove existing application directory."* Diagnosed live from `%LOCALAPPDATA%\velopack\velopack_Rift.log` + Sysinternals `handle64`.

**Root cause:** Rift running with **no workspace open** spawned the Claude CLI with no cwd set → child inherited Rift's install dir (`…\Rift\current\`). Its SessionStart hook launched the **Pulse daemon** (`projects/pulse/run.py`, `disown`ed per `session-start.sh:302`) with that cwd. The daemon survived app exit and held `current\` locked → Velopack can't rename it → every update/reinstall bricked. Velopack's apply-reap only kills `rift-tauri.exe`, never the orphaned out-of-tree daemon.

**Machine fix (done live):** killed lockers by PID (pulse + bash-hook parent), removed orphan `%LOCALAPPDATA%\Rift`, relaunched Pulse from a safe cwd (`/api/recall` 200 on :7878). User can reinstall now.

**Code fix:** [turn.rs](src-tauri/src/assistant/turn.rs) — Claude CLI spawn now `current_dir(temp_dir())` by default, overridden to workspace root when present. Child can never inherit the install dir; prevention (not reap) is the only reliable cut for a disowned daemon. `cargo check` clean.

**RESUME:** clean slate. After v0.12.3 release CI lands, confirm the user's reinstall succeeds. Then PR #6 (CLI-update test net) review/merge + CDP-gated backlog.

## Recent committed/shipped — detail in git log + CHANGELOG + `docs/ISSUES.md`
- **cont.142 → v0.12.1 + v0.12.2** split-pane send fix (#41) + STT polish UX (#40) + double-timer (#39 P0-4); then DOMPurify `3.4.3→^3.4.10` security bump (check CI gate). Both releases + check CI green.
- **cont.141 → PR #6** (open) CLI-update test net: +23 vitest cases for `cliUpdate.svelte.ts`, exported `CliUpdate` for isolated tests (singleton unchanged), no runtime change. PR #5 merged (`300b6e0`).
- **cont.140 → `300b6e0`** #39 security hardening (local_llm base_url validation · export ext allowlist · git_local env strip · opener https-only · oauth read `spawn_blocking`) + helper dedup + dead color-var cleanup.
- **cont.139 → v0.12.0** Local LLM cockpit redesign.
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
Read this + `docs/ISSUES.md` before assuming state. **v0.12.3 shipped** (self-update brick fix; release CI pending confirm). PR #6 open (CLI-update test net). Next: confirm user reinstall succeeds; #39 P0-3 + P2 (needs desktop CDP); Local LLM backend pass; #37 multi-window Route A.
