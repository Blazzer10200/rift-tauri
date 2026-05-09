# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** WPF→Tauri migration of Rift. Sibling repo to `Blazzer10200/rift` (WPF v13.55.x line still ships from there). User's daily app = WPF Rift. This project = the v14.0.0 future.

**🚨 PATH SANITY CHECK — DO THIS FIRST:** Confirm `pwd` is `C:/AI Workflow/rift-tauri/`. The sibling WPF repo at `C:/AI Workflow/Rift Project/` is **stable port-from reference** — DO NOT refactor it unless the user explicitly says "WPF" or "v13.x". When user says "rift" or "the rift project", default to `rift-tauri`.

**🚨 FIRST THING NEXT SESSION — AWAIT CLAUDE DESIGN HANDOFF.** UI foundation (Tailwind v4 + shadcn-svelte) shipped in v0.1.5-alpha. User opened a "Rift App UI" project in Anthropic Labs' Claude Design (claude.ai/design) using `docs/design/CLAUDE-DESIGN-BRIEF.md` as the foundational context, took screenshots of the running dev build, and is iterating on direction prompts there. **Once they paste/drop the exported deliverable bundle here, translate the OKLCH tokens into `src/app.css` and apply per-component visual decisions across the existing dialogs + shell.** Suggested order: (1) Confirm dialog as proof-of-concept → (2) remaining 5 dialogs → (3) AppShell/TopBar/ServerPicker → (4) Browser two-pane.

**Current state (post Session 10, v0.1.5-alpha):** Migration core complete and committed (`5b9f5f7`, v0.1.4). UI foundation + Claude Design brief committed at session close. Phase 6 (v14.0.0 ship) still deferred per user.

**Release policy:** No public ship until user explicitly says go. Iterate via `npm run tauri dev`.

## Verified phase status

| Phase | Scope | Status |
|---|---|---|
| 0 + 1a–1h, 1j | Backend (state, sftp, sync engine, tunnel, tail services) | ✅ |
| 1i | ConfigStore write-back + TOFU fingerprint persist | ✅ |
| 2 | UI shell — MainWindow port (Svelte 5 runes) | ✅ |
| 3 | Two-pane browser | ✅ |
| 4 | Sync surfaces (activity, drift, conflicts, locks, edit-in-place) | ✅ |
| 5 | Dialogs — AddServer / Bootstrap / Keygen / Reupload / Confirm / CommandPalette | ✅ |
| **5.5** | **UI redesign foundation — Tailwind v4 + shadcn-svelte + Claude Design brief** | ✅ Session 10 |
| 6 | v14.0.0 ship | ⏳ deferred |

## Session 10 — 2026-05-09 — Backlog commit + UI foundation + Claude Design brief

### Backlog commit (the FIRST THING from Session 9 HANDOFF)
Sessions 2–9 bundled into `5b9f5f7 Phases 1-5 + 1i — migration core complete (v0.1.4-alpha)` on top of bare Phase 0 scaffold. `build/` (SvelteKit static output) added to `.gitignore`.

### UI redesign foundation
After web research (cross-referenced r/sveltejs Oct 2025–Apr 2026, dev.to, svelte.dev blogs), settled on **shadcn-svelte (nova style, zinc base) + Tailwind v4 + Bits UI primitives + Svelte 5 native transitions**, with motion-sv held in reserve for layout animations. Skipped Skeleton/Flowbite per community sentiment.

Files touched:
- `vite.config.js` — `@tailwindcss/vite` plugin
- `src/app.css` — full OKLCH zinc theme (light + dark) + `@theme inline` mappings
- `src/app.html` — `class="dark"` for first-paint
- `src/routes/+layout.svelte` — `<ModeWatcher defaultMode="dark" />`
- `src/lib/utils.ts` (NEW) — `cn` + `WithElementRef`/`WithoutChild` types
- `components.json` (NEW) — shadcn-svelte CLI config
- `src/lib/components/ui/button/` (NEW, smoke-test only) — proves CLI pipeline works

### Claude Design brief
`docs/design/CLAUDE-DESIGN-BRIEF.md` — single-shot foundational context for claude.ai/design. Avoids the token-burn of "Attach codebase" by pre-digesting product summary, tech constraints, component inventory, current OKLCH tokens, non-goals, and 4 direction prompts (Linear / Raycast / Sublime / Win11-Mica). User created a "Rift App UI" project there using the brief as the Design System input, screenshots as the visual reference.

### Verified
`npm run check` 318 files 0/0 ✓ · `npm run tauri dev` boots clean ✓ · cargo left untouched.

### Bumped
0.1.4 → 0.1.5 across Cargo.toml / package.json / tauri.conf.json. v0.1.4 entry archived.

## CRITICAL DON'T-TOUCH (carries forward)
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only — never enable `default-tls`/`native-tls` (drags openssl)
- npm runner, NOT pnpm
- File-format compat w/ WPF `~/.rift/*.json` — never change PascalCase/camelCase rename rules; never drop the `serde(flatten) extra` bag on `RiftConfig`
- Velopack `VelopackApp::build().run()` first call in `run()`
- shadcn-svelte components own themselves in `src/lib/components/ui/` — don't refactor into a shared abstraction; that defeats the code-ownership model
