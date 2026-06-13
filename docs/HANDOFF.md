# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-13 (cont.123) — Release-readiness ship-blockers + robustness → v0.9.3

**SHIPPED v0.9.3** (tag-driven CI, run `27481935945`). Cleared the release-readiness audit (`docs/release-readiness-2026-06-13.md`) — both T1 blockers, all T2 should-fixes, the actionable T4 swallows:
- **T1 RR-1** new-user auth dead-end: live **Sign in** (`startLogin`) + **Re-check** on the `needsAuth` welcome card (`AssistantWelcome.svelte`), mirroring the recovery banner; dead "hit refresh" text fixed.
- **T1 RR-2** field crash file: panic hook writes a dedicated non-rotating `crash-<ts>.txt` (version+location+scrubbed backtrace) — survives a 2nd crash + pre-setup startup panics. `diagnostics::write_crash_report`, called from the `lib.rs` hook.
- **T2:** RR-4 open-path denies OS-execute exts (exe/msi/bat/…) · RR-6 steer-drain surfaces write/build errs · RR-7 oneshot surfaces stderr JoinError · RR-9 download epoch-guard kills zombie-after-stall `downloaded=true`.
- **Polish:** dropped wrench glyph from tool-group heads; mixed groups lead w/ dominant kind ("Reading 3 files +1 more", `toolCaption.ts`+vitest).
- **T4 sweep:** B3(:868 kill log)·B4(unreachable-UI perm→fast deny)·A1(killed `.expect()` panic)·B7(bridge session_id warn)·B10(STT panic log)·B11(log-rotation size cap)·RR-14(stale comment). Skipped B1(flush noise)+A4(cosmetic).

Verify: svelte-check 0/0 · vitest 12/12 · cargo check clean 0.9.3 · CDP live (boots 0 errors, wrench-free heads + captions render).

### RESUME HERE — v0.9.3 shipped; remaining audit tail (none are code-only-mine)
1. **RR-5 / #29** CSP prod-verify — **v0.9.3 IS a prod build**: install it, confirm transitions animate + update progress-bar fills + 0 CSP console violations.
2. **RR-8 / Permission** Allow/Deny round-trip — still needs `trust_level=standard` pinned on a throwaway repo (one-way pin → do deliberately). Fold into the CR-UX trust rework.
3. **Your decisions:** RR-10 `ALLOW_PRERELEASE=true` for the wider cohort? · RR-11 code-signing? · RR-12 repo collapse (#17).
4. **Deferred polish:** card treatment for single/double-node groups — visual call, left for your eye (don't regress cont.121 card design).
5. **Composer bug** — user spotted one above the composer; re-point when ready.

## Prior arcs — detail in git log + CHANGELOG
cont.122 release-readiness re-verify + Activity dock removal (`ced39af`). cont.121 Concept-D tool-group cards + auto-correct → v0.9.2. cont.120 UI Polish §1-§6 → v0.9.1. cont.119 minimal-core strip (−7,407 → 3 workspaces) → v0.9.0. **§7 Harness rebuild still OPEN**. cont.94 Fable 5 (Jun 22 sunset gate). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups` absorbs quick thoughts; threshold = TOOL count. Open = `expandedGroups.has(key) !== defaultOpen` (XOR), stores FLIPPED-from-default keys. Card + left status-rail (`::after`), NOT spine bullet — don't re-add a spine bullet to groups (steps-numbering unify kept this).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 workspaces** (Home·Chat·Settings). **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.9.2 stands.**
