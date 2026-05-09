# Rift — Claude Design Brief

**Paste this whole file as the project context in Claude Design. Then attach 4-6 screenshots of the current UI (you'll grab those yourself).** Goal: give Claude Design everything it needs in one shot so it doesn't burn tokens scanning the codebase.

---

## Product, in one paragraph

Rift is a **Windows desktop SFTP workspace launcher** for FiveM/RedM game-server developers. It connects to a remote dev server over SSH, mounts a project tree, and runs a **two-pane file browser** (local ↔ remote) with **real-time bidirectional sync**, **drift detection**, **conflict resolution**, **edit-in-place**, and a **bridge tunnel** to txAdmin (the FiveM admin console). Users manage 1-N server profiles, switch fast, and live in the app for hours per day. Solo dev tool. No collaboration features. No web surface. **Native desktop app feel is the goal.**

## Tech constraints (don't fight these)

- **Tauri 2** — native window, custom titlebar candidate, Rust backend, ~10MB bundle
- **SvelteKit + Svelte 5 (runes)** — component framework
- **Tailwind v4 + shadcn-svelte (nova style, zinc base)** — already wired
- **OKLCH design tokens** in `src/app.css` — output should be deliverable as OKLCH variables that paste into our existing token block
- **Dark mode is default**, light mode supported via `mode-watcher`
- **Desktop-only** — no mobile/responsive concerns. Min window ~1280×800, optimal ~1600×1000
- **Custom titlebar is on the table** — we can do `decorations: false` for a polished native feel

## Aesthetic priors

- **Information-dense, not airy.** Closer to Linear/Raycast/Sublime than to a marketing site.
- **Keyboard-driven.** Ctrl+K command palette is central. Visual hierarchy should reward keyboard users.
- **Calm, not flashy.** Long sessions = no high-contrast spike colors, no animations on critical paths (file ops, sync status). Reserve motion for state transitions and feedback.
- **Status-aware.** Sync state, drift count, conflict count, lock presence — these need glanceable affordances.
- **Trust-signaling.** SSH fingerprints, server identity, drift severity should look serious, not toy.

## Component inventory (what needs design)

### Shell
- **AppShell** — top-level frame. Owns titlebar area, tab nav, command palette mount, dialog mount.
- **TopBar** — server picker dropdown, connection status, current-server identity, settings entry
- **StatusHero** — banner-style state indicator (connected / connecting / error / offline)
- **ServerPicker** — list of saved servers w/ Add/Edit/Delete + Setup-key launcher

### Tabs (5 main surfaces inside AppShell)
1. **Browser** — two-pane (local / remote) file browser. The hero surface, used most.
2. **Activity** — running operations feed (uploads, downloads, sync events, errors)
3. **Drift** — list of files differing between local and remote w/ resolve actions
4. **Conflicts** — interactive resolver for files modified on both sides
5. **Settings** — server config, SSH keys, bootstrap, preferences

### Browser surface (deepest design need)
- **TwoPane** — left local, right remote, draggable splitter
- **LocalPane / RemotePane** — file tree w/ icons, size, mtime, sync status badges
- **PathBreadcrumbs** — clickable path segments per pane
- **LockBadge** — overlay on locked files (other-user editing indicator)

### Dialogs (6 modal surfaces)
1. **AddServer** — 3-step stepper (Connection → Workspace → Bridge & Save). Per-step validation. Edit mode pre-fills.
2. **Bootstrap** — 6-state-aware UI for first-time remote sync (Synced / MissingLocalRoot / Empty / Uninitialized / Partial / BadRemoteRoot). Chunked download progress.
3. **Keygen** — generate/view SSH ed25519 keypair, copy pubkey
4. **Reupload** — Skip / Always / Re-upload triplet (edit-in-place autosync prompt)
5. **Confirm** — generic Yes/No w/ optional `isDanger` styling + "Don't ask again" checkbox
6. **CommandPalette** — Ctrl+K modal. Tokenized fuzzy search. ↑↓ nav, Enter run, Esc close.

### Other
- **ActivityToast** — transient notifications (top-right corner)
- **ConflictList / ConflictResolver** — inside Conflicts tab
- **DriftReview** — inside Drift tab

## Current theme (zinc base, OKLCH) — what to evolve from or replace

```css
/* dark mode tokens currently in src/app.css */
--background: oklch(0.141 0.005 285.823);     /* near-black w/ tiny purple tilt */
--foreground: oklch(0.985 0 0);                /* near-white */
--card: oklch(0.21 0.006 285.885);             /* card surface */
--primary: oklch(0.92 0.004 286.32);           /* near-white primary (inverted) */
--secondary: oklch(0.274 0.006 286.033);
--muted: oklch(0.274 0.006 286.033);
--muted-foreground: oklch(0.705 0.015 286.067);
--accent: oklch(0.274 0.006 286.033);
--destructive: oklch(0.704 0.191 22.216);      /* red */
--border: oklch(1 0 0 / 10%);
--ring: oklch(0.552 0.016 285.938);
--radius: 0.625rem;
```

**You can replace the entire palette.** This is a foundation, not a constraint.

## Non-goals (don't waste tokens here)

- ❌ Mobile or responsive design
- ❌ Marketing-site polish (hero sections, big illustrations, parallax)
- ❌ Light-mode-first (dark is the default; light is secondary)
- ❌ Decorative animations on critical paths (file ops, sync, conflicts)
- ❌ Custom font loading (system-ui is fine; only suggest a font swap if it materially improves the feel)
- ❌ Slide deck / PPTX export — this app ships as a Tauri binary

## Deliverable I need back (handoff bundle)

For each direction explored, provide:
1. **Theme tokens** — full OKLCH variable block (light + dark) ready to paste into `src/app.css`
2. **Per-component visual decisions** for at least: Browser (TwoPane), AddServer dialog, CommandPalette, StatusHero
3. **Typography choices** — font stack + size scale
4. **Iconography direction** — outline / filled / weight; library recommendation if not lucide
5. **Spacing/density rhythm** — base unit, gap scale
6. **Animation language** — what moves, what doesn't, easing curves
7. **Custom titlebar recommendation** — y/n + design if yes

## Direction prompts to start exploration

Pick 1-2, don't try all four:

> **A) Linear-precise.** Show the Browser two-pane and AddServer dialog as if Linear designed Rift. Information-dense, restrained color, sharp typography, subtle borders.

> **B) Raycast-dark-glass.** Show CommandPalette + StatusHero + Browser as if Raycast designed Rift. Frosted-glass dialog surfaces, dark deep navy, glowing accents on focus, monospace where it earns its place.

> **C) Sublime / terminal-modern.** Show Browser + Activity feed as if Sublime Text designed Rift. Mono-leaning typography, syntax-color status semantics (green=synced, yellow=drift, red=conflict), zero rounded corners or minimal radius.

> **D) Native-Windows-11-Mica.** Show AppShell + TopBar + Browser as a polished Windows 11 Mica app. Acrylic backdrop, native titlebar treatment, FluentUI-adjacent spacing and motion.

## After Claude Design — handoff back to me

Save the deliverable as a single `.md` or `.txt` (theme tokens + per-component notes), then drop it in this Claude Code session. I'll translate the tokens into `src/app.css` and apply the styling decisions across the existing Svelte components.
