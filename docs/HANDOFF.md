# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-13 (cont.122) — Release-readiness re-verify + Activity dock removal

**1. Release-readiness audit hardened** (`docs/release-readiness-2026-06-13.md`, untracked). All 🔍 findings re-grepped against `main` via 4 parallel verifiers + manual. Corrections:
- **RR-3 REVERSED** — `registry.npmjs.org` is NOT dead; `cliUpdate.svelte.ts:30` fetches it at runtime (CLI-update check). CSP entry load-bearing — do NOT remove. Dropped from fix list.
- **B5 (`auth_update.rs`) + B9 (`convo_store.rs`) cite deleted files** (gone in the strip). B9's real `remove_file` = `state/paths.rs:69,90` (benign tmp cleanup, symptom N/A). Drop both.
- **B3 weaker** — `:881` has `clear_session_pid` mitigation; only `:868` stop-during-spawn arm exposed. **RR-2 mechanism** refined (panic hook uses log::error!/emit_with_fields, not tx.send). Rest confirmed.
- CI: v0.9.2 run `27477310839` = success, prod auto-update live.

**2. Activity dock fully removed** (`ced39af`, −1240 lines). `ActivityPanel.svelte` deleted + dock aside/resize/state/persistence/Ctrl+Shift+E purged. Surfaces were duplicated (pills/gauge/tabsbar/transcript/Ctrl+Shift+D). Plan/Tasks + Sources dropped per user call. `dockAutoOpenedThisConvo`/`opensDock` left inert in streaming.ts (out of scope). svelte-check 0/0, vitest 131/131.

**3. Steps numbering unified** (in `ced39af`) — group `.tg-num` now echoes single-node `.tl-stepdot` status-ring. Card+left-rail design (cont.121) untouched.

### RESUME HERE — next arc: T1 ship-blockers (full re-verified plan in release-readiness doc)
1. **RR-1** — new-user auth dead-end (T1). `AssistantWelcome.svelte:163-176` needsAuth card is static (no live Sign-in). Add Sign-in (→`assistant.startLogin()`) + Re-check buttons mirroring the recovery banner in `AssistantPane.svelte`. All 4 sub-claims confirmed.
2. **RR-2** — no field crash file (T1). Write dedicated `crash-<ts>.txt` on panic (`diagnostics/mod.rs` hook) — don't reuse the rotating `rift.log` slot.
3. Then T2: RR-4 (`opener:allow-open-path` `**` scope), RR-6/7/9, RR-5 (#29 CSP — needs prod build).
4. Deferred steps polish (my-call, not bugs): drop redundant wrench in group head · `captionForGroup` "Running N actions" vagueness · card treatment for single/double nodes.
5. **Composer bug** — user spotted one above the composer, deferred; have them re-point when ready.

## Prior arcs — detail in git log + CHANGELOG
cont.121 Concept-D tool-group cards + auto-correct → v0.9.2. cont.120 UI Polish §1-§6 → v0.9.1. cont.119 minimal-core strip (−7,407 → 3 workspaces) → v0.9.0. **§7 Harness rebuild still OPEN**. cont.94 Fable 5 (Jun 22 sunset gate). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups` absorbs quick thoughts; threshold = TOOL count. Open = `expandedGroups.has(key) !== defaultOpen` (XOR), stores FLIPPED-from-default keys. Card + left status-rail (`::after`), NOT spine bullet — don't re-add a spine bullet to groups (steps-numbering unify kept this).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 workspaces** (Home·Chat·Settings). **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.9.2 stands.**
