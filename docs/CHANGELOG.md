# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.36.0 — One place for everything that happens

> Notifications used to flash once in the corner and vanish. Now there's a bell in the toolbar that keeps a tidy history of everything, and updates get their own always-visible bar at the top instead of competing with the noise. Plus a quieter, more polished chat surface.

- **A real notification center.** A bell in the toolbar with an unread badge opens a dropdown listing everything that's happened — grouped by when (Now / Earlier / Today / Older), each with its own icon, detail, and time. History persists across restarts (last 50, auto-clearing anything over a week old), so a toast you missed isn't gone forever. Mark-all-read and clear-all are one click.
- **Updates live at the top now, not in the corner.** Both kinds of update — the Rift app itself and the Claude CLI — surface in a clean, always-visible bar at the top of the window whenever one is available, showing exactly which version you're moving to. One click to update, and it shows live progress while it works. When there's nothing to update, the bar takes up zero space.
- **Redesigned toasts.** The transient pop-ups that still appear for in-the-moment feedback got a visual refresh — clearer severity coloring, a rounded icon tile, and a subtle lift on hover.
- **A calmer chat surface.** Long code blocks now collapse with a soft fade and a "Show more" pill instead of running on forever; the message stream got rhythm and spacing polish; and a stray cross-project "resume" card that didn't belong was removed.
- **Fixed: error spam on startup.** A reactivity loop in the app shell logged a burst of internal errors every time Rift booted. Gone — startup is clean.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock instead of bricking until reinstall, CLI-update checks read as "Checking…" instead of showing a stale result, and git-timeout kills are scoped to git specifically.

- **v0.34.0** — Windows that stay in sync: open Rift as separate native windows (one per monitor) and the conversation list stays synced across all of them (create/rename/delete in one refreshes the rest; each keeps its own tabs). Secondary windows got the main window's shell-open file-type guards, and the release pipeline stopped marking successful releases as failed.

- **v0.33.0** — Rift runs as your Claude Code: turn on **Use my full Claude Code config** and Rift inherits your global `~/.claude` setup (`CLAUDE.md`, `settings.json`, hooks, slash commands, skills, MCP servers) exactly like the `claude` terminal (`--setting-sources user,project,local`; off is a clean sandbox). No-folder chats keep workspace-independent tools. Cleaner Workspace + Activity panel. Off-tab `ask_user`/permission prompts now respond.

- **v0.32.0** — A cleaner Workspace, and start from a folder you already have: a ground-up Workspace redesign (calmer, one screen, no scroll), adopt-an-existing-folder as a one-tap project, a verified dead-code/dependency cleanup sweep, and a full documentation refresh.

- **v0.31.2** — Project editor that tells you what's wrong: live inline glob validation (red outline + invalid-count, Save disabled until fixed), plain-English save/delete errors instead of doubled jargon, and a failed project load now shows "Couldn't load projects" + Retry instead of a misleading "No projects yet." Plus a 100+-case test net over the previously-untested project paths.

- **v0.27.0–v0.31.1** — Projects + Workspace era (detail via `git log`): projects (named folder aliases with include/exclude scoping) and a merged **Workspace** home, glob matching that actually works (`**` spans folders, validated at save), AI Health (your own Claude coaches your plan, undoable apply, Rift-settings-only), a top-to-bottom hardening pass (145-finding multi-agent review), accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
