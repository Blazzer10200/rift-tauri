# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.35.0 — Fewer silent failures

> A reliability pass on the quiet edges: when something can't go through, Rift now notices and tells you instead of hanging. No new surfaces — your turns, git operations, and updates just fail loudly when they fail at all.

- **A stuck turn now surfaces instead of hanging.** When Rift hands a message to the Claude process, the final "push it through" step could fail unnoticed — the message never arrived and the turn sat there forever with no reply and no error. Rift now checks that step and either retries on a fresh process or shows a clear error, so a dead pipe can't silently swallow a turn.
- **The auto-updater can't brick itself.** If something crashed at exactly the wrong moment mid-update, the updater's internal lock could get permanently "poisoned" — after which every future check, download, and apply failed the same way until you reinstalled. It now recovers from that state on its own.
- **Update checks read as "checking," not stale.** While a CLI-update check is in flight, the status line briefly showed the previous result. It now says "Checking for updates…" until the real answer lands.
- **Safer git-timeout cleanup.** When a git command runs past 30 seconds and Rift kills it, the kill is now scoped to git specifically — closing a tiny window where it could in theory have signalled an unrelated process that reused the same ID.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.34.0** — Windows that stay in sync: open Rift as separate native windows (one per monitor) and the conversation list stays synced across all of them (create/rename/delete in one refreshes the rest; each keeps its own tabs). Secondary windows got the main window's shell-open file-type guards, and the release pipeline stopped marking successful releases as failed.

- **v0.33.0** — Rift runs as your Claude Code: turn on **Use my full Claude Code config** and Rift inherits your global `~/.claude` setup (`CLAUDE.md`, `settings.json`, hooks, slash commands, skills, MCP servers) exactly like the `claude` terminal (`--setting-sources user,project,local`; off is a clean sandbox). No-folder chats keep workspace-independent tools. Cleaner Workspace + Activity panel. Off-tab `ask_user`/permission prompts now respond.

- **v0.32.0** — A cleaner Workspace, and start from a folder you already have: a ground-up Workspace redesign (calmer, one screen, no scroll), adopt-an-existing-folder as a one-tap project, a verified dead-code/dependency cleanup sweep, and a full documentation refresh.

- **v0.31.2** — Project editor that tells you what's wrong: live inline glob validation (red outline + invalid-count, Save disabled until fixed), plain-English save/delete errors instead of doubled jargon, and a failed project load now shows "Couldn't load projects" + Retry instead of a misleading "No projects yet." Plus a 100+-case test net over the previously-untested project paths.

- **v0.27.0–v0.31.1** — Projects + Workspace era (detail via `git log`): projects (named folder aliases with include/exclude scoping) and a merged **Workspace** home, glob matching that actually works (`**` spans folders, validated at save), AI Health (your own Claude coaches your plan, undoable apply, Rift-settings-only), a top-to-bottom hardening pass (145-finding multi-agent review), accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
