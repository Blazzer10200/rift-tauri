# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.33.0 — Rift runs as your Claude Code

> Turn on **Use my full Claude Code config** and Rift now genuinely runs as *your* Claude: it inherits your global `~/.claude` setup — `CLAUDE.md`, `settings.json`, hooks, slash commands, skills, and custom MCP servers — exactly the way the `claude` terminal does. Plus a cleaner Workspace + Activity panel, and a safer release pipeline.

- **Use my full Claude Code config — now actually full.** The setting always promised to layer your global `~/.claude/CLAUDE.md` into every turn, but a hard-coded flag quietly dropped the `user` setting source, so your global config, `settings.json`, and hooks never loaded. Fixed: with the toggle on, Rift runs with `--setting-sources user,project,local` — your full setup rides along. Off is a clean sandbox (Rift's own MCP tools only, no global config or hooks). API-key and local-LLM modes stay sandboxed by design.
- **Tools that work without a folder open.** A chat with no project open used to disable *every* tool. Now, when you're running your full Claude Code config, a no-folder chat still gets your workspace-independent tools — slash commands, skills, web search — so it behaves like `claude` in an empty directory instead of a dead sandbox. (File and shell tools still require an open folder, for path safety.)
- **Cleaner Workspace + Activity panel.** Another pass on the Workspace page and the sidebar's New-chat flow, plus a reworked Activity/Sub-agents panel — a floating top-right card that collapses to a small pill instead of a full-height sidebar, and the activity stats dashboard (streaks, peak hour, 12-week heatmap, per-model breakdown) wired up and reachable.
- **Off-tab prompts that actually respond.** An `ask_user` question or permission prompt raised while you'd switched to another tab used to render but stay dead — buttons disabled, stuck on "Connecting to the chat session…". Fixed: prompts now resolve from whichever tab owns the turn, so they work from any tab or pane.
- **A release can't ship red tests anymore.** The tag-driven release pipeline now runs the full suite (`cargo test` + `svelte-check` + `vitest`) *before* it builds and publishes — closing the gap that let a build with failing tests ship once. Plus a fixed Git-tools trust toggle that now takes effect immediately mid-session instead of waiting for an unrelated change.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.32.0** — A cleaner Workspace, and start from a folder you already have: a ground-up Workspace redesign (calmer, fits one screen, no scroll), adopt-an-existing-folder as a one-tap project from your recent folders, a verified dead-code/dependency cleanup sweep, and a full documentation refresh (16 confirmed drift errors fixed against source).

- **v0.31.2** — Project editor that tells you what's wrong: live inline glob validation (red outline + invalid-count, Save disabled until fixed), plain-English save/delete errors instead of doubled jargon, and a failed project load now shows "Couldn't load projects" + Retry instead of a misleading "No projects yet." Plus a 100+-case test net over the previously-untested project paths.

- **v0.31.1** — Project globs that actually match: `**` now spans zero-or-more folders (so `src/**/*.ts` catches top-level files and `vendor/**` hides the folder itself), invalid patterns are rejected at save time, and the stall watchdog can no longer re-arm forever while a tool hangs.

- **v0.31.0** — Projects, and a single Workspace home: named folder aliases with their own include/exclude file scoping, switchable in a click; Home and Projects merged into one **Workspace** page (greeting, current-folder context, resume-a-chat cards, full project manager); and honest stalled-turn handling that names the real cause instead of blaming the API.

- **v0.30.1** — Hardening + accessibility polish: Windows full-path fix in the git tools, defense-in-depth size/input caps, executable-open refusals, and proper screen-reader labels/roles across dialogs, menus, and the palette.

- **v0.29.0** — AI Health: a coach for your Claude plan. New workspace tab that reads how you actually use Claude through Rift, then asks **your own Claude** for a few plain-English, ranked suggestions grounded in your real numbers — with one-tap, fully-undoable apply. Only ever touches Rift's own settings, never your global Claude config.

- **v0.28.0** — Top-to-bottom hardening pass (no new features): a multi-wave multi-agent review swept the whole app (145 confirmed findings) and every fix landed here — prompt-injection sanitizing, more rejected executable types, ring-capped buffers, torn-down stray timers/listeners, and a pile of correctness fixes. It just holds up better the longer you run it.

- **v0.27.0–v0.27.1** — Activity stats panel (streaks, peak hour, 12-week heatmap, per-model breakdown), drag a sidebar chat into a split pane, faster Smart-mode replies that stream immediately, finished split-view (no overlap at 3–4 panes, per-pane folders), and calmer collapsed tool rows.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
