# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.51.0 — A faster assistant, a redesigned model picker, and a working Tasks panel

> This release kills a painful "everything is slow" regression, rebuilds the model + effort picker to look and feel like the rest of Rift, and brings the Tasks/Plan panel back to life.

### The assistant is fast again
- **Fixed the "slowest AI in the world" bug.** A previous release quietly flipped the fresh-install permission default away from "just run it," so every file read, edit, and command silently parked waiting on an approval the UI never surfaced — turns could hang for many minutes. Fresh installs now default to running tools directly again (approval prompts are still one toggle away), and the permission round-trip is hardened with a sane timeout and an actionable message if it ever does wait.
- **Extended thinking is off by default.** Matching Claude Code, a quick "hello" replies in a second or two instead of pausing to think first. Turn it back on anytime from the model menu.

### Redesigned model + effort picker
- **One polished picker, in Rift's design language.** Each model is a clean row with its own identity icon, full description (no more cut-off text), context size, and a number hotkey. The current model lights up with a checkmark.
- **"More models" flyout.** Previous-generation models (e.g. Opus 4.7) tuck into an expandable "More models" submenu instead of cluttering the main list — and it opens automatically if one of them is your active model.
- **A real effort dial.** The effort control is now a stepped slider labeled with the actual levels — **Low · Medium · Medium+ · High · X-High** (plain, honest names instead of marketing labels) — with the active level called out, smooth animations, and clickable steps. Each model shows only the levels it actually supports (Sonnet stops at High; Haiku, which doesn't use effort, hides the dial entirely).
- **Consistent typography throughout** — every label, tag, and hotkey sits on one deliberate type scale so the whole menu reads as tidy and organized.

### Tasks / Plan panel works again
- **The Tasks panel is no longer dead.** A Claude CLI update renamed its task tools, but Rift's allow-list still referenced the old names — so the model would say "there's no task tool here" and give up. Rift now recognizes the new task tools, and a live plan card renders in the conversation, ticking off 0/4 → 4/4 as the model works.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.50.0** — The biggest release in a while: the Workspace page became a real bento **dashboard** (your activity inline + a "What's new in AI" feed + a richer Projects panel), the **AI Health** tab grew into a genuine diagnostic (plain-English health score + warm-aware root-cause latency), and a broad **cross-machine compatibility** pass (safer first-run default, corporate-TLS trust, domain/locked-down machines, smaller screens, honest errors) lets Rift run cleanly on other people's Windows PCs.

- **v0.36.2** — Fable 5 is back: Anthropic's limited-run Fable 5 model returned, so it's once more an option in the model picker with its full 1M-token context and effort range.

- **v0.36.1** — The Claude CLI update actually updates now: clicking "Update" on the CLI bar would spin and then snap right back to the same prompt without moving the version (a running background Claude process held the CLI's file locked on Windows, and the bar read the stale version). Now Rift shuts those processes down before updating and re-checks right after. Plus quieter internals — bounded background-agent history and a handful of small correctness fixes.

- **v0.36.0** — One place for everything that happens: a real **notification center** (a bell in the toolbar with history grouped by when, persisting across restarts), updates moved into an always-visible top bar instead of the corner, redesigned toasts, a calmer chat surface (collapsing long code blocks, stream spacing polish), and a fix for a burst of internal errors on startup.

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
