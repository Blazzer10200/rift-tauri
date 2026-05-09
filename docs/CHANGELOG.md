# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.1.5-alpha — 2026-05-09 — UI redesign foundation: Tailwind v4 + shadcn-svelte + Claude Design brief

UI redesign substrate laid. No public ship; Phase 6 still deferred. Migration core (v0.1.4) remains functionally complete; this version adds the visual-iteration foundation and the Claude Design handoff package.

### Frontend stack additions
- **Tailwind v4** wired via `@tailwindcss/vite` plugin in `vite.config.js`.
- **shadcn-svelte (nova style, zinc base)** initialized — `components.json` configured for `$lib/components/ui` aliasing.
- **Full OKLCH theme tokens** (light + dark zinc palette, `@theme inline` mappings) replace the bare `#0F0F12`/`#E8E8EE` body styling in `src/app.css`. Designer can replace the entire palette without breaking component contracts.
- **Dark-first** — `class="dark"` baked into `src/app.html` for first-paint, `<ModeWatcher defaultMode="dark" />` mounted in `+layout.svelte` for runtime toggling.
- **Smoke test** — `button` component scaffolded via `npx shadcn-svelte add button` to validate end-to-end pipeline.

### Dep additions
- devDeps: `tailwindcss`, `@tailwindcss/vite`, `tw-animate-css`.
- deps: `clsx`, `tailwind-merge`, `mode-watcher`, `bits-ui`, `tailwind-variants` (auto-pulled by button).

### Helpers
- `src/lib/utils.ts` — `cn` helper + `WithElementRef` / `WithoutChild` / `WithoutChildren` types (required by shadcn-svelte v1.2.7+).

### Documentation
- `docs/design/CLAUDE-DESIGN-BRIEF.md` — token-efficient one-shot context for Anthropic Labs' Claude Design (claude.ai/design). Pre-digests product summary, tech constraints, full component inventory, current OKLCH tokens, non-goals, and 4 starter direction prompts (Linear-precise / Raycast-dark-glass / Sublime-terminal / Win11-Mica). Avoids the codebase-attach token-burn.

### Verified
`npm run check` 318 files 0 errors 0 warnings ✓ · `npm run tauri dev` boots clean (Vite 701 ms, cargo cached) ✓.

### Versions bumped
`Cargo.toml`, `package.json`, `tauri.conf.json` → 0.1.5. v0.1.4 entry archived to `archive/CHANGELOG-archive.md`.
