# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.36.1 — The Claude CLI update actually updates now

> Clicking "Update" on the Claude CLI bar would spin, then snap right back to the same notification as if nothing happened. Fixed — plus a round of quiet reliability work under the hood.

- **Fixed: the Claude CLI update that wouldn't stick.** Updating the Claude CLI from the top bar reported "Updating…" and then re-showed the same update prompt, never moving the version. Two things were fighting it: Rift keeps a Claude process warm for fast replies, and on Windows that process held the CLI's own file locked — so the updater silently couldn't replace it. And even when it did update, the bar kept reading the old version. Now Rift shuts those background Claude processes down before updating (so the new version can actually be written) and re-checks the version right after (so the bar clears the moment it's done).
- **Quieter under the hood.** A long session no longer lets its list of background-agent activity grow without bound, and a handful of small correctness fixes from an internal review pass landed (multi-select prompt answers with commas in them, a couple of teardown races on rapid reload).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.36.0** — One place for everything that happens: a real **notification center** (a bell in the toolbar with history grouped by when, persisting across restarts), updates moved into an always-visible top bar instead of the corner, redesigned toasts, a calmer chat surface (collapsing long code blocks, stream spacing polish), and a fix for a burst of internal errors on startup.

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
