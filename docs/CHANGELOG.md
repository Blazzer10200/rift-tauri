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

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
