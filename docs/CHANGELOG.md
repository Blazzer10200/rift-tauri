# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased

- **Background tasks no longer leave you waiting on a reply that never comes.** When the assistant ran a slow command "in the background" and said it would report back when it finished, that report could never arrive — Rift runs each turn as a one-shot, so there's nothing left listening once the turn ends (and the background command actually gets killed seconds later). The assistant is now told to run slow work in the foreground instead, so the result lands in the same reply. As a backstop, if a turn still kicks off a background task, you'll get a one-time heads-up — *"Background task won't auto-report — send a message to ask how it went"* — instead of silence.

## v0.51.1 — Live sub-agent activity

> A focused hotfix that overhauls the sub-agent panel so you can actually see what Claude's helper agents are doing, and stops it from overlapping the chat.

### You can see what sub-agents are doing now
- **A live "now doing" line per agent.** When Claude delegates work to a helper agent, the panel shows what it's touching *right now* in plain language — "Reading config.json," "Searching for…," "Thinking…" — with a live cue, instead of a meaningless checkmark and a green bar.
- **A recent-steps trail.** A finished agent shows the actual actions it took ("Searched for X · Read Y · +2 earlier steps") so you can tell what it accomplished at a glance, without expanding anything.
- **The panel header reads as alive.** "Working · 2 agents active" with a pulse while running, settling to "2 finished" when done.

### No more overlap
- **The chat no longer slides under the panel.** When the sub-agent card is open it reserves space, so wide messages stop getting clipped behind it. It releases that space the moment you minimize the panel back to its pill, and steps aside on narrow windows.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.51.0** — A faster assistant + a redesigned picker: fixed the "everything is slow" regression (a flipped permission default was parking every tool on an approval the UI never showed), turned extended thinking off by default, rebuilt the model + effort picker in Rift's design language (identity icons, a "More models" flyout, an honest Low→X-High effort dial), and revived the Tasks/Plan panel after a CLI tool-rename broke its allow-list.

- **v0.50.0** — The biggest release in a while: the Workspace page became a real bento **dashboard** (your activity inline + a "What's new in AI" feed + a richer Projects panel), the **AI Health** tab grew into a genuine diagnostic (plain-English health score + warm-aware root-cause latency), and a broad **cross-machine compatibility** pass (safer first-run default, corporate-TLS trust, domain/locked-down machines, smaller screens, honest errors) lets Rift run cleanly on other people's Windows PCs.

- **v0.36.2** — Fable 5 is back: Anthropic's limited-run Fable 5 model returned, so it's once more an option in the model picker with its full 1M-token context and effort range.

- **v0.36.1** — The Claude CLI update actually updates now: clicking "Update" on the CLI bar would spin and then snap right back to the same prompt without moving the version (a running background Claude process held the CLI's file locked on Windows, and the bar read the stale version). Now Rift shuts those processes down before updating and re-checks right after. Plus quieter internals — bounded background-agent history and a handful of small correctness fixes.

- **v0.36.0** — One place for everything that happens: a real **notification center** (a bell in the toolbar with history grouped by when, persisting across restarts), updates moved into an always-visible top bar instead of the corner, redesigned toasts, a calmer chat surface (collapsing long code blocks, stream spacing polish), and a fix for a burst of internal errors on startup.

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
