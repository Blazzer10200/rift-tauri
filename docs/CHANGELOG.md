# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.32.0 — A cleaner Workspace, and start from a folder you already have

> The Workspace page got a ground-up redesign — calmer, more modern, fits one screen with no scrolling — and you can now point Rift at a project you started long before Rift existed. Plus a codebase cleanup pass and a full documentation refresh.

- **Redesigned Workspace page.** Cleaner layout that blends with the rest of the app, fits a normal window without scrolling, and drops the duplicated greeting/time logic that used to live in two places. Resume-a-chat cards, current-folder context, and the project manager are all there, just tighter.
- **Adopt an existing project.** Already have a folder you've been working in? Rift now surfaces your recent folders as one-tap "add this as a project" entries — from a banner when you're sitting in an un-adopted folder, from the project grid, and from a recent-folders dropdown right in the project editor. No need to have started the project inside Rift.
- **Codebase cleanup + docs refresh.** A verified sweep removed dead code, two unused dependencies, and four orphaned files (suite + build still green), and every living doc was audited against the source with 16 confirmed drift errors fixed — so the docs now match what the code actually does.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.31.2** — Project editor that tells you what's wrong: live inline glob validation (red outline + invalid-count, Save disabled until fixed), plain-English save/delete errors instead of doubled jargon, and a failed project load now shows "Couldn't load projects" + Retry instead of a misleading "No projects yet." Plus a 100+-case test net over the previously-untested project paths.

- **v0.31.1** — Project globs that actually match: `**` now spans zero-or-more folders (so `src/**/*.ts` catches top-level files and `vendor/**` hides the folder itself), invalid patterns are rejected at save time, and the stall watchdog can no longer re-arm forever while a tool hangs.

- **v0.31.0** — Projects, and a single Workspace home: named folder aliases with their own include/exclude file scoping, switchable in a click; Home and Projects merged into one **Workspace** page (greeting, current-folder context, resume-a-chat cards, full project manager); and honest stalled-turn handling that names the real cause instead of blaming the API.

- **v0.30.1** — Hardening + accessibility polish: Windows full-path fix in the git tools, defense-in-depth size/input caps, executable-open refusals, and proper screen-reader labels/roles across dialogs, menus, and the palette.

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab that reads how you actually use Claude through Rift, then asks **your own Claude** for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass (no new features): a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing, more rejected executable types, ring-capped buffers, torn-down stray timers/listeners, and a pile of correctness fixes. It just holds up better the longer you run it.

- **v0.27.0–v0.27.1** — Activity stats panel (streaks, peak hour, 12-week heatmap, per-model breakdown), drag a sidebar chat into a split pane, faster Smart-mode replies that stream immediately, finished split-view (no overlap at 3–4 panes, per-pane folders), and calmer collapsed tool rows.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
