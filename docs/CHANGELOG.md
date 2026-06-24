# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.34.0 — Windows that stay in sync

> Open Rift as separate native windows — one per monitor — and your conversation list now stays in sync across all of them. Plus a security tightening for those extra windows and a quieter release pipeline.

- **Multi-window, properly synced.** You could already open a second Rift window (one per monitor), but each window kept its own copy of the conversation list — create, rename, or delete a chat in one window and the other's sidebar wouldn't notice until you reloaded it. Now every window is told the moment the conversation store changes and refreshes itself, so all your windows always show the same up-to-date list. Each window still keeps its own open tabs.
- **Secondary windows are as locked-down as the main one.** Extra windows were missing a few of the "don't shell-open this" file-type guards the main window has — so a script file (`.py`, `.sh`, `.lua`, …) could have been launched from a secondary window where the main window would've blocked it. The guard lists are now identical everywhere.
- **No more false-failed releases.** A release that built and published perfectly was being marked failed because of a hosted helper job that can't run while the build server is self-hosted. That job is gone, replaced by a check that actually confirms the new version landed — so a red mark now means something really went wrong.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.33.0** — Rift runs as your Claude Code: turn on **Use my full Claude Code config** and Rift inherits your global `~/.claude` setup (`CLAUDE.md`, `settings.json`, hooks, slash commands, skills, MCP servers) exactly like the `claude` terminal — `--setting-sources user,project,local`, off is a clean sandbox. No-folder chats keep their workspace-independent tools. Cleaner Workspace + Activity panel (floating sub-agents card, stats dashboard). Off-tab `ask_user`/permission prompts now respond. Release pipeline runs the full test suite before publishing.

- **v0.32.0** — A cleaner Workspace, and start from a folder you already have: a ground-up Workspace redesign (calmer, fits one screen, no scroll), adopt-an-existing-folder as a one-tap project from your recent folders, a verified dead-code/dependency cleanup sweep, and a full documentation refresh (16 confirmed drift errors fixed against source).

- **v0.31.2** — Project editor that tells you what's wrong: live inline glob validation (red outline + invalid-count, Save disabled until fixed), plain-English save/delete errors instead of doubled jargon, and a failed project load now shows "Couldn't load projects" + Retry instead of a misleading "No projects yet." Plus a 100+-case test net over the previously-untested project paths.

- **v0.27.0–v0.31.1** — Projects + Workspace era (detail via `git log`): projects (named folder aliases with include/exclude scoping) and a merged **Workspace** home, glob matching that actually works (`**` spans folders, validated at save), AI Health (your own Claude coaches your plan, undoable apply, Rift-settings-only), a top-to-bottom hardening pass (145-finding multi-agent review), accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
